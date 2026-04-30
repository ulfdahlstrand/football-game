mod constants;
mod game;
mod policy;
mod ai;
mod physics;
mod trainer;
mod session;
mod svg;

use std::path::Path;
use std::time::SystemTime;

use policy::{mutate, PolicyParams};
use session::{ensure_genesis, read_baseline, update_baseline, EpochSummary, SessionWriter};
use trainer::{evaluate_policies, play_match, EarlyStop};
use svg::{write_training_svg, write_progress_svg, write_matrix_svg, SessionProgress, MatrixCell};
use rayon::prelude::*;

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

fn run_matrix(policies_dir: &Path, games_per_pair: usize) {
    let sessions_dir = policies_dir.join("sessions");
    let dirs = list_sessions(&sessions_dir);

    // Load each session's best champion
    let mut sessions: Vec<(String, PolicyParams)> = Vec::new();
    for name in &dirs {
        let best_path = sessions_dir.join(name).join("best.json");
        if !best_path.exists() { continue; }
        if let Ok(text) = std::fs::read_to_string(&best_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Ok(params) = serde_json::from_value::<PolicyParams>(json["parameters"].clone()) {
                    sessions.push((name.clone(), params));
                }
            }
        }
    }

    let n = sessions.len();
    if n == 0 {
        println!("No session champions found.");
        return;
    }

    let total_matches = (n * n * games_per_pair) as u64;
    println!("Matrix: {} sessions x {} sessions x {} games = {} matches", n, n, games_per_pair, total_matches);
    println!("Using {} CPU threads via rayon", rayon::current_num_threads());

    let start = std::time::Instant::now();
    let mut matrix: Vec<Vec<MatrixCell>> = vec![vec![MatrixCell::default(); n]; n];

    for i in 0..n {
        let row_start = std::time::Instant::now();
        for j in 0..n {
            let p0 = sessions[i].1;
            let p1 = sessions[j].1;
            let seed_base = rand::random::<u64>();

            let scores: Vec<(u32, u32)> = (0..games_per_pair).into_par_iter()
                .map(|k| {
                    let seed = seed_base.wrapping_add(k as u64);
                    let (s0, s1, _) = play_match(&p0, &p1, seed);
                    (s0, s1)
                })
                .collect();

            let mut cell = MatrixCell { games: games_per_pair as u32, ..Default::default() };
            for (s0, s1) in scores {
                cell.team0_goals += s0 as u64;
                cell.team1_goals += s1 as u64;
                if s0 > s1 { cell.team0_wins += 1; }
                else if s1 > s0 { cell.team1_wins += 1; }
                else { cell.draws += 1; }
            }
            matrix[i][j] = cell;
        }
        let row_ms = row_start.elapsed().as_millis();
        println!("Row {:>2}/{}: {} ({}ms)", i+1, n, &sessions[i].0, row_ms);
    }

    let elapsed = start.elapsed();
    println!("Matrix complete in {:.1}s", elapsed.as_secs_f64());

    // Compute total training stats
    let (training_matches, training_minutes) = compute_total_stats(&sessions_dir);
    let grand_total_matches = training_matches + total_matches;

    // Write JSON results
    let json_path = sessions_dir.join("matrix.json");
    let names: Vec<&str> = sessions.iter().map(|(n, _)| n.as_str()).collect();
    let json_rows: Vec<serde_json::Value> = matrix.iter().enumerate().map(|(i, row)| {
        let cells: Vec<serde_json::Value> = row.iter().enumerate().map(|(j, c)| {
            serde_json::json!({
                "vs": names[j],
                "team0Wins": c.team0_wins,
                "team1Wins": c.team1_wins,
                "draws": c.draws,
                "team0Goals": c.team0_goals,
                "team1Goals": c.team1_goals,
                "winPct": (c.team0_wins as f64 + c.draws as f64 * 0.5) / c.games as f64,
                "goalDiffPerGame": (c.team0_goals as f64 - c.team1_goals as f64) / c.games as f64,
            })
        }).collect();
        serde_json::json!({ "session": names[i], "row": cells })
    }).collect();
    let json_doc = serde_json::json!({
        "sessions": names,
        "gamesPerPair": games_per_pair,
        "totalMatrixMatches": total_matches,
        "totalTrainingMatches": training_matches,
        "totalTrainingMinutes": training_minutes,
        "grandTotalMatches": grand_total_matches,
        "elapsedSec": elapsed.as_secs_f64(),
        "matrix": json_rows,
    });
    if let Some(parent) = json_path.parent() { let _ = std::fs::create_dir_all(parent); }
    let _ = std::fs::write(&json_path, format!("{}\n", serde_json::to_string_pretty(&json_doc).unwrap()));

    // Write SVG
    let svg_path = sessions_dir.join("matrix.svg");
    write_matrix_svg(&svg_path, &names, &matrix, grand_total_matches, training_minutes);

    println!("\nWritten:");
    println!("  {}", json_path.display());
    println!("  {}", svg_path.display());
}

fn build_opponents_index(policies_dir: &Path) {
    let mut opponents: Vec<serde_json::Value> = Vec::new();

    if policies_dir.join("baseline.json").exists() {
        opponents.push(serde_json::json!({
            "name": "baseline",
            "label": "Baseline (current champion)",
            "file": "data/policies/baseline.json"
        }));
    }
    if policies_dir.join("baseline-genesis.json").exists() {
        opponents.push(serde_json::json!({
            "name": "genesis",
            "label": "Genesis (hand-tuned original)",
            "file": "data/policies/baseline-genesis.json"
        }));
    }

    let sessions_dir = policies_dir.join("sessions");
    let dirs = list_sessions(&sessions_dir);
    for name in &dirs {
        let best_path = sessions_dir.join(name).join("best.json");
        if !best_path.exists() { continue; }
        opponents.push(serde_json::json!({
            "name": format!("{}-best", name),
            "label": format!("{} champion", name),
            "file": format!("data/policies/sessions/{}/best.json", name)
        }));
    }

    let count = opponents.len();
    let doc = serde_json::json!({ "opponents": opponents });
    let out_path = policies_dir.join("opponents.json");
    let _ = std::fs::write(&out_path, format!("{}\n", serde_json::to_string_pretty(&doc).unwrap()));
    println!("Wrote {} ({} opponents)", out_path.display(), count);
}

fn regenerate_progress_svg(policies_dir: &Path, baseline_path: &Path) {
    let sessions_dir = policies_dir.join("sessions");
    let progress_path = sessions_dir.join("progress.svg");

    let mut history_map: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    if let Ok(baseline_raw) = std::fs::read_to_string(baseline_path) {
        if let Ok(baseline_val) = serde_json::from_str::<serde_json::Value>(&baseline_raw) {
            if let Some(hist) = baseline_val["history"].as_array() {
                for e in hist {
                    if let (Some(s), Some(d)) = (e["session"].as_str(), e["goalDiff"].as_f64()) {
                        history_map.insert(s.to_string(), d);
                    }
                }
            }
        }
    }

    let mut session_dirs: Vec<String> = std::fs::read_dir(&sessions_dir)
        .ok()
        .into_iter()
        .flat_map(|rd| rd.filter_map(|entry| {
            let entry = entry.ok()?;
            if !entry.file_type().ok()?.is_dir() { return None; }
            entry.file_name().into_string().ok()
        }))
        .collect();
    // Sort numerically when names have a trailing number ("session-10" after "session-2"),
    // fall back to alphabetical for other formats.
    session_dirs.sort_by(|a, b| {
        let key = |s: &str| -> (Option<u64>, String) {
            let trailing_num: String = s.chars().rev()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>().chars().rev().collect();
            let n = trailing_num.parse::<u64>().ok();
            let prefix = s[..s.len() - trailing_num.len()].to_string();
            (n, prefix)
        };
        let (na, pa) = key(a);
        let (nb, pb) = key(b);
        pa.cmp(&pb).then_with(|| match (na, nb) {
            (Some(x), Some(y)) => x.cmp(&y),
            _ => a.cmp(b),
        })
    });

    let sessions: Vec<SessionProgress> = session_dirs.into_iter()
        .map(|name| {
            let improved = history_map.contains_key(&name);
            let goal_diff = history_map.get(&name).copied().unwrap_or(0.0);
            SessionProgress { session: name, goal_diff, improved }
        })
        .collect();

    if sessions.is_empty() {
        println!("No sessions found.");
        return;
    }
    write_progress_svg(&progress_path, &sessions);
    let improved = sessions.iter().filter(|s| s.improved).count();
    println!("Wrote {} ({}/{} improved)", progress_path.display(), improved, sessions.len());
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
    let policies_dir = project_root.join("data").join("policies");
    let baseline_path = policies_dir.join("baseline.json");

    // Regen-only mode: just rebuild progress.svg from existing session folders + baseline history
    if args.get(1).map(|s| s.as_str()) == Some("--regen-progress") {
        regenerate_progress_svg(&policies_dir, &baseline_path);
        return;
    }

    // Round-robin matrix: every session champion vs every other
    if args.get(1).map(|s| s.as_str()) == Some("--matrix") {
        let games: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1000);
        run_matrix(&policies_dir, games);
        return;
    }

    // Build opponents.json index for in-game opponent selector
    if args.get(1).map(|s| s.as_str()) == Some("--build-opponents") {
        build_opponents_index(&policies_dir);
        return;
    }

    let epochs: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(100);
    let games_per_epoch: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1000);
    let session_name: &str = args.get(3).map(|s| s.as_str()).unwrap_or("session-1");

    let baseline_file = match read_baseline(&baseline_path) {
        Ok(b) => b,
        Err(e) => { eprintln!("Error reading baseline: {}", e); std::process::exit(1); }
    };

    let initial_params = baseline_file.parameters;
    let mut champion_params = initial_params;
    let mut champion_epoch: usize = 0;
    let session_started = iso_now();

    ensure_genesis(&baseline_path, &baseline_file);

    let training_start = std::time::Instant::now();

    let writer = match SessionWriter::new(&policies_dir, session_name) {
        Ok(w) => w,
        Err(e) => { eprintln!("Error creating session dir: {}", e); std::process::exit(1); }
    };

    if let Err(e) = writer.write_initial_baseline(&initial_params, &session_started) {
        eprintln!("Warning: could not write initial baseline: {}", e);
    }

    println!(
        "Training session '{}': {} epochs x {} games/epoch",
        session_name, epochs, games_per_epoch
    );
    println!("Using {} CPU threads via rayon", rayon::current_num_threads());

    let mut history: Vec<EpochSummary> = Vec::with_capacity(epochs);
    // Adaptive mutation scale: decays when stuck, recovers on acceptance.
    let mut scale_factor: f32 = 1.0;
    let mut rejection_streak: usize = 0;
    const SCALE_DECAY_EVERY: usize = 20;  // reduce scale after this many consecutive rejections
    const SCALE_DECAY_FACTOR: f32 = 0.75;
    const SCALE_FLOOR: f32 = 0.1;
    const SCALE_RECOVER_FACTOR: f32 = 1.5;

    for epoch in 1..=epochs {
        let opponent_epoch = champion_epoch;
        let opponent_params = champion_params;
        let mut rng = rand::thread_rng();
        let candidate_params = mutate(&champion_params, &mut rng, scale_factor);

        let eval = evaluate_policies(&opponent_params, &candidate_params, games_per_epoch);
        let accepted = eval.candidate_won;
        if accepted {
            champion_params = candidate_params;
            champion_epoch = epoch;
            rejection_streak = 0;
            scale_factor = (scale_factor * SCALE_RECOVER_FACTOR).min(1.0);
        } else {
            rejection_streak += 1;
            if rejection_streak % SCALE_DECAY_EVERY == 0 {
                scale_factor = (scale_factor * SCALE_DECAY_FACTOR).max(SCALE_FLOOR);
                println!("  [scale reduced to {:.3} after {} consecutive rejections]", scale_factor, rejection_streak);
            }
        }

        let current_champion_epoch = if accepted { epoch } else { champion_epoch };
        let current_champion_params = if accepted { candidate_params } else { champion_params };

        let early_label = eval.early_stop.map(|s| match s {
            EarlyStop::Worse => "worse".to_string(),
            EarlyStop::Better => "better".to_string(),
        });

        let stop_str = eval.early_stop.map(|s| match s {
            EarlyStop::Worse => " [EARLY STOP: worse]",
            EarlyStop::Better => " [EARLY STOP: better]",
        }).unwrap_or("");

        println!(
            "epoch-{:03} {} diff={:+.3} z={:.2} games={}/{} scale={:.3} champion={}{}",
            epoch,
            if accepted { "ACCEPTED" } else { "rejected" },
            eval.goal_diff,
            eval.z_score,
            eval.games,
            games_per_epoch,
            scale_factor,
            current_champion_epoch,
            stop_str,
        );

        if let Err(e) = writer.write_epoch(
            epoch, opponent_epoch, &opponent_params, &candidate_params,
            accepted, current_champion_epoch, &current_champion_params,
            &eval, &iso_now(), games_per_epoch,
        ) {
            eprintln!("Warning: could not write epoch {}: {}", epoch, e);
        }

        history.push(EpochSummary {
            epoch,
            accepted,
            champion_epoch: current_champion_epoch,
            goal_diff: eval.goal_diff,
            baseline_avg_goals: eval.baseline_avg_goals,
            candidate_avg_goals: eval.candidate_avg_goals,
            elapsed_ms: eval.elapsed_ms,
            early_stop: early_label,
            z_score: eval.z_score,
            games_run: eval.games,
        });
    }

    let finished_at = iso_now();

    if let Err(e) = writer.write_summary(
        &session_started, &finished_at, epochs, games_per_epoch,
        champion_epoch, &champion_params, &history,
    ) {
        eprintln!("Warning: could not write summary: {}", e);
    }

    if let Err(e) = writer.write_best(champion_epoch, &champion_params, session_name) {
        eprintln!("Warning: could not write best.json: {}", e);
    }

    write_training_svg(
        &writer.session_dir().join("training-progress.svg"),
        &history,
        champion_epoch,
    );

    // Final evaluation: update baseline only if champion beats session-start baseline
    if champion_epoch > 0 {
        println!(
            "\nEvaluating final champion against session baseline ({} games)...",
            games_per_epoch
        );
        let final_eval = evaluate_policies(&initial_params, &champion_params, games_per_epoch);
        println!(
            "vs session baseline: champion={:.3} baseline={:.3} diff={:+.3} z={:.2}",
            final_eval.candidate_avg_goals,
            final_eval.baseline_avg_goals,
            final_eval.goal_diff,
            final_eval.z_score,
        );

        // Also measure against genesis so we can see total improvement
        let genesis_path = policies_dir.join("baseline-genesis.json");
        if let Ok(genesis_file) = read_baseline(&genesis_path) {
            let genesis_eval = evaluate_policies(&genesis_file.parameters, &champion_params, games_per_epoch);
            println!(
                "vs genesis:          champion={:.3} genesis={:.3}  diff={:+.3} z={:.2}",
                genesis_eval.candidate_avg_goals,
                genesis_eval.baseline_avg_goals,
                genesis_eval.goal_diff,
                genesis_eval.z_score,
            );
        }

        if final_eval.candidate_won {
            match update_baseline(
                &baseline_path, &baseline_file, &champion_params,
                session_name, champion_epoch, final_eval.goal_diff, &iso_now(),
            ) {
                Ok(_) => println!(
                    "baseline.json updated — version incremented, history appended (epoch {} diff={:+.3})",
                    champion_epoch, final_eval.goal_diff
                ),
                Err(e) => eprintln!("Warning: could not update baseline: {}", e),
            }
        } else {
            println!("Champion did not beat session baseline — baseline.json unchanged.");
        }
    } else {
        println!("\nNo improvement found this session — baseline.json unchanged.");
    }

    // Print total training time
    let elapsed = training_start.elapsed();
    let total_secs = elapsed.as_secs();
    let h = total_secs / 3600;
    let m = (total_secs % 3600) / 60;
    let s = total_secs % 60;
    if h > 0 {
        println!("\nTotal training time: {}h {}m {}s", h, m, s);
    } else {
        println!("\nTotal training time: {}m {}s", m, s);
    }

    regenerate_progress_svg(&policies_dir, &baseline_path);

    println!("Done. Champion epoch: {}", champion_epoch);
}
