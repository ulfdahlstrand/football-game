use std::path::Path;

use training_engine::policy::{self, mutate_gk_only, mutate_team_v6, TeamPolicyV6};
use training_engine::session::{
    ensure_team_v6_genesis, read_team_baseline_v6, update_team_v6_baseline,
    EpochSummary, SessionWriter,
};
use training_engine::trainer::{evaluate_team_policies_v6, EarlyStop};

use super::anneal::{
    run_team_anneal, run_team_anneal_with_prefix, random_v6_team,
    variant_label, variant_stages, AnnealVariant,
};
use training_render::write_team_layout_svg;
use super::teams::{lookup_team_desc, write_team_info_md, TEAM_DESCRIPTIONS, TEAM_NAMES};
use super::util::{iso_now, save_team_artifacts, v6_diff_slots, write_json_pretty};

pub fn run_v6_training(policies_dir: &Path, epochs: usize, games_per_epoch: usize, session_name: &str) {
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
            EarlyStop::Worse => "worse".to_string(),
            EarlyStop::Better => "better".to_string(),
            EarlyStop::Indecisive => "indecisive".to_string(),
        });
        let stop_str = eval.early_stop.map(|s| match s {
            EarlyStop::Worse => " [EARLY STOP: worse]",
            EarlyStop::Better => " [EARLY STOP: better]",
            EarlyStop::Indecisive => " [INDECISIVE: futile]",
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
    training_render::write_training_svg(
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

pub fn run_v6_population(project_root: &Path, num_teams: usize, variant: AnnealVariant, skip: usize) {
    let teams_dir = project_root.join("data").join("teams");
    std::fs::create_dir_all(&teams_dir).expect("create teams dir");
    let stages = variant_stages(variant);

    let start_idx = skip.min(TEAM_NAMES.len());
    let end_idx = (start_idx + num_teams).min(TEAM_NAMES.len());
    let n = end_idx - start_idx;

    println!("=== V6 POPULATION TRAINING ===");
    println!("Teams: {} (indices {}..{})", n, start_idx, end_idx);
    println!("Stages: {:?} ({})", stages, variant_label(variant));
    println!("Per-team folder: data/teams/{{name}}/");
    println!();

    let pop_start = std::time::Instant::now();
    for (local_idx, i) in (start_idx..end_idx).enumerate() {
        let team_name = TEAM_NAMES[i];
        let team_desc = TEAM_DESCRIPTIONS[i];
        let team_dir = teams_dir.join(team_name);

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

pub fn run_v6_team_train(project_root: &Path, team_name: &str, variant: AnnealVariant) {
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
    let stages = variant_stages(variant);

    let mut session_num = 2;
    let sessions_dir = team_dir.join("sessions");
    while sessions_dir.join(format!("s{}-anneal-stage-1-{}ep-{}g", session_num, stages[0].0, stages[0].1)).exists() {
        session_num += 1;
    }
    let session_prefix = format!("s{}-", session_num);

    println!("=== V6 TEAM CONTINUATION TRAINING ===");
    println!("Team: {}", team_name);
    println!("Variant: {} stages={:?}", variant_label(variant), stages);
    println!("Session prefix: {}", session_prefix);
    println!();

    let final_team = run_team_anneal_with_prefix(
        &team_dir, team_name, initial, stages, &session_prefix, false, None,
    );

    save_team_artifacts(
        &team_dir, team_name, &final_team,
        &format!("{}: champion after {} adaptive anneal sessions", team_name, session_num - 1),
        "population-anneal-continuation",
        session_num as u64,
    );
    println!("\nUpdated baseline + info.md + layout.svg for {}", team_name);
}

/// GK-only training across all teams in data/teams/.
pub fn run_gk_train_all(project_root: &Path, epochs: usize, games_per_epoch: usize) {
    let teams_dir = project_root.join("data").join("teams");

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

    training_engine::game::CLUSTER_START.store(true, std::sync::atomic::Ordering::Relaxed);

    println!("=== GK-ONLY TRAINING: {} teams × {} epochs × {} games ===",
        teams.len(), epochs, games_per_epoch);
    println!("Only slot 4 (GK params) is mutated. Outfield frozen.");
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
                Some(EarlyStop::Worse)      => "worse",
                Some(EarlyStop::Better)     => "better",
                Some(EarlyStop::Indecisive) => "indecsv",
                None => "full",
            };
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
        write_json_pretty(&baseline_path, &doc);

        let team_desc = lookup_team_desc(team_name);
        write_team_info_md(team_dir, team_name, team_desc, &champion);
        write_team_layout_svg(&team_dir.join("layout.svg"), team_name, team_desc, &champion);

        println!("  -> done in {:.1}s, {} accepted, baseline saved\n",
            team_start.elapsed().as_secs_f32(), accepted_count);
    }

    println!("=== GK training complete: {} teams in {:.1}s ===",
        teams.len(), total_start.elapsed().as_secs_f32());
}
