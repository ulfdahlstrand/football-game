mod constants;
mod game;
mod policy;
mod spatial;
mod brain;
mod ai;
mod physics;
mod team;
mod team_v6;
mod trainer;
mod session;
mod svg;

use std::path::Path;
use std::time::SystemTime;

use policy::{mutate_team_v6, mutate_gk_only, TeamPolicyV6, V6Params};
use session::{ensure_team_v6_genesis, read_team_baseline_v6, update_team_v6_baseline, EpochSummary, SessionWriter};
use trainer::{evaluate_team_policies_v6, EarlyStop};
use svg::write_tournament_svg;

fn numeric_session_sort(dirs: &mut Vec<String>) {
    dirs.sort_by(|a, b| {
        let key = |s: &str| -> (Option<u64>, String) {
            let trailing: String = s.chars().rev()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>().chars().rev().collect();
            let n = trailing.parse::<u64>().ok();
            let prefix = s[..s.len() - trailing.len()].to_string();
            (n, prefix)
        };
        let (na, pa) = key(a);
        let (nb, pb) = key(b);
        pa.cmp(&pb).then_with(|| match (na, nb) {
            (Some(x), Some(y)) => x.cmp(&y),
            _ => a.cmp(b),
        })
    });
}

fn list_sessions(sessions_dir: &Path) -> Vec<String> {
    let mut dirs: Vec<String> = std::fs::read_dir(sessions_dir)
        .ok()
        .into_iter()
        .flat_map(|rd| rd.filter_map(|entry| {
            let entry = entry.ok()?;
            if !entry.file_type().ok()?.is_dir() { return None; }
            entry.file_name().into_string().ok()
        }))
        .collect();
    numeric_session_sort(&mut dirs);
    dirs
}

/// Returns (total_training_matches, total_training_minutes)
fn compute_total_stats(sessions_dir: &Path) -> (u64, f64) {
    let mut total_matches: u64 = 0;
    let mut total_ms: u128 = 0;
    if let Ok(rd) = std::fs::read_dir(sessions_dir) {
        for entry in rd.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) { continue; }
            let summary_path = entry.path().join("summary.json");
            if !summary_path.exists() { continue; }
            if let Ok(text) = std::fs::read_to_string(&summary_path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                    let games_per_epoch = val["gamesPerEpoch"].as_u64().unwrap_or(0);
                    if let Some(history) = val["history"].as_array() {
                        for h in history {
                            let games = h["gamesRun"].as_u64().unwrap_or(games_per_epoch);
                            total_matches += games;
                        }
                    }
                    if let Some(ms) = val["totalTrainingElapsedMs"].as_u64() {
                        total_ms += ms as u128;
                    }
                }
            }
        }
    }
    (total_matches, total_ms as f64 / 60_000.0)
}

fn v6_diff_slots(a: &TeamPolicyV6, b: &TeamPolicyV6) -> Vec<usize> {
    (0..5).filter(|&i| {
        let pa = serde_json::to_value(a[i]).unwrap();
        let pb = serde_json::to_value(b[i]).unwrap();
        pa != pb
    }).collect()
}

fn run_v6_training(policies_dir: &Path, epochs: usize, games_per_epoch: usize, session_name: &str) {
    let baseline_path = policies_dir.join("baseline.json");

    if !baseline_path.exists() {
        eprintln!("v6 baseline not found at {}. Seed it first.", baseline_path.display());
        std::process::exit(1);
    }

    let baseline_file = match read_team_baseline_v6(&baseline_path) {
        Ok(b) => b,
        Err(e) => { eprintln!("Error reading v6 baseline: {}", e); std::process::exit(1); }
    };
    let initial_team: TeamPolicyV6 = baseline_file.player_params;
    let mut champion: TeamPolicyV6 = initial_team;
    let mut champion_epoch: usize = 0;
    let session_started = iso_now();

    ensure_team_v6_genesis(&baseline_path, &baseline_file);
    let training_start = std::time::Instant::now();

    let writer = match SessionWriter::new(policies_dir, session_name) {
        Ok(w) => w,
        Err(e) => { eprintln!("Error creating session dir: {}", e); std::process::exit(1); }
    };
    let _ = writer.write_team_v6_initial_baseline(&initial_team, &session_started);

    println!("v6 training '{}': {} epochs x {} games/epoch (5 slots, spatial-prefs cost minimisation)",
             session_name, epochs, games_per_epoch);
    println!("Using {} CPU threads via rayon", rayon::current_num_threads());

    let mut history: Vec<EpochSummary> = Vec::with_capacity(epochs);
    let mut scale_factor: f32 = 1.0;
    let mut rejection_streak: usize = 0;

    for epoch in 1..=epochs {
        let opponent_epoch = champion_epoch;
        let opponent = champion;
        let mut rng = rand::thread_rng();
        let candidate = mutate_team_v6(&champion, &mut rng, scale_factor);
        let mutated_slots = v6_diff_slots(&champion, &candidate);

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
                println!("  [scale reduced to {:.3} after {} consecutive rejections]", scale_factor, rejection_streak);
            }
        }

        let current_champion_epoch = if accepted { epoch } else { champion_epoch };
        let current_champion = if accepted { candidate } else { champion };
        let early_label = eval.early_stop.map(|s| match s {
            EarlyStop::Worse => "worse".to_string(), EarlyStop::Better => "better".to_string(), EarlyStop::Indecisive => "indecisive".to_string(),
        });
        let stop_str = eval.early_stop.map(|s| match s {
            EarlyStop::Worse => " [EARLY STOP: worse]", EarlyStop::Better => " [EARLY STOP: better]", EarlyStop::Indecisive => " [INDECISIVE: futile]",
        }).unwrap_or("");
        let slot_label: String = mutated_slots.iter()
            .map(|i| format!("{}#{}", policy::TEAM_SLOT_NAMES[*i], i))
            .collect::<Vec<_>>().join(",");

        println!("epoch-{:03} {} diff={:+.3} z={:.2} games={}/{} scale={:.3} mutated=[{}] champion={}{}",
            epoch, if accepted { "ACCEPTED" } else { "rejected" },
            eval.goal_diff, eval.z_score, eval.games, games_per_epoch,
            scale_factor, slot_label, current_champion_epoch, stop_str);

        let _ = writer.write_team_v6_epoch(
            epoch, opponent_epoch, &opponent, &candidate,
            accepted, current_champion_epoch, &current_champion,
            &mutated_slots, &eval, &iso_now(), games_per_epoch);
        history.push(EpochSummary {
            epoch, accepted, champion_epoch: current_champion_epoch,
            goal_diff: eval.goal_diff,
            baseline_avg_goals: eval.baseline_avg_goals,
            candidate_avg_goals: eval.candidate_avg_goals,
            elapsed_ms: eval.elapsed_ms, early_stop: early_label,
            z_score: eval.z_score, games_run: eval.games,
        });
    }

    let finished_at = iso_now();
    let _ = writer.write_team_v6_summary(&session_started, &finished_at, epochs, games_per_epoch,
        champion_epoch, &champion, &history);
    let _ = writer.write_team_v6_best(champion_epoch, &champion, session_name);
    crate::svg::write_training_svg(
        &writer.session_dir().join("training-progress.svg"),
        &history, champion_epoch);

    if champion_epoch > 0 {
        println!("\nEvaluating final champion against v6 session baseline ({} games)...", games_per_epoch);
        let final_eval = evaluate_team_policies_v6(&initial_team, &champion, games_per_epoch);
        println!("vs session baseline: champion={:.3} baseline={:.3} diff={:+.3} z={:.2}",
            final_eval.candidate_avg_goals, final_eval.baseline_avg_goals,
            final_eval.goal_diff, final_eval.z_score);
        if final_eval.candidate_won {
            match update_team_v6_baseline(&baseline_path, &baseline_file, &champion,
                session_name, champion_epoch, final_eval.goal_diff, &iso_now()) {
                Ok(_) => println!("v6 baseline.json updated → epoch {} diff={:+.3}",
                    champion_epoch, final_eval.goal_diff),
                Err(e) => eprintln!("Warning: could not update baseline: {}", e),
            }
        } else {
            println!("Champion did not beat session baseline — baseline.json unchanged.");
        }
    } else {
        println!("\nNo improvement found this session — baseline.json unchanged.");
    }

    let elapsed = training_start.elapsed();
    let total_secs = elapsed.as_secs();
    let h = total_secs / 3600; let m = (total_secs % 3600) / 60; let s = total_secs % 60;
    if h > 0 { println!("\nTotal training time: {}h {}m {}s", h, m, s); }
    else { println!("\nTotal training time: {}m {}s", m, s); }
    println!("Done. Champion epoch: {}", champion_epoch);
}

fn iso_now() -> String {
    let d = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let (y, mo, day, h, mi, s) = unix_to_datetime(d.as_secs());
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, mo, day, h, mi, s)
}

fn unix_to_datetime(mut ts: u64) -> (u64, u64, u64, u64, u64, u64) {
    let s = ts % 60; ts /= 60;
    let mi = ts % 60; ts /= 60;
    let h = ts % 24; ts /= 24;
    let mut days = ts;
    let mut year = 1970u64;
    loop {
        let dy = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) { 366 } else { 365 };
        if days < dy { break; }
        days -= dy;
        year += 1;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let months = [31u64, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u64;
    for &m in &months {
        if days < m { break; }
        days -= m;
        month += 1;
    }
    (year, month, days + 1, h, mi, s)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let project_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();

    if args.get(1).map(|s| s.as_str()) == Some("--v6") {
        let v6_dir = project_root.join("data").join("policies").join("v6");
        let epochs: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(100);
        let games_per_epoch: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1000);
        let session_name: String = args.get(4).cloned().unwrap_or_else(|| "session-1".to_string());
        run_v6_training(&v6_dir, epochs, games_per_epoch, &session_name);
        return;
    }
    if args.get(1).map(|s| s.as_str()) == Some("--v6-population") {
        let num_teams: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10);
        let variant = if args.iter().any(|s| s == "--full") { AnnealVariant::Full }
                      else if args.iter().any(|s| s == "--quick") { AnnealVariant::Quick }
                      else { AnnealVariant::Short };
        let skip: usize = args.iter().position(|s| s == "--skip")
            .and_then(|i| args.get(i + 1)).and_then(|s| s.parse().ok()).unwrap_or(0);
        run_v6_population(&project_root, num_teams, variant, skip);
        return;
    }
    if args.get(1).map(|s| s.as_str()) == Some("--v6-rough-team") {
        let team_name: String = args.get(2).cloned().unwrap_or_else(|| "forge-fc".to_string());
        let max_depth: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(3);
        let games: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1000);
        run_v6_rough_team(&project_root, &team_name,
            "Forge FC — deterministic ternary-ablation rough calibration",
            max_depth, games);
        return;
    }
    if args.get(1).map(|s| s.as_str()) == Some("--score-probe") {
        // Print actual scores for N games between two teams.
        // Usage: --score-probe <team_a> <team_b> [games]
        let team_a = args.get(2).cloned().unwrap_or_else(|| "granite-athletic".to_string());
        let team_b = args.get(3).cloned().unwrap_or_else(|| "nebula-rangers".to_string());
        let n: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(100);
        let teams_dir = project_root.join("data").join("teams");
        let load = |name: &str| -> TeamPolicyV6 {
            let p = teams_dir.join(name).join("baseline.json");
            read_team_baseline_v6(&p).unwrap_or_else(|e| panic!("cannot load {}: {}", name, e)).player_params
        };
        let pa = load(&team_a);
        let pb = load(&team_b);
        let mut rng = rand::thread_rng();
        let mut score_counts: std::collections::HashMap<(u32,u32), u32> = std::collections::HashMap::new();
        for i in 0..n {
            let seed: u64 = rand::Rng::gen(&mut rng);
            let swap = i % 2 == 1;
            let (g0, g1) = {
                let mut g = crate::game::Game::new();
                let mut teams: [Box<dyn crate::team::Team>; 2] = [
                    Box::new(crate::team_v6::V6Team::new(0, if swap { pb } else { pa })),
                    Box::new(crate::team_v6::V6Team::new(1, if swap { pa } else { pb })),
                ];
                while g.phase != crate::game::Phase::Fulltime { crate::physics::step_game(&mut g, &mut teams, &mut rng); }
                if swap { (g.score[1], g.score[0]) } else { (g.score[0], g.score[1]) }
            };
            *score_counts.entry((g0, g1)).or_insert(0) += 1;
        }
        println!("Score distribution: {} vs {} ({} games)", team_a, team_b, n);
        let mut counts: Vec<_> = score_counts.into_iter().collect();
        counts.sort_by(|a, b| b.1.cmp(&a.1));
        for ((a, b), c) in &counts {
            let result = if a > b { "W" } else if b > a { "L" } else { "D" };
            println!("  {}-{}  {}  x{}", a, b, result, c);
        }
        return;
    }
    if args.get(1).map(|s| s.as_str()) == Some("--v6-tournament") {
        let games_per_match: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10000);
        run_v6_tournament(&project_root, games_per_match);
        return;
    }
    if args.get(1).map(|s| s.as_str()) == Some("--param-sweep") {
        // Combined sweep: ternary ablation per param → QUICK anneal → points eval.
        // Usage: --param-sweep <team> [ablation_games] [eval_games]
        let team_name: String = args.get(2).cloned().unwrap_or_else(|| "tempest-united".to_string());
        let ablation_games: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(500);
        let eval_games: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1000);
        run_param_sweep(&project_root, &team_name, ablation_games, eval_games);
        return;
    }
    if args.get(1).map(|s| s.as_str()) == Some("--param-optimize") {
        // Block coordinate descent: sweep → combine top-N → lock → repeat (max-rounds).
        // Usage: --param-optimize <team> [ablation_games] [eval_games] [max_rounds] [max_better] [scope]
        // scope: "decision" (default) | "spatial" | "all"
        let team_name: String = args.get(2).cloned().unwrap_or_else(|| "tempest-united".to_string());
        let ablation_games: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(500);
        let eval_games: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1000);
        let max_rounds: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(3);
        let max_better: usize = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(3);
        let scope = args.get(7).map(|s| ParamScope::parse(s)).unwrap_or(ParamScope::Decision);
        run_param_optimize(&project_root, &team_name, ablation_games, eval_games, max_rounds, max_better, scope);
        return;
    }
    if args.get(1).map(|s| s.as_str()) == Some("--param-combine") {
        // Test all 2^N combinations of sweep-found improvements.
        // Usage: --param-combine <team> [eval_games] [param1,param2,...]
        // Default params: mark_distance,pass_chance_pressured,pass_chance_forward
        let team_name: String = args.get(2).cloned().unwrap_or_else(|| "tempest-united".to_string());
        let eval_games: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1000);
        let params_arg = args.get(4).cloned()
            .unwrap_or_else(|| "mark_distance,pass_chance_pressured,pass_chance_forward".to_string());
        let params: Vec<String> = params_arg.split(',').map(|s| s.trim().to_string()).collect();
        run_param_combine(&project_root, &team_name, &params, eval_games);
        return;
    }
    if args.get(1).map(|s| s.as_str()) == Some("--v6-team-svgs") {
        regenerate_all_team_svgs(&project_root);
        return;
    }
    if args.get(1).map(|s| s.as_str()) == Some("--single-stage-slot") {
        // Single-stage anneal that mutates ONLY the specified slot (0..=4).
        // Slots: 0=fwd, 1=mid-top, 2=mid-bottom, 3=def, 4=gk
        // Usage: --single-stage-slot <team> <slot> <epochs> <games>
        let team_name: String = args.get(2).cloned().unwrap_or_else(|| "granite-athletic".to_string());
        let slot: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
        let epochs: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(2000);
        let games: usize  = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(200);
        if slot > 4 { eprintln!("slot must be 0..=4"); std::process::exit(1); }
        let slot_name = ["fwd","mid-top","mid-bot","def","gk"][slot];
        let stages: &[(usize, usize)] = &[(epochs, games)];
        let teams_dir = project_root.join("data").join("teams");
        let team_dir  = teams_dir.join(&team_name);
        let baseline_path = team_dir.join("baseline.json");
        let baseline_file = read_team_baseline_v6(&baseline_path)
            .unwrap_or_else(|e| panic!("cannot load {}: {}", team_name, e));
        let start = baseline_file.player_params.clone();
        println!("[{}] single-stage SLOT-{}({}): {} epochs × {} games", team_name, slot, slot_name, epochs, games);
        let anneal_dir = team_dir.join("sessions").join(format!("single-stage-slot{}", slot));
        std::fs::create_dir_all(&anneal_dir).expect("create anneal dir");
        let champion = run_team_anneal_slot_only(&anneal_dir, &team_name, start, stages, slot);
        let team_desc_idx = TEAM_NAMES.iter().position(|n| *n == team_name.as_str());
        let team_desc = team_desc_idx.and_then(|i| TEAM_DESCRIPTIONS.get(i)).copied().unwrap_or(team_name.as_str());
        let final_doc = serde_json::json!({
            "name": team_name, "version": 1,
            "type": "team-policy-v6",
            "description": format!("{}: single-stage slot {} ({}) {}×{}", team_name, slot, slot_name, epochs, games),
            "playerParams": champion,
            "trainedAt": iso_now(),
            "trainingMethod": format!("single-stage-slot-{}", slot),
        });
        let _ = std::fs::write(&baseline_path,
            format!("{}\n", serde_json::to_string_pretty(&final_doc).unwrap()));
        write_team_info_md(&team_dir, &team_name, team_desc, &champion);
        write_team_layout_svg(&team_dir.join("layout.svg"), &team_name, team_desc, &champion);
        println!("Updated baseline + info.md + layout.svg for {}", team_name);
        return;
    }
    if args.get(1).map(|s| s.as_str()) == Some("--single-stage-gk") {
        // Single-stage anneal that mutates ONLY the GK slot (slot 4).
        // Usage: --single-stage-gk <team> <epochs> <games_per_epoch>
        let team_name: String = args.get(2).cloned().unwrap_or_else(|| "granite-athletic".to_string());
        let epochs: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(2000);
        let games: usize  = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(200);
        let stages: &[(usize, usize)] = &[(epochs, games)];
        let teams_dir = project_root.join("data").join("teams");
        let team_dir  = teams_dir.join(&team_name);
        let baseline_path = team_dir.join("baseline.json");
        let baseline_file = read_team_baseline_v6(&baseline_path)
            .unwrap_or_else(|e| panic!("cannot load {}: {}", team_name, e));
        let start = baseline_file.player_params.clone();
        println!("[{}] single-stage GK-ONLY: {} epochs × {} games", team_name, epochs, games);
        let anneal_dir = team_dir.join("sessions").join("single-stage-gk");
        std::fs::create_dir_all(&anneal_dir).expect("create anneal dir");
        let champion = run_team_anneal_slot_only(&anneal_dir, &team_name, start, stages, 4);
        let team_desc_idx = TEAM_NAMES.iter().position(|n| *n == team_name.as_str());
        let team_desc = team_desc_idx.and_then(|i| TEAM_DESCRIPTIONS.get(i)).copied().unwrap_or(team_name.as_str());
        let final_doc = serde_json::json!({
            "name": team_name, "version": 1,
            "type": "team-policy-v6",
            "description": format!("{}: single-stage GK-only {}×{}", team_name, epochs, games),
            "playerParams": champion,
            "trainedAt": iso_now(),
            "trainingMethod": "single-stage-gk-only",
        });
        let _ = std::fs::write(&baseline_path,
            format!("{}\n", serde_json::to_string_pretty(&final_doc).unwrap()));
        write_team_info_md(&team_dir, &team_name, team_desc, &champion);
        write_team_layout_svg(&team_dir.join("layout.svg"), &team_name, team_desc, &champion);
        println!("Updated baseline + info.md + layout.svg for {}", team_name);
        return;
    }
    if args.get(1).map(|s| s.as_str()) == Some("--single-stage") {
        // Single-stage anneal with custom epochs and games_per_epoch.
        // Usage: --single-stage <team> <epochs> <games_per_epoch>
        let team_name: String = args.get(2).cloned().unwrap_or_else(|| "granite-athletic".to_string());
        let epochs: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(100);
        let games: usize  = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(500);
        let stages: &[(usize, usize)] = &[(epochs, games)];
        let teams_dir = project_root.join("data").join("teams");
        let team_dir  = teams_dir.join(&team_name);
        let baseline_path = team_dir.join("baseline.json");
        let baseline_file = read_team_baseline_v6(&baseline_path)
            .unwrap_or_else(|e| panic!("cannot load {}: {}", team_name, e));
        let start = baseline_file.player_params.clone();
        println!("[{}] single-stage: {} epochs × {} games", team_name, epochs, games);
        let anneal_dir = team_dir.join("sessions").join("single-stage");
        std::fs::create_dir_all(&anneal_dir).expect("create anneal dir");
        let champion = run_team_anneal(&anneal_dir, &team_name, start, stages);
        let team_desc_idx = TEAM_NAMES.iter().position(|n| *n == team_name.as_str());
        let team_desc = team_desc_idx.and_then(|i| TEAM_DESCRIPTIONS.get(i)).copied().unwrap_or(team_name.as_str());
        let final_doc = serde_json::json!({
            "name": team_name, "version": 1,
            "type": "team-policy-v6",
            "description": format!("{}: single-stage {}×{}", team_name, epochs, games),
            "playerParams": champion,
            "trainedAt": iso_now(),
            "trainingMethod": "single-stage-anneal",
        });
        let _ = std::fs::write(&baseline_path,
            format!("{}\n", serde_json::to_string_pretty(&final_doc).unwrap()));
        write_team_info_md(&team_dir, &team_name, team_desc, &champion);
        write_team_layout_svg(&team_dir.join("layout.svg"), &team_name, team_desc, &champion);
        println!("Updated baseline + info.md + layout.svg for {}", team_name);
        return;
    }
    if args.get(1).map(|s| s.as_str()) == Some("--v6-team-train") {
        let team_name: String = args.get(2).cloned().unwrap_or_else(|| "granite-athletic".to_string());
        let variant = if args.iter().any(|s| s == "--full") { AnnealVariant::Full }
                      else if args.iter().any(|s| s == "--quick") { AnnealVariant::Quick }
                      else { AnnealVariant::Short };
        run_v6_team_train(&project_root, &team_name, variant);
        return;
    }
    // --gk-train [epochs] [games_per_epoch]
    // Train only the GK slot (slot 4) for all teams in data/teams/.
    // Outfield params are frozen. GK params (diveChance, diveCommitDist,
    // riskClearance, distributionZone, passTargetDist) are mutated and evaluated.
    if args.get(1).map(|s| s.as_str()) == Some("--gk-train") {
        let epochs: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(100);
        let games_per_epoch: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1_000_000);
        run_gk_train_all(&project_root, epochs, games_per_epoch);
        return;
    }
    if args.get(1).map(|s| s.as_str()) == Some("--v6-test-continue") {
        // Continue clustered-start training from current v6 baseline (does not
        // reset). Useful for staged annealing: 1k → 10k → 100k.
        crate::game::CLUSTER_START.store(true, std::sync::atomic::Ordering::Relaxed);
        let v6_dir = project_root.join("data").join("policies").join("v6");
        let epochs: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(50);
        let games_per_epoch: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(10000);
        let session_name: String = args.get(4).cloned().unwrap_or_else(|| "test-cont".to_string());
        println!("[v6-test-continue] CLUSTER_START enabled — continuing from current v6 baseline");
        run_v6_training(&v6_dir, epochs, games_per_epoch, &session_name);
        return;
    }
    if args.get(1).map(|s| s.as_str()) == Some("--v6-test") {
        // Clean-slate positioning test: all field players start at centre of
        // own half. Tests whether spatial prefs naturally crystallise positions.
        crate::game::CLUSTER_START.store(true, std::sync::atomic::Ordering::Relaxed);
        let v6_dir = project_root.join("data").join("policies").join("v6");
        let epochs: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(50);
        let games_per_epoch: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1000);

        // Reset the v6 baseline to fresh per-slot defaults so the test starts
        // from a known clean point. Old baseline (if any) is preserved as
        // baseline-prev.json for reference.
        std::fs::create_dir_all(&v6_dir).expect("create v6 dir");
        let baseline_path = v6_dir.join("baseline.json");
        if baseline_path.exists() {
            let _ = std::fs::rename(&baseline_path, v6_dir.join("baseline-prev.json"));
        }
        let fresh: [V6Params; 5] = [0,1,2,3,4].map(policy::v6_default_for_slot);
        let bootstrap = serde_json::json!({
            "name": "v6-baseline", "version": 1,
            "type": "team-policy-v6",
            "description": "Fresh v6 defaults seeded by --v6-test for clustered positioning experiment.",
            "playerParams": fresh,
        });
        let _ = std::fs::write(&baseline_path,
            format!("{}\n", serde_json::to_string_pretty(&bootstrap).unwrap()));
        // Remove any prior 'test' session folder for a fully clean run.
        let _ = std::fs::remove_dir_all(v6_dir.join("sessions").join("test"));
        let _ = std::fs::remove_file(v6_dir.join("baseline-genesis.json"));

        println!("[v6-test] CLUSTER_START enabled — all field players begin at FW*0.25 / FW*0.75, y=H2");
        println!("[v6-test] v6 baseline reset to per-slot defaults; running clean session 'test'");
        run_v6_training(&v6_dir, epochs, games_per_epoch, "test");
        return;
    }
    eprintln!("Unknown command. Use --v6-team-train, --single-stage, --v6-tournament, --v6-population, etc.");
    std::process::exit(1);
}

// ════════════════════════════════════════════════════════════════════════════
// V6 Population training + Round-robin tournament
// ════════════════════════════════════════════════════════════════════════════

const TEAM_NAMES: &[&str] = &[
    "aurora-fc",
    "granite-athletic",
    "phoenix-rovers",
    "tempest-united",
    "mirage-sc",
    "eclipse-town",
    "catalyst-city",
    "vortex-galaxy",
    "glacier-fc",
    "nebula-rangers",
];

const TEAM_DESCRIPTIONS: &[&str] = &[
    "Aurora FC — graceful and fluid",
    "Granite Athletic — solid and robust",
    "Phoenix Rovers — energetic, comeback-prone",
    "Tempest United — stormy, chaotic press",
    "Mirage SC — deceptive, unpredictable",
    "Eclipse Town — dark horse, counter-attack",
    "Catalyst City — fast transitions",
    "Vortex Galaxy — possession circulation",
    "Glacier FC — slow but inevitable",
    "Nebula Rangers — diffuse, exploratory",
];

/// Adaptive anneal stages: (epochs, games_per_epoch).
/// Stage advances early if rejection_streak >= 10% of stage epochs.
// FULL: 4-stage anneal for serious overnight runs.
// Each stage long enough that 10%-streak threshold lets it stabilise before advancing.
// Per-team budget: up to ~110M games at full cap; with early-stops typically 5-15M.
const ANNEAL_STAGES_FULL: &[(usize, usize)] = &[
    (10000, 100),
    (2000, 1000),
    (500, 10000),
    (100, 1_000_000),
];

// SHORT: 3-stage anneal for daily population runs.
// Per-team budget: up to ~3M games at full cap; with early-stops typically 1-2M.
// Estimated time per team: 30-60 min on 14 cores.
const ANNEAL_STAGES_SHORT: &[(usize, usize)] = &[
    (500, 500),
    (200, 5000),
    (50, 50000),
];

// QUICK: 3-stage anneal for fast smoke tests / quick-iteration.
// Per-team budget: ~375k games cap. Estimated time per team: 3-7 min on 14 cores.
const ANNEAL_STAGES_QUICK: &[(usize, usize)] = &[
    (50, 500),
    (20, 5000),
    (5, 50000),
];

/// Initialize a team with randomly perturbed per-slot defaults, so each team
/// in the population starts at a different point in the prefs landscape.
fn random_v6_team(rng: &mut impl rand::Rng) -> TeamPolicyV6 {
    let mut team: [V6Params; 5] = [
        policy::v6_default_for_slot(0),
        policy::v6_default_for_slot(1),
        policy::v6_default_for_slot(2),
        policy::v6_default_for_slot(3),
        policy::v6_default_for_slot(4),
    ];
    // Apply 3 rounds of mutate_v6 with high scale to spread initial points.
    for _ in 0..3 {
        for slot in 0..5 {
            team[slot] = policy::mutate_v6(&team[slot], rng, 1.5);
        }
    }
    team
}

/// Adaptive-anneal training: cycles through stages with 10% rejection-streak
/// early advance. Returns the final champion + per-stage history.
fn run_team_anneal(
    team_dir: &Path,
    team_name: &str,
    initial: TeamPolicyV6,
    stages: &[(usize, usize)],
) -> TeamPolicyV6 {
    run_team_anneal_with_prefix(team_dir, team_name, initial, stages, "", true, None)
}

fn run_team_anneal_slot_only(
    team_dir: &Path,
    team_name: &str,
    initial: TeamPolicyV6,
    stages: &[(usize, usize)],
    slot: usize,
) -> TeamPolicyV6 {
    run_team_anneal_with_prefix(team_dir, team_name, initial, stages, "", true, Some(slot))
}

fn run_team_anneal_with_prefix(
    team_dir: &Path,
    team_name: &str,
    initial: TeamPolicyV6,
    stages: &[(usize, usize)],
    session_prefix: &str,
    write_initial_baseline: bool,
    slot_filter: Option<usize>,
) -> TeamPolicyV6 {
    crate::game::CLUSTER_START.store(true, std::sync::atomic::Ordering::Relaxed);

    std::fs::create_dir_all(team_dir).expect("create team dir");
    std::fs::create_dir_all(team_dir.join("sessions")).expect("create team sessions dir");

    let baseline_path = team_dir.join("baseline.json");
    if write_initial_baseline {
        // Write initial baseline (only on first run)
        let bootstrap = serde_json::json!({
            "name": team_name, "version": 1,
            "type": "team-policy-v6",
            "description": format!("{}: random-init V6 team for population training", team_name),
            "playerParams": initial,
        });
        let _ = std::fs::write(&baseline_path,
            format!("{}\n", serde_json::to_string_pretty(&bootstrap).unwrap()));
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
            let mutated_slots = v6_diff_slots(&champion, &candidate);

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
                EarlyStop::Worse => "worse".to_string(), EarlyStop::Better => "better".to_string(), EarlyStop::Indecisive => "indecisive".to_string(),
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

            // Stop stage early if rejection streak exceeds threshold
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

    // Save final baseline
    let final_doc = serde_json::json!({
        "name": team_name, "version": 2,
        "type": "team-policy-v6",
        "description": format!("{}: final champion after {}-stage adaptive anneal", team_name, stages.len()),
        "playerParams": champion,
        "trainedAt": iso_now(),
        "trainedFromCluster": true,
    });
    let _ = std::fs::write(&baseline_path,
        format!("{}\n", serde_json::to_string_pretty(&final_doc).unwrap()));

    let elapsed = team_start.elapsed();
    println!("  [{}] all stages done in {:.0}s", team_name, elapsed.as_secs_f64());

    champion
}

fn write_team_info_md(team_dir: &Path, team_name: &str, description: &str, params: &TeamPolicyV6) {
    let mut md = String::new();
    md.push_str(&format!("# {}\n\n", team_name));
    md.push_str(&format!("> {}\n\n", description));
    md.push_str("Trained from clustered start (all field players at centre of own half) via\n");
    md.push_str("adaptive multi-stage anneal. Spatial preferences emerged organically from\n");
    md.push_str("mutation + selection — no positional logic was hand-coded.\n\n");
    md.push_str("## Spatial preferences (min / preferred / max)\n\n");
    md.push_str("| Slot | own_goal | side | ball | teammate | opponent |\n");
    md.push_str("|------|----------|------|------|----------|----------|\n");
    let slot_names = ["FWD", "MID-T", "MID-B", "DEF", "GK"];
    for i in 0..5 {
        let s = &params[i].spatial;
        md.push_str(&format!(
            "| {} | {:.0}/{:.0}/{:.0} | {:.0}/{:.0}/{:.0} | {:.0}/{:.0}/{:.0} | {:.0}/{:.0}/{:.0} | {:.0}/{:.0}/{:.0} |\n",
            slot_names[i],
            s.own_goal.min, s.own_goal.preferred, s.own_goal.max,
            s.side.min, s.side.preferred, s.side.max,
            s.ball.min, s.ball.preferred, s.ball.max,
            s.teammate.min, s.teammate.preferred, s.teammate.max,
            s.opponent.min, s.opponent.preferred, s.opponent.max,
        ));
    }
    md.push_str("\n## Decision parameters\n\n");
    md.push_str("| Slot | tackle | shoot_thr | aggr | risk | passDirOff | passDirDef |\n");
    md.push_str("|------|--------|-----------|------|------|------------|------------|\n");
    for i in 0..5 {
        let d = &params[i].decisions;
        md.push_str(&format!("| {} | {:.3} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} |\n",
            slot_names[i], d.tackle_chance, d.shoot_progress_threshold,
            d.aggression, d.risk_appetite, d.pass_dir_offensive, d.pass_dir_defensive));
    }
    md.push_str("\n## Inferred strategy\n\n");
    md.push_str("_(filled in after tournament analysis — see matrix in `data/matrices/`)_\n\n");
    md.push_str("## Tournament\n\n_(filled in after `--v6-tournament` run)_\n");
    let _ = std::fs::write(team_dir.join("info.md"), md);
}

#[derive(Clone, Copy)]
enum AnnealVariant { Quick, Short, Full }

fn run_v6_population(project_root: &Path, num_teams: usize, variant: AnnealVariant, skip: usize) {
    let teams_dir = project_root.join("data").join("teams");
    std::fs::create_dir_all(&teams_dir).expect("create teams dir");
    let stages: &[(usize, usize)] = match variant {
        AnnealVariant::Quick => ANNEAL_STAGES_QUICK,
        AnnealVariant::Short => ANNEAL_STAGES_SHORT,
        AnnealVariant::Full  => ANNEAL_STAGES_FULL,
    };
    let variant_label = match variant {
        AnnealVariant::Quick => "QUICK",
        AnnealVariant::Short => "SHORT",
        AnnealVariant::Full  => "FULL",
    };
    let start_idx = skip.min(TEAM_NAMES.len());
    let end_idx = (start_idx + num_teams).min(TEAM_NAMES.len());
    let n = end_idx - start_idx;

    println!("=== V6 POPULATION TRAINING ===");
    println!("Teams: {} (indices {}..{})", n, start_idx, end_idx);
    println!("Stages: {:?} ({})", stages, variant_label);
    println!("Per-team folder: data/teams/{{name}}/");
    println!();

    let pop_start = std::time::Instant::now();
    for (local_idx, i) in (start_idx..end_idx).enumerate() {
        let team_name = TEAM_NAMES[i];
        let team_desc = TEAM_DESCRIPTIONS[i];
        let team_dir = teams_dir.join(team_name);

        // Wipe prior team data so each population run is clean
        let _ = std::fs::remove_dir_all(&team_dir);

        let mut rng = rand::thread_rng();
        let initial = random_v6_team(&mut rng);

        println!("\n──── [{}/{}] {} ────", local_idx + 1, n, team_desc);
        let final_team = run_team_anneal(&team_dir, team_name, initial, stages);
        write_team_info_md(&team_dir, team_name, team_desc, &final_team);
        write_team_layout_svg(&team_dir.join("layout.svg"), team_name, team_desc, &final_team);
    }

    let total_elapsed = pop_start.elapsed();
    let h = total_elapsed.as_secs() / 3600;
    let m = (total_elapsed.as_secs() % 3600) / 60;
    println!("\n=== POPULATION TRAINING COMPLETE in {}h {}m ===", h, m);
    println!("Run --v6-tournament to evaluate teams against each other.");
}

fn run_v6_tournament(project_root: &Path, games_per_match: usize) {
    let teams_dir = project_root.join("data").join("teams");
    let matrices_dir = project_root.join("data").join("matrices");
    std::fs::create_dir_all(&matrices_dir).expect("create matrices dir");

    // Load all team baselines
    let mut teams: Vec<(String, String, TeamPolicyV6)> = vec![];
    if let Ok(entries) = std::fs::read_dir(&teams_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() { continue; }
            let baseline = path.join("baseline.json");
            if !baseline.exists() { continue; }
            let team_name = path.file_name().unwrap().to_string_lossy().into_owned();
            let baseline_file = match read_team_baseline_v6(&baseline) {
                Ok(b) => b,
                Err(e) => { eprintln!("  ! cannot read {}: {}", team_name, e); continue; }
            };
            let display = baseline_file.name.clone().unwrap_or_else(|| team_name.clone());
            teams.push((team_name, display, baseline_file.player_params));
        }
    }
    teams.sort_by(|a, b| a.0.cmp(&b.0));
    let n = teams.len();
    if n < 2 {
        eprintln!("Need at least 2 teams in data/teams/. Found {}.", n);
        std::process::exit(1);
    }

    println!("=== V6 ROUND-ROBIN TOURNAMENT ===");
    println!("Teams: {}", n);
    println!("Games per matchup: {}", games_per_match);
    println!();

    // Disable cluster start for tournament — teams should play with their
    // emergent positions from real spawn locations.
    crate::game::CLUSTER_START.store(false, std::sync::atomic::Ordering::Relaxed);

    let total_pairs = n * (n - 1) / 2;
    let mut pair_idx = 0;
    let mut matrix: Vec<Vec<f64>>  = vec![vec![0.0; n]; n]; // goal diff (row's perspective)
    let mut z_matrix: Vec<Vec<f64>> = vec![vec![0.0; n]; n];
    // Points: [i][j] = points row-team earned in the match vs column-team
    let mut pts: Vec<Vec<f64>> = vec![vec![0.0; n]; n];
    // Win/draw/loss counts per team
    let mut team_wins:   Vec<u64> = vec![0; n];
    let mut team_draws:  Vec<u64> = vec![0; n];
    let mut team_losses: Vec<u64> = vec![0; n];

    let tournament_start = std::time::Instant::now();
    for i in 0..n {
        for j in (i + 1)..n {
            pair_idx += 1;
            let eval = evaluate_team_policies_v6(&teams[i].2, &teams[j].2, games_per_match);
            // baseline=i, candidate=j → eval.wins = j wins, eval.losses = i wins
            matrix[i][j] = -eval.goal_diff;
            matrix[j][i] =  eval.goal_diff;
            z_matrix[i][j] = -eval.z_score;
            z_matrix[j][i] =  eval.z_score;

            let i_wins  = eval.losses; // i won when candidate(j) lost
            let j_wins  = eval.wins;
            let draws   = eval.draws;
            let i_pts = i_wins as f64 * 3.0 + draws as f64;
            let j_pts = j_wins as f64 * 3.0 + draws as f64;
            pts[i][j] = i_pts;
            pts[j][i] = j_pts;
            team_wins[i]   += i_wins;  team_wins[j]   += j_wins;
            team_draws[i]  += draws;   team_draws[j]  += draws;
            team_losses[i] += j_wins;  team_losses[j] += i_wins;

            println!("  [{}/{}] {:20} vs {:20}: pts={:.0}-{:.0} (W{}D{}L{}) diff={:+.0}",
                pair_idx, total_pairs, teams[i].0, teams[j].0,
                i_pts, j_pts, i_wins, draws, j_wins, eval.goal_diff);
        }
    }

    // Rankings: primary = total points, secondary = goal-diff
    let mut totals: Vec<(usize, f64, f64, f64)> = (0..n).map(|i| {
        let total_pts:  f64 = pts[i].iter().sum();
        let total_diff: f64 = matrix[i].iter().sum();
        let total_z:    f64 = z_matrix[i].iter().sum();
        (i, total_pts, total_diff, total_z)
    }).collect();
    totals.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap()
        .then(b.2.partial_cmp(&a.2).unwrap()));
    // Rebuild totals as (idx, diff, z) for SVG compat
    let totals_compat: Vec<(usize, f64, f64)> = totals.iter()
        .map(|(i, _, gd, z)| (*i, *gd, *z)).collect();

    println!("\n=== STANDINGS ===");
    println!("{:>4}  {:20}  {:>6}  {:>4}  {:>4}  {:>4}  {:>10}",
        "rank", "team", "pts", "W", "D", "L", "goal-diff");
    for (rank, (i, total_pts, total_diff, _)) in totals.iter().enumerate() {
        println!("{:>4}  {:20}  {:>6.0}  {:>4}  {:>4}  {:>4}  {:>+10.0}",
            rank + 1, teams[*i].0, total_pts,
            team_wins[*i], team_draws[*i], team_losses[*i], total_diff);
    }

    // Write matrix folder
    let timestamp = iso_now().replace(':', "").replace('-', "").chars().take(15).collect::<String>();
    let out_dir = matrices_dir.join(format!("v6-rr-{}", timestamp));
    std::fs::create_dir_all(&out_dir).expect("create matrix dir");

    let team_names: Vec<String> = teams.iter().map(|(n, _, _)| n.clone()).collect();
    let matrix_doc = serde_json::json!({
        "type": "v6-round-robin",
        "createdAt": iso_now(),
        "gamesPerMatch": games_per_match,
        "teams": team_names,
        "goalDiffMatrix": matrix,
        "zScoreMatrix": z_matrix,
        "pointsMatrix": pts,
        "rankings": totals.iter().enumerate().map(|(rank, (i, pts, gd, z))| serde_json::json!({
            "rank": rank + 1, "team": teams[*i].0,
            "points": pts, "goalDiff": gd, "zSum": z,
            "wins": team_wins[*i], "draws": team_draws[*i], "losses": team_losses[*i],
        })).collect::<Vec<_>>(),
    });
    let _ = std::fs::write(out_dir.join("matrix.json"),
        format!("{}\n", serde_json::to_string_pretty(&matrix_doc).unwrap()));

    let team_name_refs: Vec<&str> = teams.iter().map(|(n, _, _)| n.as_str()).collect();
    write_tournament_svg(
        &out_dir.join("matrix.svg"),
        &team_name_refs,
        &matrix,
        &z_matrix,
        &totals_compat,
        games_per_match,
    );

    let mut md = String::new();
    md.push_str(&format!("# V6 Round-robin tournament — {}\n\n", iso_now()));
    md.push_str(&format!("Games per matchup: {}\n\n", games_per_match));
    md.push_str("## Standings\n\n");
    md.push_str("| Rank | Team | Pts | W | D | L | Goal diff |\n|------|------|-----|---|---|---|-----------|\n");
    for (rank, (i, total_pts, total_diff, _)) in totals.iter().enumerate() {
        md.push_str(&format!("| {} | {} | {:.0} | {} | {} | {} | {:+.0} |\n",
            rank + 1, teams[*i].0, total_pts,
            team_wins[*i], team_draws[*i], team_losses[*i], total_diff));
    }
    md.push_str("\n## Points matrix (row earned vs column)\n\n|     |");
    for (name, _, _) in &teams { md.push_str(&format!(" {} |", name)); }
    md.push('\n');
    md.push_str("|-----|");
    for _ in &teams { md.push_str("----|"); }
    md.push('\n');
    for i in 0..n {
        md.push_str(&format!("| **{}** |", teams[i].0));
        for j in 0..n {
            if i == j { md.push_str(" — |"); }
            else { md.push_str(&format!(" {:.0} |", pts[i][j])); }
        }
        md.push('\n');
    }
    let _ = std::fs::write(out_dir.join("summary.md"), md);

    // Update info.md
    for (rank, (i, total_pts, total_diff, _)) in totals.iter().enumerate() {
        let team_dir = teams_dir.join(&teams[*i].0);
        let info_path = team_dir.join("info.md");
        if let Ok(existing) = std::fs::read_to_string(&info_path) {
            let placeholder = "_(filled in after `--v6-tournament` run)_";
            let replacement = format!(
                "**Rank**: {} / {}\n\n**Points**: {:.0} (W{} D{} L{})\n\n**Goal-diff**: {:+.0}\n\n**Best vs**: {}\n\n**Worst vs**: {}",
                rank + 1, n, total_pts, team_wins[*i], team_draws[*i], team_losses[*i], total_diff,
                { let (best_j, _) = pts[*i].iter().enumerate().filter(|(j,_)| j!=i).max_by(|a,b| a.1.partial_cmp(b.1).unwrap()).unwrap(); teams[best_j].0.clone() },
                { let (worst_j, _) = pts[*i].iter().enumerate().filter(|(j,_)| j!=i).min_by(|a,b| a.1.partial_cmp(b.1).unwrap()).unwrap(); teams[worst_j].0.clone() },
            );
            let _ = std::fs::write(&info_path, existing.replace(placeholder, &replacement));
        }
    }

    let elapsed = tournament_start.elapsed();
    println!("\n=== TOURNAMENT COMPLETE in {:.0}s ===", elapsed.as_secs_f64());
    println!("Matrix → {}", out_dir.display());
}

// ─── helpers for param sweep ───────────────────────────────────────────────

/// Set one named decision param to `value` across all 5 slots of a team policy.
fn set_decision_param_all_slots(team: &mut TeamPolicyV6, param: &str, value: f32) {
    for slot in team.iter_mut() {
        match param {
            "pass_chance_pressured"      => slot.decisions.pass_chance_pressured      = value,
            "pass_chance_wing"           => slot.decisions.pass_chance_wing           = value,
            "pass_chance_forward"        => slot.decisions.pass_chance_forward        = value,
            "pass_chance_default"        => slot.decisions.pass_chance_default        = value,
            "shoot_progress_threshold"   => slot.decisions.shoot_progress_threshold   = value,
            "tackle_chance"              => slot.decisions.tackle_chance              = value,
            "forward_pass_min_gain"      => slot.decisions.forward_pass_min_gain      = value,
            "mark_distance"              => slot.decisions.mark_distance              = value,
            "aggression"                 => slot.decisions.aggression                 = value,
            "risk_appetite"              => slot.decisions.risk_appetite              = value,
            "pass_dir_offensive"         => slot.decisions.pass_dir_offensive         = value,
            "pass_dir_defensive"         => slot.decisions.pass_dir_defensive         = value,
            "pass_dir_neutral"           => slot.decisions.pass_dir_neutral           = value,
            _ => eprintln!("  ! unknown decision param: {}", param),
        }
    }
}

// ─── shared param specs (used by sweep + optimize) ────────────────────────

const ALL_DECISION_PARAMS: &[(&str, f32, f32)] = &[
    ("pass_chance_pressured",    0.02,  0.4),
    ("pass_chance_wing",         0.01,  0.25),
    ("pass_chance_forward",      0.005, 0.18),
    ("pass_chance_default",      0.005, 0.2),
    ("shoot_progress_threshold", 0.55,  0.9),
    ("tackle_chance",            0.01,  0.22),
    ("forward_pass_min_gain",    0.0,   18.0),
    ("mark_distance",            25.0,  85.0),
    ("aggression",               0.0,   2.0),
    ("risk_appetite",            0.0,   1.0),
    ("pass_dir_offensive",       0.0,   2.0),
    ("pass_dir_defensive",       0.0,   2.0),
    ("pass_dir_neutral",         0.0,   2.0),
];

/// Set one named decision param for a single slot (dot-path notation).
fn set_decision_slot(slot: &mut V6Params, param: &str, value: f32) {
    match param {
        "pass_chance_pressured"    => slot.decisions.pass_chance_pressured    = value,
        "pass_chance_wing"         => slot.decisions.pass_chance_wing         = value,
        "pass_chance_forward"      => slot.decisions.pass_chance_forward      = value,
        "pass_chance_default"      => slot.decisions.pass_chance_default      = value,
        "shoot_progress_threshold" => slot.decisions.shoot_progress_threshold = value,
        "tackle_chance"            => slot.decisions.tackle_chance            = value,
        "forward_pass_min_gain"    => slot.decisions.forward_pass_min_gain    = value,
        "mark_distance"            => slot.decisions.mark_distance            = value,
        "aggression"               => slot.decisions.aggression               = value,
        "risk_appetite"            => slot.decisions.risk_appetite            = value,
        "pass_dir_offensive"       => slot.decisions.pass_dir_offensive       = value,
        "pass_dir_defensive"       => slot.decisions.pass_dir_defensive       = value,
        "pass_dir_neutral"         => slot.decisions.pass_dir_neutral         = value,
        _ => {}
    }
}

/// JSON camelCase field name for a decision param.
fn decision_json_field(param: &str) -> &'static str {
    match param {
        "mark_distance"            => "markDistance",
        "pass_chance_pressured"    => "passChancePressured",
        "pass_chance_forward"      => "passChanceForward",
        "pass_chance_wing"         => "passChanceWing",
        "pass_chance_default"      => "passChanceDefault",
        "shoot_progress_threshold" => "shootProgressThreshold",
        "tackle_chance"            => "tackleChance",
        "forward_pass_min_gain"    => "forwardPassMinGain",
        "aggression"               => "aggression",
        "risk_appetite"            => "riskAppetite",
        "pass_dir_offensive"       => "passDirOffensive",
        "pass_dir_defensive"       => "passDirDefensive",
        "pass_dir_neutral"         => "passDirNeutral",
        _ => panic!("unknown param: {}", param),
    }
}

// ─── spatial param specs ──────────────────────────────────────────────────
//
// 5 dimensions × 3 sub-params (min/preferred/max) = 15 params per slot.
// Bounds match `mutate_v6` ranges (same across all slots).
// IMPORTANT: name uses dot-path "spatial.<dim>.<sub>" — must match v6_get/set_field.
// Each "param" is swept across all 5 slots in the optimizer (per-slot ternary).

const ALL_SPATIAL_PARAMS: &[(&str, f32, f32)] = &[
    ("spatial.own_goal.min",       0.0, 900.0),
    ("spatial.own_goal.preferred", 0.0, 900.0),
    ("spatial.own_goal.max",       0.0, 900.0),
    ("spatial.side.min",           0.0, 520.0),
    ("spatial.side.preferred",     0.0, 520.0),
    ("spatial.side.max",           0.0, 520.0),
    ("spatial.ball.min",           0.0, 700.0),
    ("spatial.ball.preferred",     0.0, 700.0),
    ("spatial.ball.max",           0.0, 700.0),
    ("spatial.teammate.min",       0.0, 400.0),
    ("spatial.teammate.preferred", 0.0, 400.0),
    ("spatial.teammate.max",       0.0, 400.0),
    ("spatial.opponent.min",       0.0, 400.0),
    ("spatial.opponent.preferred", 0.0, 400.0),
    ("spatial.opponent.max",       0.0, 400.0),
];

/// Read a value from anneal-result JSON for a single slot, using dot-path notation.
/// Handles both decision params ("decisions.pass_chance_pressured")
/// and spatial params ("spatial.own_goal.preferred", ".min", ".max").
fn read_v6_field_from_json(slot_json: &serde_json::Value, dot_path: &str) -> Option<f64> {
    let parts: Vec<&str> = dot_path.split('.').collect();
    match parts.as_slice() {
        ["decisions", field] => {
            // snake → camel for known decision fields
            let camel = decision_json_field(field);
            slot_json["decisions"][camel].as_f64()
        }
        ["spatial", dim, sub] => {
            // dim is snake_case (own_goal, side, ball, teammate, opponent) — convert
            let dim_camel = match *dim {
                "own_goal" => "ownGoal",
                "side"     => "side",
                "ball"     => "ball",
                "teammate" => "teammate",
                "opponent" => "opponent",
                _ => return None,
            };
            // sub is "min", "preferred", "max" (already camel-compatible)
            slot_json["spatial"][dim_camel][sub].as_f64()
        }
        _ => None,
    }
}

/// Param scope: which families to optimize.
#[derive(Clone, Copy, Debug, PartialEq)]
enum ParamScope { Decision, Spatial, All }

impl ParamScope {
    fn parse(s: &str) -> Self {
        match s {
            "spatial" | "Spatial"   => ParamScope::Spatial,
            "all" | "All"           => ParamScope::All,
            _                       => ParamScope::Decision,
        }
    }
    fn params(self) -> Vec<(&'static str, f32, f32)> {
        // Returns (full_dot_path, lo, hi). Decision params get "decisions." prefix.
        let mut out: Vec<(&'static str, f32, f32)> = Vec::new();
        if matches!(self, ParamScope::Decision | ParamScope::All) {
            // Decision params need the "decisions." prefix added
            for (n, lo, hi) in ALL_DECISION_PARAMS { out.push((decision_full_path(n), *lo, *hi)); }
        }
        if matches!(self, ParamScope::Spatial | ParamScope::All) {
            for &spec in ALL_SPATIAL_PARAMS { out.push(spec); }
        }
        out
    }
    fn label(self) -> &'static str {
        match self {
            ParamScope::Decision => "decision",
            ParamScope::Spatial  => "spatial",
            ParamScope::All      => "all",
        }
    }
}

/// Decision params are stored without the "decisions." prefix in ALL_DECISION_PARAMS.
/// This helper returns the full dot-path version for a decision short-name.
fn decision_full_path(short: &str) -> &'static str {
    match short {
        "pass_chance_pressured"    => "decisions.pass_chance_pressured",
        "pass_chance_wing"         => "decisions.pass_chance_wing",
        "pass_chance_forward"      => "decisions.pass_chance_forward",
        "pass_chance_default"      => "decisions.pass_chance_default",
        "shoot_progress_threshold" => "decisions.shoot_progress_threshold",
        "tackle_chance"            => "decisions.tackle_chance",
        "forward_pass_min_gain"    => "decisions.forward_pass_min_gain",
        "mark_distance"            => "decisions.mark_distance",
        "aggression"               => "decisions.aggression",
        "risk_appetite"            => "decisions.risk_appetite",
        "pass_dir_offensive"       => "decisions.pass_dir_offensive",
        "pass_dir_defensive"       => "decisions.pass_dir_defensive",
        "pass_dir_neutral"         => "decisions.pass_dir_neutral",
        _ => panic!("unknown decision short-name: {}", short),
    }
}

// ─── block coordinate descent optimizer ───────────────────────────────────

/// Block coordinate descent:
///   Round 1: sweep all params → find "better" → combine → lock best combo
///   Round 2: sweep remaining params → find "better" → combine → lock
///   ... until no improvement found
fn run_param_optimize(project_root: &Path, team_name: &str, ablation_games: usize, eval_games: usize, max_rounds: usize, max_better: usize, scope: ParamScope) {
    let team_dir  = project_root.join("data/teams").join(team_name);
    let opt_dir   = team_dir.join(format!("param-optimize-{}", scope.label()));
    std::fs::create_dir_all(&opt_dir).expect("create param-optimize dir");

    // Load original baseline (fixed reference throughout all rounds)
    let orig_file = read_team_baseline_v6(&team_dir.join("baseline.json"))
        .expect("read baseline");
    let original: TeamPolicyV6 = orig_file.player_params;

    let mut working = original;   // evolves each round
    let mut locked: Vec<String> = Vec::new();
    let mut round = 0usize;
    let mut total_point_gain = 0.0f64;
    let scope_params = scope.params();

    crate::game::CLUSTER_START.store(true, std::sync::atomic::Ordering::Relaxed);

    println!("=== PARAM OPTIMIZE: {} ({} scope) ===", team_name, scope.label());
    println!("Method: block coordinate descent (sweep → combine → lock → repeat)");
    println!("Ablation: {}g | Eval: {}g | Anneal: QUICK", ablation_games, eval_games);
    println!("Total params in scope: {}", scope_params.len());
    println!();

    loop {
        round += 1;
        let round_dir = opt_dir.join(format!("round-{:02}", round));
        std::fs::create_dir_all(&round_dir).expect("create round dir");

        // Params to sweep this round (skip locked) — full dot-paths now
        let remaining: Vec<(&str, f32, f32)> = scope_params.iter()
            .filter(|(name, _, _)| !locked.contains(&name.to_string()))
            .copied().collect();

        if remaining.is_empty() {
            println!("All params locked. Done.");
            break;
        }

        println!("╔═══ ROUND {} ═══════════════════════════════════════════╗", round);
        println!("  Locked:    {} params: {}", locked.len(), if locked.is_empty() { "none".to_string() } else { locked.join(", ") });
        println!("  Sweeping:  {} params", remaining.len());
        println!();

        // ── Phase 1: sweep remaining params ───────────────────────────────
        let mut better_with_score: Vec<(String, f64)> = Vec::new();

        for (i, &(param, lo, hi)) in remaining.iter().enumerate() {
            // param is now full dot-path (e.g. "decisions.aggression" or "spatial.own_goal.preferred")
            // Use last 2 path components as folder name to keep it filesystem-safe
            let folder_name: String = param.replace('.', "__");
            let param_dir = round_dir.join(&folder_name);
            std::fs::create_dir_all(&param_dir).expect("create param dir");
            let field = param.to_string();

            println!("  [{}/{}] sweeping {}", i + 1, remaining.len(), param);

            // Ternary ablation per slot on working policy
            let mut candidate = working;
            let mut accepted = 0usize;
            for slot in 0..5 {
                accepted += ablate_v6_field_ternary(
                    &mut candidate, slot, &field, lo, hi, 4, ablation_games, 1.5,
                );
            }

            // QUICK anneal from ternary result
            let anneal_dir = param_dir.join("anneal");
            std::fs::create_dir_all(&anneal_dir).expect("create anneal dir");
            let champion = run_team_anneal(&anneal_dir, team_name, candidate, ANNEAL_STAGES_QUICK);

            // Evaluate vs original
            let eval = evaluate_team_policies_v6(&original, &champion, eval_games);
            let pt_diff = eval.candidate_points - eval.baseline_points;
            let verdict = if pt_diff > 0.0 { "better ✓" } else if pt_diff < 0.0 { "worse" } else { "draw" };
            println!("    → ptdiff={:+.0} W{}D{}L{} | {}", pt_diff, eval.wins, eval.draws, eval.losses, verdict);

            // Save per-param result
            let r = serde_json::json!({
                "param": param, "round": round,
                "ternaryAccepted": accepted,
                "pointDiff": pt_diff,
                "candidatePoints": eval.candidate_points,
                "baselinePoints": eval.baseline_points,
                "wins": eval.wins, "draws": eval.draws, "losses": eval.losses,
                "runAt": iso_now(),
            });
            let _ = std::fs::write(param_dir.join("result.json"),
                format!("{}\n", serde_json::to_string_pretty(&r).unwrap()));

            if pt_diff > 0.0 {
                better_with_score.push((param.to_string(), pt_diff));
            }
        }

        // Sort by point_diff descending and keep top max_better
        better_with_score.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let total_better_found = better_with_score.len();
        better_with_score.truncate(max_better);
        let better_params: Vec<String> = better_with_score.iter().map(|(p, _)| p.clone()).collect();

        println!();
        println!("  Found {} better params total (taking top {})", total_better_found, better_params.len());
        for (p, s) in &better_with_score {
            println!("    {} (+{:.0})", p, s);
        }

        if better_params.is_empty() {
            println!("\n  No improvement found in round {}. Converged!", round);
            break;
        }

        // ── Phase 2: combine all 2^N combos of top-N better params ────────
        println!("\n  Testing {} combinations of top {} better params...", 1 << better_params.len(), better_params.len());
        let n = better_params.len();
        let n_combos = 1usize << n;
        let combine_dir = round_dir.join("combine");
        std::fs::create_dir_all(&combine_dir).expect("create combine dir");

        let mut best_policy  = working;
        let mut best_pt_diff = 0.0f64;
        let mut best_label   = "working (no change)".to_string();

        for mask in 0..n_combos {
            let active: Vec<&str> = (0..n)
                .filter(|i| mask & (1 << i) != 0)
                .map(|i| better_params[i].as_str())
                .collect();
            let label = if active.is_empty() { "baseline".to_string() } else { active.join("+") };

            // Build starting policy: working + overrides from each selected param's anneal.
            // param is now a full dot-path (e.g. "decisions.aggression" or "spatial.own_goal.preferred")
            let mut start = working;
            for param in &active {
                let folder = param.replace('.', "__");
                let anneal_path = round_dir.join(&folder).join("anneal").join("baseline.json");
                if let Ok(text) = std::fs::read_to_string(&anneal_path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(slots) = json["playerParams"].as_array() {
                            for (s, slot_json) in slots.iter().enumerate() {
                                if let Some(val) = read_v6_field_from_json(slot_json, param) {
                                    v6_set_field(&mut start[s], param, val as f32);
                                }
                            }
                        }
                    }
                }
            }

            let combo_dir = combine_dir.join(format!("{:0>width$b}", mask, width = n));
            std::fs::create_dir_all(&combo_dir).expect("create combo dir");
            let champion = run_team_anneal(&combo_dir, team_name, start, ANNEAL_STAGES_QUICK);

            let eval = evaluate_team_policies_v6(&original, &champion, eval_games);
            let pt_diff = eval.candidate_points - eval.baseline_points;
            println!("    {:40} ptdiff={:+.0} W{}D{}L{}",
                label, pt_diff, eval.wins, eval.draws, eval.losses);

            let combo_r = serde_json::json!({
                "mask": mask, "label": &label, "activeParams": &active,
                "pointDiff": pt_diff,
                "wins": eval.wins, "draws": eval.draws, "losses": eval.losses,
                "runAt": iso_now(),
            });
            let _ = std::fs::write(combo_dir.join("result.json"),
                format!("{}\n", serde_json::to_string_pretty(&combo_r).unwrap()));

            if pt_diff > best_pt_diff {
                best_pt_diff = pt_diff;
                best_policy  = champion;
                best_label   = label;
            }
        }

        if best_pt_diff <= 0.0 {
            println!("\n  No combination improved over original. Converged!");
            break;
        }

        // Lock the params from the best combo
        let best_active: Vec<String> = (0..n)
            .filter(|i| best_label.contains(better_params[*i].as_str()))
            .map(|i| better_params[i].clone())
            .collect();

        println!("\n  ✓ Round {} winner: '{}' (ptdiff={:+.0})", round, best_label, best_pt_diff);
        println!("  Locking: {:?}", best_active);

        for p in &best_active { locked.push(p.clone()); }
        working = best_policy;
        total_point_gain += best_pt_diff;

        // Save round summary + intermediate baseline
        let round_summary = serde_json::json!({
            "round": round, "bestCombo": &best_label,
            "lockedThisRound": &best_active,
            "pointDiffVsOriginal": best_pt_diff,
            "totalPointGain": total_point_gain,
            "allLocked": &locked,
            "runAt": iso_now(),
        });
        let _ = std::fs::write(round_dir.join("summary.json"),
            format!("{}\n", serde_json::to_string_pretty(&round_summary).unwrap()));

        println!("  Total gain so far: {:+.0} pts vs original", total_point_gain);
        println!("╚═══════════════════════════════════════════════════════════╝\n");

        if round >= max_rounds {
            println!("Reached max rounds ({}). Stopping.", max_rounds);
            break;
        }
    }

    // Save final optimized policy
    let final_doc = serde_json::json!({
        "name": team_name,
        "type": "team-policy-v6",
        "description": format!("{}: block-coord-descent optimized, {} rounds, {:+.0} pt gain", team_name, round, total_point_gain),
        "playerParams": working,
        "lockedParams": &locked,
        "rounds": round,
        "totalPointGain": total_point_gain,
        "optimizedAt": iso_now(),
    });
    let final_path = opt_dir.join("optimized-baseline.json");
    let _ = std::fs::write(&final_path,
        format!("{}\n", serde_json::to_string_pretty(&final_doc).unwrap()));

    println!("=== OPTIMIZATION COMPLETE ===");
    println!("Rounds: {} | Total pt gain vs original: {:+.0}", round, total_point_gain);
    println!("Locked params: {}", if locked.is_empty() { "none".to_string() } else { locked.join(", ") });
    println!("Final policy → {}", final_path.display());
    println!("\nTo apply: copy optimized-baseline.json to baseline.json and run --v6-team-train");
}

/// Combined parameter sweep for `team_name`:
/// Phase 1: Ternary ablation per parameter per slot → finds local optimum fast.
/// Phase 2: QUICK anneal from the ternary-found starting point → escapes local minima.
/// Metric: football points (3=win, 1=draw, 0=loss) vs all other v6 teams.
/// Output: data/teams/<team>/param-sweep/<param>/  +  summary.md
fn run_param_sweep(project_root: &Path, team_name: &str, ablation_games: usize, eval_games: usize) {
    let team_dir = project_root.join("data").join("teams").join(team_name);
    let baseline_path = team_dir.join("baseline.json");

    let baseline_file = match read_team_baseline_v6(&baseline_path) {
        Ok(b) => b,
        Err(e) => { eprintln!("Cannot read baseline for {}: {}", team_name, e); std::process::exit(1); }
    };
    let original: TeamPolicyV6 = baseline_file.player_params;

    let sweep_dir = team_dir.join("param-sweep");
    std::fs::create_dir_all(&sweep_dir).expect("create param-sweep dir");

    // Decision param specs using the dot-path notation for v6_get/set_field
    let param_specs: &[(&str, f32, f32)] = &[
        ("decisions.pass_chance_pressured",    0.02,  0.4),
        ("decisions.pass_chance_wing",         0.01,  0.25),
        ("decisions.pass_chance_forward",      0.005, 0.18),
        ("decisions.pass_chance_default",      0.005, 0.2),
        ("decisions.shoot_progress_threshold", 0.55,  0.9),
        ("decisions.tackle_chance",            0.01,  0.22),
        ("decisions.forward_pass_min_gain",    0.0,   18.0),
        ("decisions.mark_distance",            25.0,  85.0),
        ("decisions.aggression",               0.0,   2.0),
        ("decisions.risk_appetite",            0.0,   1.0),
        ("decisions.pass_dir_offensive",       0.0,   2.0),
        ("decisions.pass_dir_defensive",       0.0,   2.0),
        ("decisions.pass_dir_neutral",         0.0,   2.0),
    ];

    println!("=== PARAM SWEEP: {} ===", team_name);
    println!("Method: ternary ablation ({}g/probe) → QUICK anneal → points eval ({}g)", ablation_games, eval_games);
    println!("Decision params: {} | Output: {}", param_specs.len(), sweep_dir.display());
    println!();

    crate::game::CLUSTER_START.store(true, std::sync::atomic::Ordering::Relaxed);
    const Z_ACCEPT_TERNARY: f64 = 1.5;
    const TERNARY_DEPTH: usize = 4;

    struct SweepResult {
        field: String,
        original_vals: Vec<f32>,  // one per slot
        ternary_vals:  Vec<f32>,  // after ablation
        point_diff: f64,
        wins: u64, draws: u64, losses: u64,
    }
    let mut results: Vec<SweepResult> = Vec::new();
    let sweep_start = std::time::Instant::now();

    for (param_idx, &(field, lo, hi)) in param_specs.iter().enumerate() {
        let short_name = field.trim_start_matches("decisions.");
        let param_dir = sweep_dir.join(short_name);
        std::fs::create_dir_all(&param_dir).expect("create param dir");

        println!("[{}/{}] {}", param_idx + 1, param_specs.len(), field);

        // Record original values per slot
        let original_vals: Vec<f32> = (0..5).map(|s| v6_get_field(&original[s], field)).collect();
        println!("  original: {:?}", original_vals.iter().map(|v| format!("{:.4}", v)).collect::<Vec<_>>());

        // ── Phase 1: ternary ablation per slot ────────────────────────────
        let mut candidate = original;
        let mut total_accepted = 0;
        for slot in 0..5 {
            let accepted = ablate_v6_field_ternary(
                &mut candidate, slot, field, lo, hi,
                TERNARY_DEPTH, ablation_games, Z_ACCEPT_TERNARY,
            );
            total_accepted += accepted;
        }
        let ternary_vals: Vec<f32> = (0..5).map(|s| v6_get_field(&candidate[s], field)).collect();
        println!("  ternary:  {:?} ({} accepted)", ternary_vals.iter().map(|v| format!("{:.4}", v)).collect::<Vec<_>>(), total_accepted);

        // ── Phase 2: QUICK anneal from ternary result ─────────────────────
        println!("  running QUICK anneal from ternary result...");
        let anneal_dir = param_dir.join("anneal");
        std::fs::create_dir_all(&anneal_dir).expect("create anneal dir");
        let final_team = run_team_anneal(&anneal_dir, team_name, candidate, ANNEAL_STAGES_QUICK);

        // ── Phase 3: evaluate with points vs original ─────────────────────
        // baseline=original, candidate=final_team
        // eval.wins = final_team wins, eval.losses = original wins
        let eval = evaluate_team_policies_v6(&original, &final_team, eval_games);
        let point_diff = eval.candidate_points - eval.baseline_points;
        let verdict = if point_diff > 0.0 { "NEW REGION BETTER" } else if point_diff < 0.0 { "ORIGINAL BETTER" } else { "DRAW" };
        println!("  eval: pts={:.0}-{:.0} W{}D{}L{} | {}",
            eval.candidate_points, eval.baseline_points,
            eval.wins, eval.draws, eval.losses, verdict);

        // Save JSON
        let r = serde_json::json!({
            "field": field,
            "lo": lo, "hi": hi,
            "originalValues": original_vals,
            "ternaryValues": ternary_vals,
            "ternaryAccepted": total_accepted,
            "pointDiff": point_diff,
            "candidatePoints": eval.candidate_points,
            "baselinePoints": eval.baseline_points,
            "wins": eval.wins, "draws": eval.draws, "losses": eval.losses,
            "ablationGames": ablation_games, "evalGames": eval_games,
            "runAt": iso_now(),
        });
        let _ = std::fs::write(param_dir.join("result.json"),
            format!("{}\n", serde_json::to_string_pretty(&r).unwrap()));

        results.push(SweepResult {
            field: field.to_string(),
            original_vals, ternary_vals,
            point_diff,
            wins: eval.wins, draws: eval.draws, losses: eval.losses,
        });
        println!();
    }

    // ── summary table ──────────────────────────────────────────────────────
    println!("\n=== PARAM SWEEP SUMMARY: {} ===", team_name);
    println!("{:<35}  {:>8}  {:>4}  {:>4}  {:>4}  {}", "param", "pt-diff", "W", "D", "L", "verdict");
    println!("{}", "-".repeat(65));

    let mut md = format!("# Param sweep — {} — {}\n\n", team_name, iso_now());
    md.push_str(&format!("Method: ternary ablation ({}g) → QUICK anneal → points eval ({}g)\n\n", ablation_games, eval_games));
    md.push_str("| Param | Pt diff | W | D | L | Verdict |\n|-------|---------|---|---|---|--------|\n");

    for r in &results {
        let short = r.field.trim_start_matches("decisions.");
        let verdict = if r.point_diff > 0.0 { "better" } else if r.point_diff < 0.0 { "worse" } else { "draw" };
        println!("{:<35}  {:>+8.0}  {:>4}  {:>4}  {:>4}  {}", short, r.point_diff, r.wins, r.draws, r.losses, verdict);
        md.push_str(&format!("| {} | {:+.0} | {} | {} | {} | {} |\n", short, r.point_diff, r.wins, r.draws, r.losses, verdict));
    }

    let _ = std::fs::write(sweep_dir.join("summary.md"), &md);
    let elapsed = sweep_start.elapsed();
    println!("\nSweep complete in {:.0}s → {}", elapsed.as_secs_f64(), sweep_dir.display());
}

/// Test all 2^N combinations of param sweep improvements.
/// For each combination: take original baseline, override selected params' per-slot values
/// from their respective sweep anneal results, run QUICK anneal, evaluate with points.
fn run_param_combine(project_root: &Path, team_name: &str, params: &[String], eval_games: usize) {
    let team_dir   = project_root.join("data/teams").join(team_name);
    let sweep_dir  = team_dir.join("param-sweep");
    let combine_dir = sweep_dir.join("combine");
    std::fs::create_dir_all(&combine_dir).expect("create combine dir");

    // Map param short-name → decisions field name (camelCase in JSON)
    fn field_name(param: &str) -> &'static str {
        match param {
            "mark_distance"            => "markDistance",
            "pass_chance_pressured"    => "passChancePressured",
            "pass_chance_forward"      => "passChanceForward",
            "pass_chance_wing"         => "passChanceWing",
            "pass_chance_default"      => "passChanceDefault",
            "shoot_progress_threshold" => "shootProgressThreshold",
            "tackle_chance"            => "tackleChance",
            "forward_pass_min_gain"    => "forwardPassMinGain",
            "aggression"               => "aggression",
            "risk_appetite"            => "riskAppetite",
            "pass_dir_offensive"       => "passDirOffensive",
            "pass_dir_defensive"       => "passDirDefensive",
            "pass_dir_neutral"         => "passDirNeutral",
            _ => panic!("unknown param: {}", param),
        }
    }

    // Load original baseline
    let orig_baseline = read_team_baseline_v6(&team_dir.join("baseline.json"))
        .expect("read original baseline");
    let original = orig_baseline.player_params;

    // Load anneal results for each param (raw JSON for per-slot value extraction)
    let mut anneal_jsons: Vec<serde_json::Value> = Vec::new();
    for param in params {
        let anneal_path = sweep_dir.join(param).join("anneal").join("baseline.json");
        let text = std::fs::read_to_string(&anneal_path)
            .unwrap_or_else(|_| panic!("cannot read anneal result for {}", param));
        let json: serde_json::Value = serde_json::from_str(&text)
            .unwrap_or_else(|_| panic!("cannot parse anneal JSON for {}", param));
        anneal_jsons.push(json);
    }

    let n = params.len();
    let n_combinations = 1usize << n;

    println!("=== PARAM COMBINE: {} ===", team_name);
    println!("Params: {}", params.join(", "));
    println!("Combinations: 2^{} = {}", n, n_combinations);
    println!("Eval games: {} | Anneal: QUICK", eval_games);
    println!();

    crate::game::CLUSTER_START.store(true, std::sync::atomic::Ordering::Relaxed);

    struct ComboResult {
        mask: usize,
        label: String,
        point_diff: f64,
        wins: u64, draws: u64, losses: u64,
    }
    let mut results: Vec<ComboResult> = Vec::new();

    for mask in 0..n_combinations {
        // Build label
        let active: Vec<&str> = params.iter().enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .map(|(_, p)| p.as_str())
            .collect();
        let label = if active.is_empty() {
            "original".to_string()
        } else {
            active.join("+")
        };

        println!("[{}/{}] {}", mask + 1, n_combinations, label);

        // Build starting policy: original + selected param overrides
        let mut start = original;
        for (i, param) in params.iter().enumerate() {
            if mask & (1 << i) == 0 { continue; }
            let jfield = field_name(param);
            let anneal_slots = anneal_jsons[i]["playerParams"].as_array()
                .expect("playerParams array");
            for slot in 0..5 {
                let val = anneal_slots[slot]["decisions"][jfield]
                    .as_f64()
                    .unwrap_or_else(|| panic!("missing {} in slot {}", jfield, slot)) as f32;
                // Set via v6_set_field using decisions. prefix
                let dotpath = format!("decisions.{}", param.replace('_', "_"));
                // Use the match-based setter directly
                match param.as_str() {
                    "mark_distance"            => start[slot].decisions.mark_distance            = val,
                    "pass_chance_pressured"    => start[slot].decisions.pass_chance_pressured    = val,
                    "pass_chance_forward"      => start[slot].decisions.pass_chance_forward      = val,
                    "pass_chance_wing"         => start[slot].decisions.pass_chance_wing         = val,
                    "pass_chance_default"      => start[slot].decisions.pass_chance_default      = val,
                    "shoot_progress_threshold" => start[slot].decisions.shoot_progress_threshold = val,
                    "tackle_chance"            => start[slot].decisions.tackle_chance            = val,
                    "forward_pass_min_gain"    => start[slot].decisions.forward_pass_min_gain    = val,
                    "aggression"               => start[slot].decisions.aggression               = val,
                    "risk_appetite"            => start[slot].decisions.risk_appetite            = val,
                    "pass_dir_offensive"       => start[slot].decisions.pass_dir_offensive       = val,
                    "pass_dir_defensive"       => start[slot].decisions.pass_dir_defensive       = val,
                    "pass_dir_neutral"         => start[slot].decisions.pass_dir_neutral         = val,
                    _ => {}
                }
                let _ = dotpath; // suppress unused warning
            }
        }

        // Run QUICK anneal
        let run_dir = combine_dir.join(format!("{:0>width$b}-{}", mask, label, width = n));
        std::fs::create_dir_all(&run_dir).expect("create combo dir");
        let champion = run_team_anneal(&run_dir, team_name, start, ANNEAL_STAGES_QUICK);

        // Evaluate vs original
        let eval = evaluate_team_policies_v6(&original, &champion, eval_games);
        let point_diff = eval.candidate_points - eval.baseline_points;
        println!("  → pts={:.0}-{:.0} W{}D{}L{} diff={:+.0}",
            eval.candidate_points, eval.baseline_points,
            eval.wins, eval.draws, eval.losses, point_diff);

        // Save result JSON
        let r = serde_json::json!({
            "mask": mask, "label": &label,
            "activeParams": &active,
            "pointDiff": point_diff,
            "candidatePoints": eval.candidate_points,
            "baselinePoints": eval.baseline_points,
            "wins": eval.wins, "draws": eval.draws, "losses": eval.losses,
            "evalGames": eval_games, "runAt": iso_now(),
        });
        let _ = std::fs::write(run_dir.join("result.json"),
            format!("{}\n", serde_json::to_string_pretty(&r).unwrap()));

        results.push(ComboResult { mask, label, point_diff,
            wins: eval.wins, draws: eval.draws, losses: eval.losses });
        println!();
    }

    // Sort by point_diff descending
    results.sort_by(|a, b| b.point_diff.partial_cmp(&a.point_diff).unwrap());

    println!("\n=== COMBINATION RESULTS (sorted) ===");
    println!("{:<50}  {:>8}  {:>4}  {:>4}  {:>4}", "combination", "pt-diff", "W", "D", "L");
    println!("{}", "-".repeat(72));

    let mut md = format!("# Param combine — {} — {}\n\n", team_name, iso_now());
    md.push_str(&format!("Params: {} | Eval: {}g | Anneal: QUICK\n\n", params.join(", "), eval_games));
    md.push_str("| Combination | Pt diff | W | D | L |\n|-------------|---------|---|---|---|\n");

    for r in &results {
        println!("{:<50}  {:>+8.0}  {:>4}  {:>4}  {:>4}",
            r.label, r.point_diff, r.wins, r.draws, r.losses);
        md.push_str(&format!("| {} | {:+.0} | {} | {} | {} |\n",
            r.label, r.point_diff, r.wins, r.draws, r.losses));
    }

    let _ = std::fs::write(combine_dir.join("summary.md"), &md);
    println!("\nResults → {}", combine_dir.display());
}

// ════════════════════════════════════════════════════════════════════════════
// V6 Rough-calibration team (ternary ablation, "Lite" scope)
// ════════════════════════════════════════════════════════════════════════════

const V6_LITE_FIELDS: &[(&str, f32, f32)] = &[
    ("spatial.own_goal.preferred", 0.0, 900.0),
    ("spatial.side.preferred",     0.0, 520.0),
    ("spatial.ball.preferred",     0.0, 700.0),
    ("spatial.teammate.preferred", 20.0, 400.0),
    ("spatial.opponent.preferred", 15.0, 400.0),
    ("decisions.pass_chance_pressured", 0.02, 0.4),
    ("decisions.pass_chance_wing",      0.01, 0.25),
    ("decisions.pass_chance_forward",   0.005, 0.18),
    ("decisions.pass_chance_default",   0.005, 0.2),
    ("decisions.shoot_progress_threshold", 0.55, 0.9),
    ("decisions.tackle_chance",         0.01, 0.22),
    ("decisions.forward_pass_min_gain", 0.0, 18.0),
    ("decisions.mark_distance",         25.0, 85.0),
    ("decisions.aggression",            0.0, 2.0),
    ("decisions.risk_appetite",         0.0, 1.0),
    ("decisions.pass_dir_offensive",    0.0, 2.0),
    ("decisions.pass_dir_defensive",    0.0, 2.0),
    ("decisions.pass_dir_neutral",      0.0, 2.0),
];

fn v6_get_field(p: &V6Params, path: &str) -> f32 {
    match path {
        "spatial.own_goal.min"       => p.spatial.own_goal.min,
        "spatial.own_goal.preferred" => p.spatial.own_goal.preferred,
        "spatial.own_goal.max"       => p.spatial.own_goal.max,
        "spatial.side.min"           => p.spatial.side.min,
        "spatial.side.preferred"     => p.spatial.side.preferred,
        "spatial.side.max"           => p.spatial.side.max,
        "spatial.ball.min"           => p.spatial.ball.min,
        "spatial.ball.preferred"     => p.spatial.ball.preferred,
        "spatial.ball.max"           => p.spatial.ball.max,
        "spatial.teammate.min"       => p.spatial.teammate.min,
        "spatial.teammate.preferred" => p.spatial.teammate.preferred,
        "spatial.teammate.max"       => p.spatial.teammate.max,
        "spatial.opponent.min"       => p.spatial.opponent.min,
        "spatial.opponent.preferred" => p.spatial.opponent.preferred,
        "spatial.opponent.max"       => p.spatial.opponent.max,
        "decisions.pass_chance_pressured" => p.decisions.pass_chance_pressured,
        "decisions.pass_chance_wing"      => p.decisions.pass_chance_wing,
        "decisions.pass_chance_forward"   => p.decisions.pass_chance_forward,
        "decisions.pass_chance_default"   => p.decisions.pass_chance_default,
        "decisions.shoot_progress_threshold" => p.decisions.shoot_progress_threshold,
        "decisions.tackle_chance"         => p.decisions.tackle_chance,
        "decisions.forward_pass_min_gain" => p.decisions.forward_pass_min_gain,
        "decisions.mark_distance"         => p.decisions.mark_distance,
        "decisions.aggression"            => p.decisions.aggression,
        "decisions.risk_appetite"         => p.decisions.risk_appetite,
        "decisions.pass_dir_offensive"    => p.decisions.pass_dir_offensive,
        "decisions.pass_dir_defensive"    => p.decisions.pass_dir_defensive,
        "decisions.pass_dir_neutral"      => p.decisions.pass_dir_neutral,
        _ => panic!("unknown V6 field: {}", path),
    }
}

fn v6_set_field(p: &mut V6Params, path: &str, value: f32) {
    // Helper: set min/max while keeping min ≤ preferred ≤ max invariant
    fn set_min(d: &mut crate::policy::DistancePref, v: f32) {
        d.min = v.min(d.max);
        if d.preferred < d.min { d.preferred = d.min; }
    }
    fn set_max(d: &mut crate::policy::DistancePref, v: f32) {
        d.max = v.max(d.min);
        if d.preferred > d.max { d.preferred = d.max; }
    }
    match path {
        "spatial.own_goal.min"       => set_min(&mut p.spatial.own_goal, value),
        "spatial.own_goal.preferred" => { p.spatial.own_goal.preferred = value; p.spatial.own_goal.clamp_self(); },
        "spatial.own_goal.max"       => set_max(&mut p.spatial.own_goal, value),
        "spatial.side.min"           => set_min(&mut p.spatial.side, value),
        "spatial.side.preferred"     => { p.spatial.side.preferred = value; p.spatial.side.clamp_self(); },
        "spatial.side.max"           => set_max(&mut p.spatial.side, value),
        "spatial.ball.min"           => set_min(&mut p.spatial.ball, value),
        "spatial.ball.preferred"     => { p.spatial.ball.preferred = value; p.spatial.ball.clamp_self(); },
        "spatial.ball.max"           => set_max(&mut p.spatial.ball, value),
        "spatial.teammate.min"       => set_min(&mut p.spatial.teammate, value),
        "spatial.teammate.preferred" => { p.spatial.teammate.preferred = value; p.spatial.teammate.clamp_self(); },
        "spatial.teammate.max"       => set_max(&mut p.spatial.teammate, value),
        "spatial.opponent.min"       => set_min(&mut p.spatial.opponent, value),
        "spatial.opponent.preferred" => { p.spatial.opponent.preferred = value; p.spatial.opponent.clamp_self(); },
        "spatial.opponent.max"       => set_max(&mut p.spatial.opponent, value),
        "decisions.pass_chance_pressured" => p.decisions.pass_chance_pressured = value,
        "decisions.pass_chance_wing"      => p.decisions.pass_chance_wing = value,
        "decisions.pass_chance_forward"   => p.decisions.pass_chance_forward = value,
        "decisions.pass_chance_default"   => p.decisions.pass_chance_default = value,
        "decisions.shoot_progress_threshold" => p.decisions.shoot_progress_threshold = value,
        "decisions.tackle_chance"         => p.decisions.tackle_chance = value,
        "decisions.forward_pass_min_gain" => p.decisions.forward_pass_min_gain = value,
        "decisions.mark_distance"         => p.decisions.mark_distance = value,
        "decisions.aggression"            => p.decisions.aggression = value,
        "decisions.risk_appetite"         => p.decisions.risk_appetite = value,
        "decisions.pass_dir_offensive"    => p.decisions.pass_dir_offensive = value,
        "decisions.pass_dir_defensive"    => p.decisions.pass_dir_defensive = value,
        "decisions.pass_dir_neutral"      => p.decisions.pass_dir_neutral = value,
        _ => panic!("unknown V6 field: {}", path),
    }
}

fn ablate_v6_field_ternary(
    champion: &mut TeamPolicyV6,
    slot: usize,
    field: &str,
    lo: f32,
    hi: f32,
    max_depth: usize,
    games: usize,
    z_accept: f64,
) -> usize {
    let mut interval_lo = lo;
    let mut interval_hi = hi;
    let mut accepted = 0usize;

    for depth in 0..max_depth {
        let mid = (interval_lo + interval_hi) / 2.0;
        let current_val = v6_get_field(&champion[slot], field);
        let mut best_target = current_val;
        let mut best_z: f64 = z_accept;

        for (label, target) in [("lo", interval_lo), ("mid", mid), ("hi", interval_hi)] {
            if (target - current_val).abs() < 1e-6 { continue; }
            let mut variant = *champion;
            v6_set_field(&mut variant[slot], field, target);
            let eval = evaluate_team_policies_v6(champion, &variant, games);
            // POINTS-BASED ternary: use point z-score (W/D/L) instead of goal-diff z-score
            let won = eval.point_z_score > best_z;
            println!(
                "    d{} slot={} {}={:.3}->{:.3} ({}) ptdiff={:+.0} pz={:+.2} (gd={:+.0} gz={:+.2}) g={}/{}{}",
                depth, slot, field, current_val, target, label,
                eval.point_diff, eval.point_z_score,
                eval.goal_diff, eval.z_score, eval.games, games,
                if won { " *winner*" } else { "" });
            if won { best_z = eval.point_z_score; best_target = target; }
        }

        if best_target == current_val { break; }
        v6_set_field(&mut champion[slot], field, best_target);
        accepted += 1;

        let span = interval_hi - interval_lo;
        if (best_target - interval_lo).abs() < 1e-6 { interval_hi = mid; }
        else if (best_target - interval_hi).abs() < 1e-6 { interval_lo = mid; }
        else {
            let q = span / 4.0;
            interval_lo = (best_target - q).max(lo);
            interval_hi = (best_target + q).min(hi);
        }
    }
    accepted
}

fn run_v6_rough_team(project_root: &Path, team_name: &str, team_desc: &str, max_depth: usize, games: usize) {
    crate::game::CLUSTER_START.store(true, std::sync::atomic::Ordering::Relaxed);
    const Z_ACCEPT: f64 = 1.5;

    let teams_dir = project_root.join("data").join("teams");
    let team_dir = teams_dir.join(team_name);
    let _ = std::fs::remove_dir_all(&team_dir);
    std::fs::create_dir_all(&team_dir).expect("create team dir");

    let mut champion: TeamPolicyV6 = [
        policy::v6_default_for_slot(0),
        policy::v6_default_for_slot(1),
        policy::v6_default_for_slot(2),
        policy::v6_default_for_slot(3),
        policy::v6_default_for_slot(4),
    ];

    println!("=== V6 ROUGH-CAL TEAM: {} ===", team_name);
    println!("Fields: {} × 5 slots = {} ablations, depth={}, games={}",
             V6_LITE_FIELDS.len(), V6_LITE_FIELDS.len() * 5, max_depth, games);

    let total_start = std::time::Instant::now();
    let mut total_accepted = 0;
    for slot in 0..5 {
        for &(field, lo, hi) in V6_LITE_FIELDS {
            println!("  slot {} field {}", slot, field);
            let accepted = ablate_v6_field_ternary(&mut champion, slot, field, lo, hi, max_depth, games, Z_ACCEPT);
            total_accepted += accepted;
        }
    }
    let elapsed = total_start.elapsed();
    let h = elapsed.as_secs() / 3600;
    let m = (elapsed.as_secs() % 3600) / 60;
    let s = elapsed.as_secs() % 60;
    println!("\n=== ROUGH-CAL DONE in {}h {}m {}s — {} field-improvements accepted ===",
             h, m, s, total_accepted);

    let final_doc = serde_json::json!({
        "name": team_name, "version": 1,
        "type": "team-policy-v6",
        "description": format!("{}: deterministic ternary-ablation rough calibration ({} fields)", team_name, V6_LITE_FIELDS.len() * 5),
        "playerParams": champion,
        "trainedAt": iso_now(),
        "trainingMethod": "ternary-ablation-lite",
    });
    let _ = std::fs::write(team_dir.join("baseline.json"),
        format!("{}\n", serde_json::to_string_pretty(&final_doc).unwrap()));
    write_team_info_md(&team_dir, team_name, team_desc, &champion);
    write_team_layout_svg(&team_dir.join("layout.svg"), team_name, team_desc, &champion);
}

// ════════════════════════════════════════════════════════════════════════════
// Per-team layout SVG (preferred positions + spatial awareness)
// ════════════════════════════════════════════════════════════════════════════

fn compute_v6_preferred_xy(params: &V6Params, own_goal_x: f32) -> (f32, f32) {
    let target_y = params.spatial.side.preferred.clamp(20.0, 500.0);
    let dy = target_y - 260.0;
    let pref = params.spatial.own_goal.preferred;
    let dx_sq = pref * pref - dy * dy;
    let target_x = if dx_sq > 0.0 {
        own_goal_x + dx_sq.sqrt()
    } else {
        own_goal_x + pref
    };
    (target_x.clamp(20.0, 860.0), target_y)
}

pub fn write_team_layout_svg(out_path: &Path, team_name: &str, team_desc: &str, params: &TeamPolicyV6) {
    let mut svg = String::new();
    svg.push_str(r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 880 660" font-family="ui-monospace,Menlo,monospace">"##);
    // Pitch
    svg.push_str(r##"<rect width="880" height="520" fill="#2a6318"/>"##);
    // Stripes for atmosphere
    for i in 0..11 {
        let opacity = if i % 2 == 0 { "0.06" } else { "0.025" };
        let fill = if i % 2 == 0 { "black" } else { "white" };
        svg.push_str(&format!(r##"<rect x="{}" y="0" width="80" height="520" fill="{}" fill-opacity="{}"/>"##, i*80, fill, opacity));
    }
    // Field lines
    svg.push_str(r##"<rect x="18" y="8" width="844" height="504" fill="none" stroke="rgba(255,255,255,0.82)" stroke-width="2"/>"##);
    svg.push_str(r##"<line x1="440" y1="8" x2="440" y2="512" stroke="rgba(255,255,255,0.82)" stroke-width="2"/>"##);
    svg.push_str(r##"<circle cx="440" cy="260" r="62" fill="none" stroke="rgba(255,255,255,0.82)" stroke-width="2"/>"##);
    svg.push_str(r##"<circle cx="440" cy="260" r="3" fill="rgba(255,255,255,0.8)"/>"##);
    // Penalty areas + goal areas
    svg.push_str(r##"<rect x="18" y="172" width="106" height="176" fill="none" stroke="rgba(255,255,255,0.82)" stroke-width="2"/>"##);
    svg.push_str(r##"<rect x="756" y="172" width="106" height="176" fill="none" stroke="rgba(255,255,255,0.82)" stroke-width="2"/>"##);
    svg.push_str(r##"<rect x="18" y="214" width="54" height="92" fill="none" stroke="rgba(255,255,255,0.82)" stroke-width="2"/>"##);
    svg.push_str(r##"<rect x="808" y="214" width="54" height="92" fill="none" stroke="rgba(255,255,255,0.82)" stroke-width="2"/>"##);
    // Goals
    svg.push_str(r##"<rect x="-8" y="195" width="26" height="130" fill="rgba(255,255,255,0.1)" stroke="rgba(255,255,255,0.9)" stroke-width="3"/>"##);
    svg.push_str(r##"<rect x="862" y="195" width="26" height="130" fill="rgba(255,255,255,0.1)" stroke="rgba(255,255,255,0.9)" stroke-width="3"/>"##);

    let slots = ["FWD", "MID-T", "MID-B", "DEF", "GK"];
    let colors = ["#ff6b35", "#5b9bff", "#5b9bff", "#84cc16", "#fbbf24"];
    let own_goal_x = 18.0_f32;

    // Spatial-awareness rings (own_goal min/max as circles around own goal)
    for i in 0..5 {
        let s = &params[i].spatial;
        // Min/max as faint arcs from own goal point
        svg.push_str(&format!(r##"<circle cx="{}" cy="260" r="{:.0}" fill="none" stroke="{}" stroke-width="1" stroke-opacity="0.18" stroke-dasharray="4,4"/>"##,
            own_goal_x, s.own_goal.min, colors[i]));
        svg.push_str(&format!(r##"<circle cx="{}" cy="260" r="{:.0}" fill="none" stroke="{}" stroke-width="1" stroke-opacity="0.18" stroke-dasharray="4,4"/>"##,
            own_goal_x, s.own_goal.max, colors[i]));
    }

    // Player markers: preferred position + comfort zone (opponent.preferred as repel-radius hint)
    for i in 0..5 {
        let s = &params[i].spatial;
        // GK spatial params are overridden by goal-line logic in both engines —
        // always show GK at the actual patrol position instead of misleading spatial prefs.
        let (px, py) = if i == 4 {
            // GK patrol position: FIELD_LINE + PR*1.5 = 18 + 21 = 39, vertically centered
            (crate::constants::FIELD_LINE + 21.0, 260.0)
        } else {
            compute_v6_preferred_xy(&params[i], own_goal_x)
        };

        // Side band — vertical stripe at side.min/max y
        svg.push_str(&format!(r##"<line x1="{:.0}" y1="{:.0}" x2="{:.0}" y2="{:.0}" stroke="{}" stroke-width="1" stroke-opacity="0.22" stroke-dasharray="2,3"/>"##,
            px, s.side.min, px, s.side.max, colors[i]));

        // Ball-preferred halo (lightly drawn around preferred position)
        let ball_r = s.ball.preferred.clamp(20.0, 200.0);
        svg.push_str(&format!(r##"<circle cx="{:.0}" cy="{:.0}" r="{:.0}" fill="{}" fill-opacity="0.05" stroke="{}" stroke-width="1" stroke-opacity="0.25"/>"##,
            px, py, ball_r, colors[i], colors[i]));

        // Opponent-preferred (mark/space) ring
        let opp_r = s.opponent.preferred.clamp(15.0, 200.0);
        svg.push_str(&format!(r##"<circle cx="{:.0}" cy="{:.0}" r="{:.0}" fill="none" stroke="{}" stroke-width="2" stroke-opacity="0.5" stroke-dasharray="6,3"/>"##,
            px, py, opp_r, colors[i]));

        // Player dot + label
        svg.push_str(&format!(r##"<circle cx="{:.0}" cy="{:.0}" r="13" fill="{}" stroke="white" stroke-width="2.5"/>"##,
            px, py, colors[i]));
        svg.push_str(&format!(r##"<text x="{:.0}" y="{:.0}" fill="white" font-size="10" font-weight="bold" text-anchor="middle">{}</text>"##,
            px, py + 3.5, slots[i]));
    }

    // Header
    svg.push_str(&format!(r##"<text x="10" y="22" fill="rgba(255,255,255,0.95)" font-size="16" font-weight="bold">{}</text>"##, team_name));
    svg.push_str(&format!(r##"<text x="10" y="38" fill="rgba(255,255,255,0.7)" font-size="11">{}</text>"##, team_desc));
    svg.push_str(r##"<text x="870" y="22" fill="rgba(255,255,255,0.55)" font-size="9" text-anchor="end">solid: preferred · faint: own_goal min/max · dashed ring: opponent pref</text>"##);

    // Legend table below pitch
    svg.push_str(r##"<rect y="520" width="880" height="140" fill="#0a0a0a"/>"##);
    svg.push_str(r##"<text x="10" y="538" fill="white" font-size="11" font-weight="bold">Spatial preferences (min / preferred / max)</text>"##);
    svg.push_str(r##"<text x="10" y="556" fill="rgba(255,255,255,0.55)" font-size="9">slot     own_goal              side                 ball                 teammate           opponent</text>"##);
    let header_y = 575.0;
    for i in 0..5 {
        let s = &params[i].spatial;
        let y = header_y + (i as f32) * 14.0;
        let txt = format!(
            "{:<6}  {:>4.0}/{:>4.0}/{:>4.0}     {:>4.0}/{:>4.0}/{:>4.0}      {:>4.0}/{:>4.0}/{:>4.0}      {:>4.0}/{:>4.0}/{:>4.0}    {:>4.0}/{:>4.0}/{:>4.0}",
            slots[i],
            s.own_goal.min, s.own_goal.preferred, s.own_goal.max,
            s.side.min, s.side.preferred, s.side.max,
            s.ball.min, s.ball.preferred, s.ball.max,
            s.teammate.min, s.teammate.preferred, s.teammate.max,
            s.opponent.min, s.opponent.preferred, s.opponent.max,
        );
        svg.push_str(&format!(r##"<text x="10" y="{:.0}" fill="{}" font-size="10" xml:space="preserve">{}</text>"##,
            y, colors[i], txt));
    }
    svg.push_str("</svg>");
    let _ = std::fs::write(out_path, svg);
}

fn regenerate_all_team_svgs(project_root: &Path) {
    let teams_dir = project_root.join("data").join("teams");
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(&teams_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() { continue; }
            let baseline = path.join("baseline.json");
            if !baseline.exists() { continue; }
            let team_name = path.file_name().unwrap().to_string_lossy().into_owned();
            let baseline_file = match read_team_baseline_v6(&baseline) {
                Ok(b) => b, Err(e) => { eprintln!("  ! {}: {}", team_name, e); continue; }
            };
            let display = baseline_file.description.clone().unwrap_or_else(|| team_name.clone());
            write_team_layout_svg(&path.join("layout.svg"), &team_name, &display, &baseline_file.player_params);
            println!("  ✓ {} → {}/layout.svg", team_name, path.display());
            count += 1;
        }
    }
    println!("Regenerated {} team layout SVGs", count);
}

fn run_v6_team_train(project_root: &Path, team_name: &str, variant: AnnealVariant) {
    let team_dir = project_root.join("data").join("teams").join(team_name);
    if !team_dir.exists() {
        eprintln!("Team {} does not exist in data/teams/. Train it first.", team_name);
        std::process::exit(1);
    }
    let baseline_path = team_dir.join("baseline.json");
    let baseline_file = match read_team_baseline_v6(&baseline_path) {
        Ok(b) => b, Err(e) => { eprintln!("Cannot read baseline: {}", e); std::process::exit(1); }
    };
    let initial = baseline_file.player_params;
    let stages: &[(usize, usize)] = match variant {
        AnnealVariant::Quick => ANNEAL_STAGES_QUICK,
        AnnealVariant::Short => ANNEAL_STAGES_SHORT,
        AnnealVariant::Full  => ANNEAL_STAGES_FULL,
    };
    let label = match variant { AnnealVariant::Quick => "QUICK", AnnealVariant::Short => "SHORT", AnnealVariant::Full => "FULL" };

    // Find next session number to avoid stage-name collisions
    let mut session_num = 2;
    let sessions_dir = team_dir.join("sessions");
    while sessions_dir.join(format!("s{}-anneal-stage-1-{}ep-{}g", session_num, stages[0].0, stages[0].1)).exists() {
        session_num += 1;
    }
    let session_prefix = format!("s{}-", session_num);

    let team_desc_idx = TEAM_NAMES.iter().position(|n| *n == team_name);
    let team_desc = team_desc_idx.and_then(|i| TEAM_DESCRIPTIONS.get(i)).copied().unwrap_or(team_name);

    println!("=== V6 TEAM CONTINUATION TRAINING ===");
    println!("Team: {}", team_name);
    println!("Variant: {} stages={:?}", label, stages);
    println!("Session prefix: {}", session_prefix);
    println!();

    let final_team = run_team_anneal_with_prefix(&team_dir, team_name, initial, stages, &session_prefix, false, None);

    // Persist updated baseline + info + svg
    let final_doc = serde_json::json!({
        "name": team_name, "version": session_num,
        "type": "team-policy-v6",
        "description": format!("{}: champion after {} adaptive anneal sessions", team_name, session_num - 1),
        "playerParams": final_team,
        "trainedAt": iso_now(),
        "trainingMethod": "population-anneal-continuation",
    });
    let _ = std::fs::write(&baseline_path,
        format!("{}\n", serde_json::to_string_pretty(&final_doc).unwrap()));
    write_team_info_md(&team_dir, team_name, team_desc, &final_team);
    write_team_layout_svg(&team_dir.join("layout.svg"), team_name, team_desc, &final_team);
    println!("\nUpdated baseline + info.md + layout.svg for {}", team_name);
}

/// GK-only training across all teams in data/teams/.
/// Outfield slots (0-3) are frozen; only slot 4 (GK) is mutated.
/// Each team trains independently, evaluation is the team vs itself with
/// only the GK slot swapped.
fn run_gk_train_all(project_root: &Path, epochs: usize, games_per_epoch: usize) {
    let teams_dir = project_root.join("data").join("teams");

    // Load all team baselines
    let mut teams: Vec<(String, std::path::PathBuf, TeamPolicyV6)> = vec![];
    if let Ok(entries) = std::fs::read_dir(&teams_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() { continue; }
            let baseline = path.join("baseline.json");
            if !baseline.exists() { continue; }
            let team_name = path.file_name().unwrap().to_string_lossy().into_owned();
            match read_team_baseline_v6(&baseline) {
                Ok(b) => {
                    let mut params = b.player_params;
                    // Old baselines lack gk params — bootstrap defaults for slot 4.
                    if params[4].gk.is_none() {
                        params[4].gk = Some(policy::GkDecisionParams::default());
                    }
                    teams.push((team_name, path, params));
                }
                Err(e) => eprintln!("  ! skip {}: {}", team_name, e),
            }
        }
    }
    teams.sort_by(|a, b| a.0.cmp(&b.0));

    if teams.is_empty() {
        eprintln!("No teams found in data/teams/. Run --v6-population first.");
        std::process::exit(1);
    }

    crate::game::CLUSTER_START.store(true, std::sync::atomic::Ordering::Relaxed);

    println!("=== GK-ONLY TRAINING: {} teams × {} epochs × {} games ===",
        teams.len(), epochs, games_per_epoch);
    println!("Only slot 4 (GK params: diveChance, diveCommitDist, riskClearance,");
    println!("  distributionZone, passTargetDist) is mutated. Outfield frozen.");
    println!();

    let total_start = std::time::Instant::now();

    for (team_name, team_dir, initial) in &teams {
        let mut champion = *initial;
        let mut scale_factor: f32 = 1.0;
        let mut rejection_streak: usize = 0;
        let mut accepted_count = 0usize;
        let team_start = std::time::Instant::now();

        println!("[{}] starting GK anneal ({} epochs × {} games)", team_name, epochs, games_per_epoch);

        for epoch in 1..=epochs {
            let opponent = champion;
            let mut rng = rand::thread_rng();
            let candidate = mutate_gk_only(&champion, &mut rng, scale_factor);
            let eval = evaluate_team_policies_v6(&opponent, &candidate, games_per_epoch);

            let stop_label = match eval.early_stop {
                Some(crate::trainer::EarlyStop::Worse)      => "worse",
                Some(crate::trainer::EarlyStop::Better)     => "better",
                Some(crate::trainer::EarlyStop::Indecisive) => "indecsv",
                None => "full",
            };
            // GK-only training: accept on goal_diff > 0 (simple majority).
            // The standard point_z_score > 1.0 criterion needs too many games
            // to detect the small effect of a single GK param change.
            let candidate_won = eval.goal_diff > 0.0;
            if candidate_won {
                champion = candidate;
                accepted_count += 1;
                rejection_streak = 0;
                scale_factor = (scale_factor * 1.5).min(1.0);
            } else {
                rejection_streak += 1;
                if rejection_streak % 20 == 0 {
                    scale_factor = (scale_factor * 0.75).max(0.1);
                }
            }

            if epoch <= 5 || epoch % 10 == 0 || epoch == epochs {
                let gk = champion[4].gk.unwrap_or_default();
                println!("  epoch {:>4}/{} | {} games z={:.2} [{}] won={} | accepted {} | scale {:.2} | gk: dive={:.2} dist={:.0} risk={:.2} zone={:.2} pass={:.0}",
                    epoch, epochs, eval.games, eval.z_score, stop_label, candidate_won,
                    accepted_count, scale_factor,
                    gk.gk_dive_chance, gk.gk_dive_commit_dist,
                    gk.gk_risk_clearance, gk.gk_distribution_zone,
                    gk.gk_pass_target_dist);
            }
        }

        // Persist updated baseline
        let baseline_path = team_dir.join("baseline.json");
        let existing = read_team_baseline_v6(&baseline_path).ok();
        let name = existing.as_ref().and_then(|b| b.name.clone()).unwrap_or_else(|| team_name.clone());
        let version = existing.as_ref().and_then(|b| b.version).unwrap_or(1) + 1;
        let doc = serde_json::json!({
            "name": name, "version": version,
            "type": "team-policy-v6",
            "description": format!("{}: GK-only training, {} epochs", team_name, epochs),
            "playerParams": champion,
            "trainedAt": iso_now(),
            "trainingMethod": "gk-only-anneal",
        });
        let _ = std::fs::write(&baseline_path,
            format!("{}\n", serde_json::to_string_pretty(&doc).unwrap()));

        let team_desc_idx = TEAM_NAMES.iter().position(|n| *n == team_name.as_str());
        let team_desc = team_desc_idx.and_then(|i| TEAM_DESCRIPTIONS.get(i)).copied().unwrap_or(team_name.as_str());
        write_team_info_md(team_dir, team_name, team_desc, &champion);
        write_team_layout_svg(&team_dir.join("layout.svg"), team_name, team_desc, &champion);

        println!("  -> done in {:.1}s, {} accepted, baseline saved\n",
            team_start.elapsed().as_secs_f32(), accepted_count);
    }

    println!("=== GK training complete: {} teams in {:.1}s ===",
        teams.len(), total_start.elapsed().as_secs_f32());
}

