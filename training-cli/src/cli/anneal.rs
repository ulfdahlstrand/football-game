use std::path::Path;

use training_engine::policy::{self, mutate_team_v6, TeamPolicyV6, V6Params};
use training_engine::session::{EpochSummary, SessionWriter};
use training_engine::trainer::{evaluate_team_policies_v6, EarlyStop};

use super::util::{iso_now, save_team_artifacts, v6_diff_slots};

#[derive(Clone, Copy)]
pub enum AnnealVariant { Quick, Short, Full }

pub const ANNEAL_STAGES_FULL: &[(usize, usize)] = &[
    (10000, 100),
    (2000, 1000),
    (500, 10000),
    (100, 1_000_000),
];

pub const ANNEAL_STAGES_SHORT: &[(usize, usize)] = &[
    (500, 500),
    (200, 5000),
    (50, 50000),
];

pub const ANNEAL_STAGES_QUICK: &[(usize, usize)] = &[
    (50, 500),
    (20, 5000),
    (5, 50000),
];

pub fn variant_stages(v: AnnealVariant) -> &'static [(usize, usize)] {
    match v {
        AnnealVariant::Quick => ANNEAL_STAGES_QUICK,
        AnnealVariant::Short => ANNEAL_STAGES_SHORT,
        AnnealVariant::Full  => ANNEAL_STAGES_FULL,
    }
}

pub fn variant_label(v: AnnealVariant) -> &'static str {
    match v {
        AnnealVariant::Quick => "QUICK",
        AnnealVariant::Short => "SHORT",
        AnnealVariant::Full  => "FULL",
    }
}

/// Slumpmässigt initierat V6-lag (3 omgångar mutate_v6 från default per slot).
pub fn random_v6_team(rng: &mut impl rand::Rng) -> TeamPolicyV6 {
    let mut team: [V6Params; 5] = [
        policy::v6_default_for_slot(0),
        policy::v6_default_for_slot(1),
        policy::v6_default_for_slot(2),
        policy::v6_default_for_slot(3),
        policy::v6_default_for_slot(4),
    ];
    for _ in 0..3 {
        for slot in 0..5 {
            team[slot] = policy::mutate_v6(&team[slot], rng, 1.5);
        }
    }
    team
}

/// Default-anrop: hela lagets mutationer, skriv initial baseline.
pub fn run_team_anneal(
    team_dir: &Path,
    team_name: &str,
    initial: TeamPolicyV6,
    stages: &[(usize, usize)],
) -> TeamPolicyV6 {
    run_team_anneal_with_prefix(team_dir, team_name, initial, stages, "", true, None)
}

/// Mutera bara en specifik slot (0..=4).
pub fn run_team_anneal_slot_only(
    team_dir: &Path,
    team_name: &str,
    initial: TeamPolicyV6,
    stages: &[(usize, usize)],
    slot: usize,
) -> TeamPolicyV6 {
    run_team_anneal_with_prefix(team_dir, team_name, initial, stages, "", true, Some(slot))
}

pub fn run_team_anneal_with_prefix(
    team_dir: &Path,
    team_name: &str,
    initial: TeamPolicyV6,
    stages: &[(usize, usize)],
    session_prefix: &str,
    write_initial_baseline: bool,
    slot_filter: Option<usize>,
) -> TeamPolicyV6 {
    training_engine::game::CLUSTER_START.store(true, std::sync::atomic::Ordering::Relaxed);

    std::fs::create_dir_all(team_dir).expect("create team dir");
    std::fs::create_dir_all(team_dir.join("sessions")).expect("create team sessions dir");

    let baseline_path = team_dir.join("baseline.json");
    if write_initial_baseline {
        let bootstrap = serde_json::json!({
            "name": team_name, "version": 1,
            "type": "team-policy-v6",
            "description": format!("{}: random-init V6 team for population training", team_name),
            "playerParams": initial,
        });
        super::util::write_json_pretty(&baseline_path, &bootstrap);
    }

    let mut champion: TeamPolicyV6 = initial;
    let team_start = std::time::Instant::now();

    for (stage_idx, &(stage_epochs, games_per_epoch)) in stages.iter().enumerate() {
        let stage_name = format!("{}anneal-stage-{}-{}ep-{}g", session_prefix, stage_idx + 1, stage_epochs, games_per_epoch);
        let writer = match SessionWriter::new(team_dir, &stage_name) {
            Ok(w) => w,
            Err(e) => { eprintln!("  ! could not create stage dir for {}: {}", team_name, e); continue; }
        };
        let stage_started = iso_now();
        let _ = writer.write_team_v6_initial_baseline(&champion, &stage_started);

        let stage_start = std::time::Instant::now();
        println!("  [{}] stage {}: {} epochs × {} games", team_name, stage_idx + 1, stage_epochs, games_per_epoch);

        let mut history: Vec<EpochSummary> = Vec::new();
        let mut champion_epoch: usize = 0;
        let mut scale_factor: f32 = 1.0;
        let mut rejection_streak: usize = 0;
        let advance_threshold = (stage_epochs as f64 * 0.10).max(50.0) as usize;

        for epoch in 1..=stage_epochs {
            let opponent = champion;
            let mut rng = rand::thread_rng();
            let candidate = if let Some(slot) = slot_filter {
                policy::mutate_slot_only(&champion, slot, &mut rng, scale_factor)
            } else {
                mutate_team_v6(&champion, &mut rng, scale_factor)
            };
            let _mutated_slots = v6_diff_slots(&champion, &candidate);

            let eval = evaluate_team_policies_v6(&opponent, &candidate, games_per_epoch);
            let accepted = eval.candidate_won;
            if accepted {
                champion = candidate;
                champion_epoch = epoch;
                rejection_streak = 0;
                scale_factor = (scale_factor * 1.5).min(1.0);
            } else {
                rejection_streak += 1;
                if rejection_streak % 20 == 0 {
                    scale_factor = (scale_factor * 0.75).max(0.1);
                }
            }

            let early_label = eval.early_stop.map(|s| match s {
                EarlyStop::Worse => "worse".to_string(),
                EarlyStop::Better => "better".to_string(),
                EarlyStop::Indecisive => "indecisive".to_string(),
            });
            let stop_str = eval.early_stop.map(|s| match s {
                EarlyStop::Worse => " [EARLY STOP: worse]",
                EarlyStop::Better => " [EARLY STOP: better]",
                EarlyStop::Indecisive => " [INDECISIVE: futile]",
            }).unwrap_or("");
            history.push(EpochSummary {
                epoch, accepted, champion_epoch,
                goal_diff: eval.goal_diff,
                baseline_avg_goals: eval.baseline_avg_goals,
                candidate_avg_goals: eval.candidate_avg_goals,
                elapsed_ms: eval.elapsed_ms, early_stop: early_label,
                z_score: eval.z_score, games_run: eval.games,
            });

            println!("    e{:04} {} diff={:+.1} z={:.2} g={}/{} streak={}{}",
                epoch, if accepted { "ACC" } else { "rej" },
                eval.goal_diff, eval.z_score, eval.games, games_per_epoch, rejection_streak, stop_str);

            if rejection_streak >= advance_threshold {
                println!("    [stage advance: {} rejection streak ≥ {}]", rejection_streak, advance_threshold);
                break;
            }
        }

        let stage_finished = iso_now();
        let actual_epochs = history.len();
        let _ = writer.write_team_v6_summary(
            &stage_started, &stage_finished, actual_epochs, games_per_epoch,
            champion_epoch, &champion, &history);
        let _ = writer.write_team_v6_best(champion_epoch, &champion, &stage_name);

        let stage_elapsed = stage_start.elapsed();
        println!("  [{}] stage {} done in {:.0}s ({} epochs, champion epoch {})",
            team_name, stage_idx + 1, stage_elapsed.as_secs_f64(),
            actual_epochs, champion_epoch);
    }

    // Skriv slutgiltig baseline (utan info.md / layout.svg — caller hanterar det)
    let final_doc = serde_json::json!({
        "name": team_name, "version": 2,
        "type": "team-policy-v6",
        "description": format!("{}: final champion after {}-stage adaptive anneal", team_name, stages.len()),
        "playerParams": champion,
        "trainedAt": iso_now(),
        "trainedFromCluster": true,
    });
    super::util::write_json_pretty(&baseline_path, &final_doc);

    let elapsed = team_start.elapsed();
    println!("  [{}] all stages done in {:.0}s", team_name, elapsed.as_secs_f64());

    champion
}

// ── Single-stage helpers (ersätter 3× duplicerade --single-stage* block) ─────

#[derive(Clone, Copy)]
pub enum SingleStageMode {
    Full,
    SlotOnly(usize),
    GkOnly,
}

impl SingleStageMode {
    fn folder_name(&self) -> String {
        match self {
            Self::Full         => "single-stage".into(),
            Self::SlotOnly(s)  => format!("single-stage-slot{}", s),
            Self::GkOnly       => "single-stage-gk".into(),
        }
    }

    fn slot_filter(&self) -> Option<usize> {
        match self {
            Self::Full         => None,
            Self::SlotOnly(s)  => Some(*s),
            Self::GkOnly       => Some(4),
        }
    }

    fn description(&self, team_name: &str, epochs: usize, games: usize) -> String {
        match self {
            Self::Full        => format!("{}: single-stage {}×{}", team_name, epochs, games),
            Self::SlotOnly(s) => {
                let slot_name = ["fwd","mid-top","mid-bot","def","gk"][*s];
                format!("{}: single-stage slot {} ({}) {}×{}", team_name, s, slot_name, epochs, games)
            },
            Self::GkOnly      => format!("{}: single-stage GK-only {}×{}", team_name, epochs, games),
        }
    }

    fn training_method(&self) -> String {
        match self {
            Self::Full         => "single-stage-anneal".into(),
            Self::SlotOnly(s)  => format!("single-stage-slot-{}", s),
            Self::GkOnly       => "single-stage-gk-only".into(),
        }
    }
}

/// Kör en enstegs anneal — full / slot-only / gk-only via `mode`.
/// Ersätter de tre `--single-stage*`-blocken som tidigare upprepades i main.rs.
pub fn run_single_stage(
    project_root: &Path,
    team_name: &str,
    mode: SingleStageMode,
    epochs: usize,
    games: usize,
) {
    let teams_dir = project_root.join("data").join("teams");
    let team_dir = teams_dir.join(team_name);
    let baseline_path = team_dir.join("baseline.json");

    let baseline_file = training_engine::session::read_team_baseline_v6(&baseline_path)
        .unwrap_or_else(|e| panic!("cannot load {}: {}", team_name, e));
    let start = baseline_file.player_params;

    let mode_label = match mode {
        SingleStageMode::Full        => "single-stage".to_string(),
        SingleStageMode::SlotOnly(s) => format!("single-stage SLOT-{}", s),
        SingleStageMode::GkOnly      => "single-stage GK-ONLY".to_string(),
    };
    println!("[{}] {}: {} epochs × {} games", team_name, mode_label, epochs, games);

    let stages: &[(usize, usize)] = &[(epochs, games)];
    let anneal_dir = team_dir.join("sessions").join(mode.folder_name());
    std::fs::create_dir_all(&anneal_dir).expect("create anneal dir");

    let champion = run_team_anneal_with_prefix(
        &anneal_dir, team_name, start, stages, "",
        true, mode.slot_filter(),
    );

    save_team_artifacts(
        &team_dir, team_name, &champion,
        &mode.description(team_name, epochs, games),
        &mode.training_method(),
        1,
    );
    println!("Updated baseline + info.md + layout.svg for {}", team_name);
}
