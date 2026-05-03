mod constants;
mod game;
mod policy;
mod spatial;
mod brain;
mod ai;
mod physics;
mod trainer;
mod session;
mod svg;

use std::path::Path;
use std::time::SystemTime;

use policy::{mutate, mutate_team, mutate_team_v3, mutate_team_v4,
              PolicyParams, TeamPolicy, TeamPolicyV3, TeamPolicyV4, V3Params, V4Params};
use session::{
    ensure_genesis, ensure_team_genesis, ensure_team_v3_genesis, ensure_team_v4_genesis,
    read_baseline, read_team_baseline, read_team_baseline_v3, read_team_baseline_v4,
    update_baseline, update_team_baseline, update_team_v3_baseline, update_team_v4_baseline,
    EpochSummary, SessionWriter,
};
use trainer::{evaluate_policies, evaluate_team_policies, evaluate_team_policies_v3,
              evaluate_team_policies_v4, play_match, EarlyStop};
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
            let p_i = sessions[i].1;
            let p_j = sessions[j].1;
            let seed_base = rand::random::<u64>();

            // Side-swap: half games with i as team 0 (home), half with i as team 1 (away).
            // Removes positional home advantage so result reflects true relative strength.
            // From row team i's perspective: scores = (i_goals, j_goals) regardless of side.
            let scores: Vec<(u32, u32)> = (0..games_per_pair).into_par_iter()
                .map(|k| {
                    let seed = seed_base.wrapping_add(k as u64);
                    if k % 2 == 0 {
                        // i is home (team 0), j is away (team 1)
                        let (s0, s1, _) = play_match(&p_i, &p_j, seed);
                        (s0, s1)
                    } else {
                        // j is home (team 0), i is away (team 1) — swap perspective
                        let (s0, s1, _) = play_match(&p_j, &p_i, seed);
                        (s1, s0)
                    }
                })
                .collect();

            let mut cell = MatrixCell { games: games_per_pair as u32, ..Default::default() };
            for (i_goals, j_goals) in scores {
                cell.team0_goals += i_goals as u64;
                cell.team1_goals += j_goals as u64;
                if i_goals > j_goals { cell.team0_wins += 1; }
                else if j_goals > i_goals { cell.team1_wins += 1; }
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

/// Scans v1, v2 and v3 policy folders and writes a single combined
/// data/policies/opponents.json with all available champions. Each entry
/// carries its `version` (v1/v2/v3) so the in-game loader can handle the
/// different policy file formats correctly.
fn build_opponents_index(policies_root: &Path) {
    let mut opponents: Vec<serde_json::Value> = Vec::new();

    // Helper: add opponents from one version directory.
    fn add_version(out: &mut Vec<serde_json::Value>, root: &Path, version: &str) {
        let dir_name = version;
        let version_dir = root.join(dir_name);
        if !version_dir.exists() { return; }

        let baseline = version_dir.join("baseline.json");
        if baseline.exists() {
            out.push(serde_json::json!({
                "name": format!("{}-baseline", version),
                "label": format!("{} baseline (current champion)", version),
                "version": version,
                "file": format!("data/policies/{}/baseline.json", dir_name),
            }));
        }
        let genesis = version_dir.join("baseline-genesis.json");
        if genesis.exists() {
            out.push(serde_json::json!({
                "name": format!("{}-genesis", version),
                "label": format!("{} genesis", version),
                "version": version,
                "file": format!("data/policies/{}/baseline-genesis.json", dir_name),
            }));
        }

        let sessions_dir = version_dir.join("sessions");
        let dirs = list_sessions(&sessions_dir);
        for name in &dirs {
            let best_path = sessions_dir.join(name).join("best.json");
            if !best_path.exists() { continue; }
            out.push(serde_json::json!({
                "name": format!("{}-{}-best", version, name),
                "label": format!("{} {} champion", version, name),
                "version": version,
                "file": format!("data/policies/{}/sessions/{}/best.json", dir_name, name),
            }));
        }
    }

    add_version(&mut opponents, policies_root, "v1");
    add_version(&mut opponents, policies_root, "v2");
    add_version(&mut opponents, policies_root, "v3");
    add_version(&mut opponents, policies_root, "v4");

    let count = opponents.len();
    let doc = serde_json::json!({ "opponents": opponents });
    let out_path = policies_root.join("opponents.json");
    let _ = std::fs::write(&out_path, format!("{}\n", serde_json::to_string_pretty(&doc).unwrap()));
    println!("Wrote {} ({} opponents across v1/v2/v3/v4)", out_path.display(), count);
}

/// v2-style matrix that ALSO includes v1 baseline (and v2 baseline) as
/// reference rows/columns, plus every v2 session champion. Each entry is
/// represented as 5 brains; pairs are played side-swapped.
fn run_matrix_v2(policies_root: &Path, games_per_pair: usize) {
    use brain::PlayerBrain;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;
    use crate::game::Game;
    use crate::physics::step_game;

    let v1_dir = policies_root.join("v1");
    let v2_dir = policies_root.join("v2");
    let v3_dir = policies_root.join("v3");
    let v2_sessions_dir = v2_dir.join("sessions");

    // Each entry: (display_name, [PlayerBrain; 5])
    let mut entries: Vec<(String, [PlayerBrain; 5])> = Vec::new();

    // v1: baseline + every session champion
    if let Ok(v1) = session::read_baseline(&v1_dir.join("baseline.json")) {
        entries.push(("v1-baseline".to_string(), [PlayerBrain::V1(v1.parameters); 5]));
    }
    {
        let v1_sessions_dir = v1_dir.join("sessions");
        for name in list_sessions(&v1_sessions_dir) {
            let best_path = v1_sessions_dir.join(&name).join("best.json");
            if !best_path.exists() { continue; }
            if let Ok(text) = std::fs::read_to_string(&best_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Ok(p) = serde_json::from_value::<PolicyParams>(json["parameters"].clone()) {
                        entries.push((format!("v1-{}", name), [PlayerBrain::V1(p); 5]));
                    }
                }
            }
        }
    }

    // v2: baseline + every session champion
    if let Ok(v2) = session::read_team_baseline(&v2_dir.join("baseline.json")) {
        let mut bs = [PlayerBrain::default(); 5];
        for i in 0..5 { bs[i] = PlayerBrain::V2(v2.player_params[i]); }
        entries.push(("v2-baseline".to_string(), bs));
    }
    for name in list_sessions(&v2_sessions_dir) {
        let best_path = v2_sessions_dir.join(&name).join("best.json");
        if !best_path.exists() { continue; }
        if let Ok(text) = std::fs::read_to_string(&best_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Ok(pp) = serde_json::from_value::<[PolicyParams; 5]>(json["playerParams"].clone()) {
                    let mut bs = [PlayerBrain::default(); 5];
                    for i in 0..5 { bs[i] = PlayerBrain::V2(pp[i]); }
                    entries.push((format!("v2-{}", name), bs));
                }
            }
        }
    }

    // v3: baseline + every session champion
    if let Ok(v3) = session::read_team_baseline_v3(&v3_dir.join("baseline.json")) {
        let mut bs = [PlayerBrain::default(); 5];
        for i in 0..5 { bs[i] = PlayerBrain::V3(v3.player_params[i]); }
        entries.push(("v3-baseline".to_string(), bs));
    }
    {
        let v3_sessions_dir = v3_dir.join("sessions");
        for name in list_sessions(&v3_sessions_dir) {
            let best_path = v3_sessions_dir.join(&name).join("best.json");
            if !best_path.exists() { continue; }
            if let Ok(text) = std::fs::read_to_string(&best_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Ok(pp) = serde_json::from_value::<[V3Params; 5]>(json["playerParams"].clone()) {
                        let mut bs = [PlayerBrain::default(); 5];
                        for i in 0..5 { bs[i] = PlayerBrain::V3(pp[i]); }
                        entries.push((format!("v3-{}", name), bs));
                    }
                }
            }
        }
    }

    // v4: baseline + every session champion (only canonical session-N, skip slot-* / combined-*)
    let v4_dir = policies_root.join("v4");
    if let Ok(v4) = session::read_team_baseline_v4(&v4_dir.join("baseline.json")) {
        let mut bs = [PlayerBrain::default(); 5];
        for i in 0..5 { bs[i] = PlayerBrain::V4(v4.player_params[i]); }
        entries.push(("v4-baseline".to_string(), bs));
    }
    {
        let v4_sessions_dir = v4_dir.join("sessions");
        for name in list_sessions(&v4_sessions_dir) {
            // only "session-N" pattern, skip ablation-style sessions
            if !name.starts_with("session-") { continue; }
            let best_path = v4_sessions_dir.join(&name).join("best.json");
            if !best_path.exists() { continue; }
            if let Ok(text) = std::fs::read_to_string(&best_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Ok(pp) = serde_json::from_value::<[V4Params; 5]>(json["playerParams"].clone()) {
                        let mut bs = [PlayerBrain::default(); 5];
                        for i in 0..5 { bs[i] = PlayerBrain::V4(pp[i]); }
                        entries.push((format!("v4-{}", name), bs));
                    }
                }
            }
        }
    }

    // Sort: version (v1<v2<v3<v4), then baseline first, then session by trailing number
    entries.sort_by_key(|(name, _)| {
        let ver_rank = if name.starts_with("v1") { 0 }
                       else if name.starts_with("v2") { 1 }
                       else if name.starts_with("v3") { 2 }
                       else { 3 };
        let is_baseline = name.contains("-baseline");
        let session_num: u64 = name.chars().rev()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>().chars().rev().collect::<String>()
            .parse().unwrap_or(0);
        (ver_rank, if is_baseline { 0 } else { 1 }, session_num)
    });

    let n = entries.len();
    if n == 0 {
        println!("No entries found.");
        return;
    }
    let total_matches = (n * n * games_per_pair) as u64;
    println!("Matrix v2: {} entries x {} entries x {} games = {} matches", n, n, games_per_pair, total_matches);
    println!("Using {} CPU threads via rayon", rayon::current_num_threads());

    let start = std::time::Instant::now();
    let mut matrix: Vec<Vec<MatrixCell>> = vec![vec![MatrixCell::default(); n]; n];

    for i in 0..n {
        let row_start = std::time::Instant::now();
        for j in 0..n {
            let bi = entries[i].1;
            let bj = entries[j].1;
            let seed_base = rand::random::<u64>();

            let scores: Vec<(u32, u32)> = (0..games_per_pair).into_par_iter()
                .map(|k| {
                    let seed = seed_base.wrapping_add(k as u64);
                    let swap = k % 2 == 1;
                    // 5 brains for team 0, 5 for team 1, side-swapped
                    let (home, away) = if swap { (&bj, &bi) } else { (&bi, &bj) };
                    let mut all = [PlayerBrain::default(); 10];
                    for s in 0..5 { all[s] = home[s]; all[5 + s] = away[s]; }

                    let mut rng = SmallRng::seed_from_u64(seed);
                    let mut game = Game::for_mixed_battle(all);
                    while game.phase != crate::game::Phase::Fulltime {
                        step_game(&mut game, &mut rng);
                    }
                    if swap { (game.score[1], game.score[0]) } else { (game.score[0], game.score[1]) }
                })
                .collect();

            let mut cell = MatrixCell { games: games_per_pair as u32, ..Default::default() };
            for (s_i, s_j) in scores {
                cell.team0_goals += s_i as u64;
                cell.team1_goals += s_j as u64;
                if s_i > s_j { cell.team0_wins += 1; }
                else if s_j > s_i { cell.team1_wins += 1; }
                else { cell.draws += 1; }
            }
            matrix[i][j] = cell;
        }
        let row_ms = row_start.elapsed().as_millis();
        println!("Row {:>2}/{}: {} ({}ms)", i + 1, n, &entries[i].0, row_ms);
    }
    let elapsed = start.elapsed();
    println!("Matrix v2 complete in {:.1}s", elapsed.as_secs_f64());

    // Compute total stats from training summaries (v1 + v2)
    let (v1_matches, v1_min) = compute_total_stats(&policies_root.join("v1").join("sessions"));
    let (v2_matches, v2_min) = compute_total_stats(&v2_sessions_dir);
    let training_matches = v1_matches + v2_matches;
    let training_minutes = v1_min + v2_min;
    let grand_total_matches = training_matches + total_matches;

    let names: Vec<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();

    // JSON — written at top-level data/policies/ since matrix spans all versions
    let json_path = policies_root.join("matrix.json");
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
        serde_json::json!({ "entry": names[i], "row": cells })
    }).collect();
    let json_doc = serde_json::json!({
        "entries": names,
        "gamesPerPair": games_per_pair,
        "totalMatrixMatches": total_matches,
        "totalTrainingMatches": training_matches,
        "totalTrainingMinutes": training_minutes,
        "grandTotalMatches": grand_total_matches,
        "elapsedSec": elapsed.as_secs_f64(),
        "matrix": json_rows,
    });
    let _ = std::fs::write(&json_path, format!("{}\n", serde_json::to_string_pretty(&json_doc).unwrap()));

    // SVG — written at top-level data/policies/
    let svg_path = policies_root.join("matrix.svg");
    write_matrix_svg(&svg_path, &names, &matrix, grand_total_matches, training_minutes);

    println!("\nWritten:");
    println!("  {}", json_path.display());
    println!("  {}", svg_path.display());
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

fn v3_diff_slots(a: &TeamPolicyV3, b: &TeamPolicyV3) -> Vec<usize> {
    (0..5).filter(|i| {
        let p = &a[*i]; let q = &b[*i];
        let base_diff =
            (p.base.pass_chance_pressured - q.base.pass_chance_pressured).abs() > 1e-9
                || (p.base.pass_chance_wing - q.base.pass_chance_wing).abs() > 1e-9
                || (p.base.pass_chance_forward - q.base.pass_chance_forward).abs() > 1e-9
                || (p.base.pass_chance_default - q.base.pass_chance_default).abs() > 1e-9
                || (p.base.shoot_progress_threshold - q.base.shoot_progress_threshold).abs() > 1e-9
                || (p.base.tackle_chance - q.base.tackle_chance).abs() > 1e-9
                || (p.base.forward_pass_min_gain - q.base.forward_pass_min_gain).abs() > 1e-9
                || (p.base.mark_distance - q.base.mark_distance).abs() > 1e-9;
        let mod_diff =
            (p.aggression - q.aggression).abs() > 1e-9
                || (p.risk_appetite - q.risk_appetite).abs() > 1e-9
                || (p.edge_avoidance - q.edge_avoidance).abs() > 1e-9
                || (p.pressure_radius - q.pressure_radius).abs() > 1e-9
                || (p.goal_attraction - q.goal_attraction).abs() > 1e-9
                || (p.block_avoidance - q.block_avoidance).abs() > 1e-9
                || (p.block_distance - q.block_distance).abs() > 1e-9
                || (p.clear_shot_bonus - q.clear_shot_bonus).abs() > 1e-9
                || p.corridor_preference != q.corridor_preference
                || (0..5).any(|k| (p.zone_aggression[k] - q.zone_aggression[k]).abs() > 1e-9);
        base_diff || mod_diff
    }).collect()
}

// All probe-able fields on a V3Params instance, with their (min, max) bounds.
// We isolate one field at a time and test extremes vs the current champion.
// Adds the v4-only fields on top of the v3 list. Used by run_v4_rough_all.
const V4_ONLY_FIELDS: &[(&str, f32, f32)] = &[
    ("passDirOffensive", 0.0, 2.0),
    ("passDirDefensive", 0.0, 2.0),
    ("passDirNeutral",   0.0, 2.0),
    ("gkFreedom",        0.0, 1.0),
];

fn set_field_v4(p: &mut V4Params, field: &str, value: f32) {
    match field {
        "passDirOffensive" => p.pass_dir_offensive = value,
        "passDirDefensive" => p.pass_dir_defensive = value,
        "passDirNeutral"   => p.pass_dir_neutral = value,
        "gkFreedom"        => p.gk_freedom = value,
        _ => set_field(&mut p.v3, field, value),
    }
}
fn get_field_v4(p: &V4Params, field: &str) -> f32 {
    match field {
        "passDirOffensive" => p.pass_dir_offensive,
        "passDirDefensive" => p.pass_dir_defensive,
        "passDirNeutral"   => p.pass_dir_neutral,
        "gkFreedom"        => p.gk_freedom,
        _ => get_field(&p.v3, field),
    }
}

const ABLATION_FIELDS: &[(&str, f32, f32)] = &[
    ("base.passChancePressured",   0.02, 0.4),
    ("base.passChanceWing",        0.01, 0.25),
    ("base.passChanceForward",     0.005, 0.18),
    ("base.passChanceDefault",     0.005, 0.2),
    ("base.shootProgressThreshold",0.55, 0.9),
    ("base.tackleChance",          0.01, 0.22),
    ("base.forwardPassMinGain",    0.0, 18.0),
    ("base.markDistance",          25.0, 85.0),
    ("aggression",                 0.0, 2.0),
    ("riskAppetite",               0.0, 1.0),
    ("edgeAvoidance",              0.0, 1.0),
    ("pressureRadius",             30.0, 150.0),
    ("goalAttraction",             0.0, 1.0),
    ("blockAvoidance",             0.0, 1.0),
    ("blockDistance",              10.0, 60.0),
    ("clearShotBonus",             0.0, 1.0),
    ("zoneAggression0",            0.3, 2.0),
    ("zoneAggression1",            0.3, 2.0),
    ("zoneAggression2",            0.3, 2.0),
    ("zoneAggression3",            0.3, 2.0),
    ("zoneAggression4",            0.3, 2.0),
];

fn set_field(p: &mut V3Params, field: &str, value: f32) {
    match field {
        "base.passChancePressured" => p.base.pass_chance_pressured = value,
        "base.passChanceWing"      => p.base.pass_chance_wing = value,
        "base.passChanceForward"   => p.base.pass_chance_forward = value,
        "base.passChanceDefault"   => p.base.pass_chance_default = value,
        "base.shootProgressThreshold" => p.base.shoot_progress_threshold = value,
        "base.tackleChance"        => p.base.tackle_chance = value,
        "base.forwardPassMinGain"  => p.base.forward_pass_min_gain = value.round(),
        "base.markDistance"        => p.base.mark_distance = value.round(),
        "aggression"               => p.aggression = value,
        "riskAppetite"             => p.risk_appetite = value,
        "edgeAvoidance"            => p.edge_avoidance = value,
        "pressureRadius"           => p.pressure_radius = value,
        "goalAttraction"           => p.goal_attraction = value,
        "blockAvoidance"           => p.block_avoidance = value,
        "blockDistance"            => p.block_distance = value,
        "clearShotBonus"           => p.clear_shot_bonus = value,
        "zoneAggression0"          => p.zone_aggression[0] = value,
        "zoneAggression1"          => p.zone_aggression[1] = value,
        "zoneAggression2"          => p.zone_aggression[2] = value,
        "zoneAggression3"          => p.zone_aggression[3] = value,
        "zoneAggression4"          => p.zone_aggression[4] = value,
        _ => eprintln!("unknown field: {}", field),
    }
}

fn get_field(p: &V3Params, field: &str) -> f32 {
    match field {
        "base.passChancePressured" => p.base.pass_chance_pressured,
        "base.passChanceWing"      => p.base.pass_chance_wing,
        "base.passChanceForward"   => p.base.pass_chance_forward,
        "base.passChanceDefault"   => p.base.pass_chance_default,
        "base.shootProgressThreshold" => p.base.shoot_progress_threshold,
        "base.tackleChance"        => p.base.tackle_chance,
        "base.forwardPassMinGain"  => p.base.forward_pass_min_gain,
        "base.markDistance"        => p.base.mark_distance,
        "aggression"               => p.aggression,
        "riskAppetite"             => p.risk_appetite,
        "edgeAvoidance"            => p.edge_avoidance,
        "pressureRadius"           => p.pressure_radius,
        "goalAttraction"           => p.goal_attraction,
        "blockAvoidance"           => p.block_avoidance,
        "blockDistance"            => p.block_distance,
        "clearShotBonus"           => p.clear_shot_bonus,
        "zoneAggression0"          => p.zone_aggression[0],
        "zoneAggression1"          => p.zone_aggression[1],
        "zoneAggression2"          => p.zone_aggression[2],
        "zoneAggression3"          => p.zone_aggression[3],
        "zoneAggression4"          => p.zone_aggression[4],
        _ => 0.0,
    }
}

/// Ternary search on one (slot, field): tests lo / mid / hi each iteration,
/// applies the winner if z>Z_ACCEPT, narrows the interval, repeats up to
/// `max_depth` times. Returns probe records for the report.
fn ablate_field_ternary(
    champion: &mut TeamPolicyV3,
    slot: usize,
    field: &str,
    lo: f32,
    hi: f32,
    max_depth: usize,
    games: usize,
    z_accept: f64,
) -> (Vec<serde_json::Value>, usize) {
    let mut probes = Vec::new();
    let mut interval_lo = lo;
    let mut interval_hi = hi;
    let mut accepted = 0usize;
    let slot_name = policy::TEAM_SLOT_NAMES[slot];

    for depth in 0..max_depth {
        let mid = (interval_lo + interval_hi) / 2.0;
        let current_val = get_field(&champion[slot], field);

        let mut best_target = current_val;
        let mut best_z: f64 = z_accept;

        for (label, target) in [("lo", interval_lo), ("mid", mid), ("hi", interval_hi)] {
            if (target - current_val).abs() < 1e-6 { continue; }

            let mut variant = *champion;
            set_field(&mut variant[slot], field, target);
            let eval = evaluate_team_policies_v3(champion, &variant, games);

            let won = eval.z_score > best_z;
            println!(
                "  d{} slot={} {} {} {}={:.3}->{:.3}  diff={:+.0} z={:+.2} games={}/{}{}",
                depth, slot, slot_name, label, field,
                current_val, target, eval.goal_diff, eval.z_score,
                eval.games, games,
                if won { " *winner*" } else { "" },
            );

            probes.push(serde_json::json!({
                "slot": slot, "slotName": slot_name, "field": field,
                "depth": depth, "label": label,
                "current": current_val, "target": target,
                "intervalLo": interval_lo, "intervalHi": interval_hi,
                "goalDiff": eval.goal_diff, "zScore": eval.z_score,
                "games": eval.games, "accepted": won,
            }));

            if won {
                best_z = eval.z_score;
                best_target = target;
            }
        }

        if best_target == current_val {
            // No candidate beat champion at this depth; stop early
            break;
        }

        // Commit the best target
        set_field(&mut champion[slot], field, best_target);
        accepted += 1;

        // Narrow interval around the winner
        let span = interval_hi - interval_lo;
        if (best_target - interval_lo).abs() < 1e-6 {
            interval_hi = mid;
        } else if (best_target - interval_hi).abs() < 1e-6 {
            interval_lo = mid;
        } else {
            // mid won — focus around it (quarter of original span on each side)
            let q = span / 4.0;
            interval_lo = (best_target - q).max(lo);
            interval_hi = (best_target + q).min(hi);
        }
    }
    (probes, accepted)
}

/// V4 version of ablate_field_ternary — operates on TeamPolicyV4 and uses
/// evaluate_team_policies_v4 + set_field_v4 / get_field_v4 so v4-specific
/// fields (passDir multipliers, gkFreedom) can be probed alongside v3 fields.
fn ablate_field_ternary_v4(
    champion: &mut TeamPolicyV4,
    slot: usize,
    field: &str,
    lo: f32,
    hi: f32,
    max_depth: usize,
    games: usize,
    z_accept: f64,
) -> (Vec<serde_json::Value>, usize) {
    let mut probes = Vec::new();
    let mut interval_lo = lo;
    let mut interval_hi = hi;
    let mut accepted = 0usize;
    let slot_name = policy::TEAM_SLOT_NAMES[slot];

    for depth in 0..max_depth {
        let mid = (interval_lo + interval_hi) / 2.0;
        let current_val = get_field_v4(&champion[slot], field);
        let mut best_target = current_val;
        let mut best_z: f64 = z_accept;

        for (label, target) in [("lo", interval_lo), ("mid", mid), ("hi", interval_hi)] {
            if (target - current_val).abs() < 1e-6 { continue; }

            let mut variant = *champion;
            set_field_v4(&mut variant[slot], field, target);
            let eval = evaluate_team_policies_v4(champion, &variant, games);

            let won = eval.z_score > best_z;
            println!(
                "  d{} slot={} {} {} {}={:.3}->{:.3}  diff={:+.0} z={:+.2} games={}/{}{}",
                depth, slot, slot_name, label, field,
                current_val, target, eval.goal_diff, eval.z_score,
                eval.games, games,
                if won { " *winner*" } else { "" },
            );
            probes.push(serde_json::json!({
                "slot": slot, "slotName": slot_name, "field": field,
                "depth": depth, "label": label,
                "current": current_val, "target": target,
                "intervalLo": interval_lo, "intervalHi": interval_hi,
                "goalDiff": eval.goal_diff, "zScore": eval.z_score,
                "games": eval.games, "accepted": won,
            }));

            if won {
                best_z = eval.z_score;
                best_target = target;
            }
        }

        if best_target == current_val { break; }
        set_field_v4(&mut champion[slot], field, best_target);
        accepted += 1;
        let span = interval_hi - interval_lo;
        if (best_target - interval_lo).abs() < 1e-6 {
            interval_hi = mid;
        } else if (best_target - interval_hi).abs() < 1e-6 {
            interval_lo = mid;
        } else {
            let q = span / 4.0;
            interval_lo = (best_target - q).max(lo);
            interval_hi = (best_target + q).min(hi);
        }
    }
    (probes, accepted)
}

/// v4 rough calibration: ablate each slot independently against the v4
/// starting baseline. Produces 5 sessions + a combined champion that merges
/// each slot's wins into a single TeamPolicyV3.
fn run_v4_rough_all(policies_root: &Path, games: usize, max_depth: usize) {
    const Z_ACCEPT: f64 = 1.5;
    let v4_dir = policies_root.join("v4");
    let v4_baseline_path = v4_dir.join("baseline.json");

    // Read v4 baseline (V4Params format). Auto-bootstrap from v3 if missing.
    if !v4_baseline_path.exists() {
        let v3_baseline = policies_root.join("v3").join("baseline.json");
        if !v3_baseline.exists() {
            eprintln!("Neither v4 nor v3 baseline exists.");
            std::process::exit(1);
        }
        std::fs::create_dir_all(&v4_dir).expect("create v4 dir");
        let v3_file = match read_team_baseline_v3(&v3_baseline) {
            Ok(f) => f, Err(e) => { eprintln!("read v3: {}", e); std::process::exit(1); }
        };
        let v4_params: [V4Params; 5] = [0,1,2,3,4].map(|i| V4Params {
            v3: v3_file.player_params[i],
            pass_dir_offensive: 1.0, pass_dir_defensive: 1.0,
            pass_dir_neutral: 1.0, gk_freedom: 0.0,
        });
        let bootstrap = serde_json::json!({
            "name": "v4-baseline", "version": 1,
            "type": "team-policy-v4",
            "playerParams": v4_params,
        });
        let _ = std::fs::write(&v4_baseline_path,
            format!("{}\n", serde_json::to_string_pretty(&bootstrap).unwrap()));
        println!("v4 bootstrapped from v3.");
    }

    let baseline_file = match read_team_baseline_v4(&v4_baseline_path) {
        Ok(b) => b,
        Err(e) => { eprintln!("Error reading v4 baseline: {}", e); std::process::exit(1); }
    };
    let initial_team: TeamPolicyV4 = baseline_file.player_params;

    let role_labels = ["fwd", "mid-top", "mid-bottom", "def", "gk"];
    let games_tag = if games >= 1_000_000 { format!("{}M", games / 1_000_000) }
                   else if games >= 1_000 { format!("{}k", games / 1_000) }
                   else { format!("{}", games) };

    // Combine v3-fields + v4-only fields for full ablation
    let total_fields = ABLATION_FIELDS.len() + V4_ONLY_FIELDS.len();
    println!("v4 rough calibration: 5 slots × {} fields ({} v3 + {} v4) × depth={}, {} games per probe",
             total_fields, ABLATION_FIELDS.len(), V4_ONLY_FIELDS.len(), max_depth, games);
    println!("Starting baseline: {}", v4_baseline_path.display());
    println!("Session tag: -{}\n", games_tag);

    let total_started = std::time::Instant::now();
    let mut combined_champion = initial_team;

    for slot in 0..5 {
        let session_name = format!("slot-{}-{}-rough-{}", slot, role_labels[slot], games_tag);
        let writer = match SessionWriter::new(&v4_dir, &session_name) {
            Ok(w) => w,
            Err(e) => { eprintln!("Error creating session dir: {}", e); continue; }
        };

        println!("─── slot {} ({}) ──────────────────────────────────────", slot, role_labels[slot]);
        let slot_started = std::time::Instant::now();
        let mut champion = initial_team;
        let mut probes_for_slot: Vec<serde_json::Value> = Vec::new();
        let mut accepted_for_slot = 0usize;

        for &(field, lo, hi) in ABLATION_FIELDS.iter().chain(V4_ONLY_FIELDS.iter()) {
            let (probes, accepted) = ablate_field_ternary_v4(
                &mut champion, slot, field, lo, hi, max_depth, games, Z_ACCEPT,
            );
            probes_for_slot.extend(probes);
            accepted_for_slot += accepted;
        }

        // Save session in v4 format
        let _ = writer.write_team_v4_initial_baseline(&initial_team, &iso_now());
        let _ = writer.write_team_v4_best(0, &champion, &session_name);
        let report = serde_json::json!({
            "name": session_name, "mode": "v4-rough-calibration",
            "slot": slot, "slotName": role_labels[slot],
            "gamesPerProbe": games, "maxDepth": max_depth, "zAccept": Z_ACCEPT,
            "totalProbes": probes_for_slot.len(),
            "acceptedCount": accepted_for_slot,
            "elapsedSec": slot_started.elapsed().as_secs_f64(),
            "probes": probes_for_slot,
        });
        let _ = std::fs::write(
            writer.session_dir().join("ablation-report.json"),
            format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
        );

        let eval = evaluate_team_policies_v4(&initial_team, &champion, games);
        println!(
            "slot {} done in {:.1}s: {} field-improvements applied. vs v4-baseline: diff={:+.0} z={:+.2}\n",
            slot, slot_started.elapsed().as_secs_f64(), accepted_for_slot, eval.goal_diff, eval.z_score,
        );

        combined_champion[slot] = champion[slot];
    }

    let combined_name = format!("combined-rough-{}", games_tag);
    let combined_writer = SessionWriter::new(&v4_dir, &combined_name)
        .expect("combined session dir");
    let _ = combined_writer.write_team_v4_initial_baseline(&initial_team, &iso_now());
    let _ = combined_writer.write_team_v4_best(0, &combined_champion, &combined_name);

    println!("─── combined ──────────────────────────────────────────────");
    let final_eval = evaluate_team_policies_v4(&initial_team, &combined_champion, games);
    println!(
        "Combined champion (all 5 slots merged) vs v4-baseline: champion={:.3} baseline={:.3} diff={:+.0} z={:+.2}",
        final_eval.candidate_avg_goals, final_eval.baseline_avg_goals,
        final_eval.goal_diff, final_eval.z_score,
    );

    let total_elapsed = total_started.elapsed();
    let h = total_elapsed.as_secs() / 3600;
    let m = (total_elapsed.as_secs() % 3600) / 60;
    let s = total_elapsed.as_secs() % 60;
    if h > 0 { println!("\nTotal v4 rough-calibration time: {}h {}m {}s", h, m, s); }
    else { println!("\nTotal v4 rough-calibration time: {}m {}s", m, s); }
    println!("Sessions: data/policies/v4/sessions/slot-*-rough-{}/", games_tag);
    println!("Combined: data/policies/v4/sessions/combined-rough-{}/best.json", games_tag);
    println!("(v4/baseline.json NOT modified — copy combined best.json over to commit)");
}

/// V5 iterative rough calibration:
/// 1. Read v4 baseline as starting champion
/// 2. Run v4-style ablation for all 5 slots → produce iteration's combined
/// 3. Evaluate combined vs previous champion
/// 4. If z >= convergence_z → adopt as new champion, repeat
/// 5. If z < convergence_z → converged, stop
/// 6. Save final champion to v5/best.json (v4/baseline.json untouched)
fn run_v5_iterative(
    policies_root: &Path, games: usize,
    max_iters: usize, convergence_z: f64, max_depth: usize,
) {
    const Z_ACCEPT_PROBE: f64 = 1.5;  // per-probe acceptance inside ablation

    let v4_baseline_path = policies_root.join("v4").join("baseline.json");
    let v5_dir = policies_root.join("v5");
    std::fs::create_dir_all(&v5_dir).expect("create v5 dir");

    let baseline_file = match read_team_baseline_v4(&v4_baseline_path) {
        Ok(b) => b,
        Err(e) => { eprintln!("Error reading v4 baseline (v5 needs it as start): {}", e); std::process::exit(1); }
    };
    let initial_team: TeamPolicyV4 = baseline_file.player_params;
    let mut current_champion: TeamPolicyV4 = initial_team;

    let games_tag = if games >= 1_000 { format!("{}k", games / 1_000) } else { format!("{}", games) };

    println!(
        "v5 iterative rough calibration: max {} iterations, {} games per probe, convergence_z={}, max_depth={}",
        max_iters, games, convergence_z, max_depth,
    );
    println!("Starting baseline: {}", v4_baseline_path.display());
    println!("Convergence: stop when an iteration's combined champion has z < {} vs previous.\n", convergence_z);

    let role_labels = ["fwd", "mid-top", "mid-bottom", "def", "gk"];
    let started = std::time::Instant::now();
    let mut converged = false;
    let mut total_probes = 0usize;
    let mut total_accepted = 0usize;

    for iter in 0..max_iters {
        let iter_started = std::time::Instant::now();
        println!("════════ iteration {} ════════", iter);

        let mut combined = current_champion;
        let mut iter_probes = 0usize;
        let mut iter_accepted = 0usize;

        for slot in 0..5 {
            let session_name = format!("iter-{}-slot-{}-{}-rough-{}", iter, slot, role_labels[slot], games_tag);
            let writer = match SessionWriter::new(&v5_dir, &session_name) {
                Ok(w) => w,
                Err(e) => { eprintln!("session dir: {}", e); continue; }
            };
            let slot_started = std::time::Instant::now();
            // Each slot starts from current_champion (independent slot search)
            let mut slot_champion = current_champion;
            let mut probes_for_slot: Vec<serde_json::Value> = Vec::new();
            let mut acc_for_slot = 0usize;

            for &(field, lo, hi) in ABLATION_FIELDS.iter().chain(V4_ONLY_FIELDS.iter()) {
                let (probes, accepted) = ablate_field_ternary_v4(
                    &mut slot_champion, slot, field, lo, hi,
                    max_depth, games, Z_ACCEPT_PROBE,
                );
                probes_for_slot.extend(probes);
                acc_for_slot += accepted;
            }

            let _ = writer.write_team_v4_initial_baseline(&current_champion, &iso_now());
            let _ = writer.write_team_v4_best(0, &slot_champion, &session_name);
            let report = serde_json::json!({
                "name": session_name, "mode": "v5-iterative",
                "iteration": iter, "slot": slot, "slotName": role_labels[slot],
                "gamesPerProbe": games, "maxDepth": max_depth, "zAccept": Z_ACCEPT_PROBE,
                "totalProbes": probes_for_slot.len(),
                "acceptedCount": acc_for_slot,
                "elapsedSec": slot_started.elapsed().as_secs_f64(),
                "probes": probes_for_slot,
            });
            let _ = std::fs::write(
                writer.session_dir().join("ablation-report.json"),
                format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
            );

            iter_probes += probes_for_slot.len();
            iter_accepted += acc_for_slot;
            println!(
                "  iter {} slot {} ({}): {} probes, {} accepted, {:.1}s",
                iter, slot, role_labels[slot],
                probes_for_slot.len(), acc_for_slot, slot_started.elapsed().as_secs_f64(),
            );

            // Merge into combined
            combined[slot] = slot_champion[slot];
        }

        // Save this iteration's combined
        let combined_name = format!("iter-{}-combined-rough-{}", iter, games_tag);
        let combined_writer = SessionWriter::new(&v5_dir, &combined_name).expect("combined dir");
        let _ = combined_writer.write_team_v4_initial_baseline(&current_champion, &iso_now());
        let _ = combined_writer.write_team_v4_best(0, &combined, &combined_name);

        // Evaluate combined vs current_champion (= previous iteration's combined,
        // or initial baseline if iter==0)
        let conv_eval = evaluate_team_policies_v4(&current_champion, &combined, games);
        let iter_secs = iter_started.elapsed().as_secs_f64();
        println!(
            "  iter {} combined: {} probes, {} accepted, {:.1}s. vs prev champion: diff={:+.0} z={:+.2}",
            iter, iter_probes, iter_accepted, iter_secs, conv_eval.goal_diff, conv_eval.z_score,
        );

        total_probes += iter_probes;
        total_accepted += iter_accepted;

        if conv_eval.z_score < convergence_z {
            println!("  → CONVERGED (z={:.2} < {})\n", conv_eval.z_score, convergence_z);
            converged = true;
            break;
        }

        // Adopt as new champion for next iteration
        println!("  → significant improvement, adopting as next iter's baseline\n");
        current_champion = combined;
    }

    if !converged {
        println!("Reached max_iters={} without convergence.", max_iters);
    }

    // Final eval vs original v4 baseline
    println!("\n────── FINAL ──────");
    let final_eval = evaluate_team_policies_v4(&initial_team, &current_champion, games);
    println!(
        "v5 final champion vs v4 starting baseline: diff={:+.0} z={:+.2}",
        final_eval.goal_diff, final_eval.z_score,
    );

    // Save final champion to v5/best.json
    let best_path = v5_dir.join("best.json");
    let final_doc = serde_json::json!({
        "name": "v5-best", "version": 1, "type": "team-policy-v4",
        "description": "v5 iterative rough calibration champion. Same V4Params shape as v4 but refined via convergent coordinate ascent.",
        "sourceMode": "v5-iterative",
        "gamesPerProbe": games, "maxIters": max_iters, "convergenceZ": convergence_z,
        "iterationsRun": if converged { /* count from session dirs */ 0 } else { max_iters },
        "totalProbes": total_probes, "totalAccepted": total_accepted,
        "vsV4Baseline": { "goalDiff": final_eval.goal_diff, "zScore": final_eval.z_score },
        "playerParams": current_champion,
    });
    let _ = std::fs::write(&best_path,
        format!("{}\n", serde_json::to_string_pretty(&final_doc).unwrap()));

    let elapsed = started.elapsed();
    let h = elapsed.as_secs() / 3600;
    let m = (elapsed.as_secs() % 3600) / 60;
    let s = elapsed.as_secs() % 60;
    if h > 0 { println!("\nTotal v5 time: {}h {}m {}s", h, m, s); }
    else { println!("\nTotal v5 time: {}m {}s", m, s); }
    println!("Final: {}", best_path.display());
    println!("(v4/baseline.json NOT modified — copy v5/best.json over to commit)");
}

fn run_v3_ablate(policies_dir: &Path, games: usize, session_name: &str, max_depth: usize) {
    const Z_ACCEPT: f64 = 1.5;
    let baseline_path = policies_dir.join("baseline.json");
    let baseline_file = match read_team_baseline_v3(&baseline_path) {
        Ok(b) => b,
        Err(e) => { eprintln!("Error reading v3 baseline: {}", e); std::process::exit(1); }
    };
    let initial_team: TeamPolicyV3 = baseline_file.player_params;
    let mut champion: TeamPolicyV3 = initial_team;

    let writer = match SessionWriter::new(policies_dir, session_name) {
        Ok(w) => w,
        Err(e) => { eprintln!("Error creating session dir: {}", e); std::process::exit(1); }
    };

    println!(
        "v3 ablation '{}': 5 slots x {} fields x [lo,mid,hi] x depth={}, {} games per probe",
        session_name, ABLATION_FIELDS.len(), max_depth, games,
    );
    println!("Using {} CPU threads. Z_ACCEPT={}", rayon::current_num_threads(), Z_ACCEPT);
    println!("Worst-case probes: {}", 5 * ABLATION_FIELDS.len() * 3 * max_depth);

    let started = std::time::Instant::now();
    let mut all_probes: Vec<serde_json::Value> = Vec::new();
    let mut accepted_total = 0usize;

    for slot in 0..5 {
        for &(field, lo, hi) in ABLATION_FIELDS {
            let (probes, accepted) = ablate_field_ternary(
                &mut champion, slot, field, lo, hi,
                max_depth, games, Z_ACCEPT,
            );
            all_probes.extend(probes);
            accepted_total += accepted;
        }
    }

    let elapsed = started.elapsed();
    println!(
        "\nAblation done in {:.1}s. {} field-improvements applied across {} probes total.",
        elapsed.as_secs_f64(), accepted_total, all_probes.len()
    );

    let _ = writer.write_team_v3_initial_baseline(&initial_team, &iso_now());
    let _ = writer.write_team_v3_best(0, &champion, session_name);
    let report_doc = serde_json::json!({
        "name": session_name, "mode": "v3-ablate-ternary",
        "gamesPerProbe": games,
        "maxDepth": max_depth,
        "zAccept": Z_ACCEPT,
        "totalProbes": all_probes.len(),
        "acceptedCount": accepted_total,
        "elapsedSec": elapsed.as_secs_f64(),
        "probes": all_probes,
    });
    let _ = std::fs::write(
        writer.session_dir().join("ablation-report.json"),
        format!("{}\n", serde_json::to_string_pretty(&report_doc).unwrap()),
    );

    if accepted_total > 0 {
        println!("\nEvaluating ablated champion vs starting baseline ({} games)...", games);
        let final_eval = evaluate_team_policies_v3(&initial_team, &champion, games);
        println!(
            "vs starting baseline: champion={:.3} baseline={:.3} diff={:+.3} z={:+.2}",
            final_eval.candidate_avg_goals, final_eval.baseline_avg_goals,
            final_eval.goal_diff, final_eval.z_score,
        );
    }
    println!("Report: {}", writer.session_dir().join("ablation-report.json").display());
    println!("Best champion: {}", writer.session_dir().join("best.json").display());
    println!("(v3/baseline.json NOT modified — copy best.json over if you want to commit it)");
}

fn v4_diff_slots(a: &TeamPolicyV4, b: &TeamPolicyV4) -> Vec<usize> {
    (0..5).filter(|i| {
        let p = &a[*i]; let q = &b[*i];
        let v3_diff = (p.v3.base.pass_chance_pressured - q.v3.base.pass_chance_pressured).abs() > 1e-9
            || (p.v3.base.pass_chance_wing - q.v3.base.pass_chance_wing).abs() > 1e-9
            || (p.v3.base.pass_chance_forward - q.v3.base.pass_chance_forward).abs() > 1e-9
            || (p.v3.base.pass_chance_default - q.v3.base.pass_chance_default).abs() > 1e-9
            || (p.v3.base.shoot_progress_threshold - q.v3.base.shoot_progress_threshold).abs() > 1e-9
            || (p.v3.base.tackle_chance - q.v3.base.tackle_chance).abs() > 1e-9
            || (p.v3.base.forward_pass_min_gain - q.v3.base.forward_pass_min_gain).abs() > 1e-9
            || (p.v3.base.mark_distance - q.v3.base.mark_distance).abs() > 1e-9
            || (p.v3.aggression - q.v3.aggression).abs() > 1e-9
            || (p.v3.risk_appetite - q.v3.risk_appetite).abs() > 1e-9
            || (p.v3.edge_avoidance - q.v3.edge_avoidance).abs() > 1e-9
            || (p.v3.pressure_radius - q.v3.pressure_radius).abs() > 1e-9
            || (p.v3.goal_attraction - q.v3.goal_attraction).abs() > 1e-9
            || (p.v3.block_avoidance - q.v3.block_avoidance).abs() > 1e-9
            || (p.v3.block_distance - q.v3.block_distance).abs() > 1e-9
            || (p.v3.clear_shot_bonus - q.v3.clear_shot_bonus).abs() > 1e-9
            || p.v3.corridor_preference != q.v3.corridor_preference
            || (0..5).any(|k| (p.v3.zone_aggression[k] - q.v3.zone_aggression[k]).abs() > 1e-9);
        let v4_diff = (p.pass_dir_offensive - q.pass_dir_offensive).abs() > 1e-9
            || (p.pass_dir_defensive - q.pass_dir_defensive).abs() > 1e-9
            || (p.pass_dir_neutral - q.pass_dir_neutral).abs() > 1e-9
            || (p.gk_freedom - q.gk_freedom).abs() > 1e-9;
        v3_diff || v4_diff
    }).collect()
}

fn run_v4_training(policies_dir: &Path, epochs: usize, games_per_epoch: usize, session_name: &str) {
    let baseline_path = policies_dir.join("baseline.json");

    // Auto-bootstrap v4 from v3 if missing
    if !baseline_path.exists() {
        let v3_baseline = policies_dir.parent().unwrap().join("v3").join("baseline.json");
        if !v3_baseline.exists() {
            eprintln!("Neither v4 nor v3 baseline exists. Run --v3 first.");
            std::process::exit(1);
        }
        std::fs::create_dir_all(policies_dir).expect("create v4 dir");
        // Read v3 and wrap each player in V4Params with default v4 fields
        let v3_file = read_team_baseline_v3(&v3_baseline).expect("read v3 baseline");
        let v4_params: [V4Params; 5] = [0,1,2,3,4].map(|i| V4Params {
            v3: v3_file.player_params[i],
            pass_dir_offensive: 1.0,
            pass_dir_defensive: 1.0,
            pass_dir_neutral: 1.0,
            gk_freedom: 0.0,
        });
        let bootstrap = serde_json::json!({
            "name": "v4-baseline", "version": 1,
            "type": "team-policy-v4",
            "description": "v4 bootstrapped from v3. New v4 fields default to v3-equivalent (multipliers=1, gk_freedom=0).",
            "playerParams": v4_params,
        });
        let _ = std::fs::write(&baseline_path,
            format!("{}\n", serde_json::to_string_pretty(&bootstrap).unwrap()));
        println!("v4 bootstrapped from v3 at {}", baseline_path.display());
    }

    let baseline_file = match read_team_baseline_v4(&baseline_path) {
        Ok(b) => b,
        Err(e) => { eprintln!("Error reading v4 baseline: {}", e); std::process::exit(1); }
    };
    let initial_team: TeamPolicyV4 = baseline_file.player_params;
    let mut champion: TeamPolicyV4 = initial_team;
    let mut champion_epoch: usize = 0;
    let session_started = iso_now();

    ensure_team_v4_genesis(&baseline_path, &baseline_file);
    let training_start = std::time::Instant::now();

    let writer = match SessionWriter::new(policies_dir, session_name) {
        Ok(w) => w,
        Err(e) => { eprintln!("Error creating session dir: {}", e); std::process::exit(1); }
    };
    let _ = writer.write_team_v4_initial_baseline(&initial_team, &session_started);

    println!("v4 training '{}': {} epochs x {} games/epoch (5 slots, spatial+pass-dir+gk-freedom)",
             session_name, epochs, games_per_epoch);
    println!("Using {} CPU threads via rayon", rayon::current_num_threads());

    let mut history: Vec<EpochSummary> = Vec::with_capacity(epochs);
    let mut scale_factor: f32 = 1.0;
    let mut rejection_streak: usize = 0;

    for epoch in 1..=epochs {
        let opponent_epoch = champion_epoch;
        let opponent = champion;
        let mut rng = rand::thread_rng();
        let candidate = mutate_team_v4(&champion, &mut rng, scale_factor);
        let mutated_slots = v4_diff_slots(&champion, &candidate);

        let eval = evaluate_team_policies_v4(&opponent, &candidate, games_per_epoch);
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
            EarlyStop::Worse => "worse".to_string(), EarlyStop::Better => "better".to_string(),
        });
        let stop_str = eval.early_stop.map(|s| match s {
            EarlyStop::Worse => " [EARLY STOP: worse]", EarlyStop::Better => " [EARLY STOP: better]",
        }).unwrap_or("");
        let slot_label: String = mutated_slots.iter()
            .map(|i| format!("{}#{}", policy::TEAM_SLOT_NAMES[*i], i))
            .collect::<Vec<_>>().join(",");

        println!("epoch-{:03} {} diff={:+.3} z={:.2} games={}/{} scale={:.3} mutated=[{}] champion={}{}",
            epoch, if accepted { "ACCEPTED" } else { "rejected" },
            eval.goal_diff, eval.z_score, eval.games, games_per_epoch,
            scale_factor, slot_label, current_champion_epoch, stop_str);

        let _ = writer.write_team_v4_epoch(
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
    let _ = writer.write_team_v4_summary(&session_started, &finished_at, epochs, games_per_epoch,
        champion_epoch, &champion, &history);
    let _ = writer.write_team_v4_best(champion_epoch, &champion, session_name);
    crate::svg::write_training_svg(
        &writer.session_dir().join("training-progress.svg"),
        &history, champion_epoch);

    if champion_epoch > 0 {
        println!("\nEvaluating final champion against v4 session baseline ({} games)...", games_per_epoch);
        let final_eval = evaluate_team_policies_v4(&initial_team, &champion, games_per_epoch);
        println!("vs session baseline: champion={:.3} baseline={:.3} diff={:+.3} z={:.2}",
            final_eval.candidate_avg_goals, final_eval.baseline_avg_goals,
            final_eval.goal_diff, final_eval.z_score);
        if final_eval.candidate_won {
            match update_team_v4_baseline(&baseline_path, &baseline_file, &champion,
                session_name, champion_epoch, final_eval.goal_diff, &iso_now()) {
                Ok(_) => println!("v4 baseline.json updated → epoch {} diff={:+.3}",
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

fn run_v3_training(policies_dir: &Path, epochs: usize, games_per_epoch: usize, session_name: &str) {
    let baseline_path = policies_dir.join("baseline.json");

    let baseline_file = match read_team_baseline_v3(&baseline_path) {
        Ok(b) => b,
        Err(e) => { eprintln!("Error reading v3 baseline: {}", e); std::process::exit(1); }
    };

    let initial_team: TeamPolicyV3 = baseline_file.player_params;
    let mut champion: TeamPolicyV3 = initial_team;
    let mut champion_epoch: usize = 0;
    let session_started = iso_now();

    ensure_team_v3_genesis(&baseline_path, &baseline_file);
    let training_start = std::time::Instant::now();

    let writer = match SessionWriter::new(policies_dir, session_name) {
        Ok(w) => w,
        Err(e) => { eprintln!("Error creating session dir: {}", e); std::process::exit(1); }
    };
    if let Err(e) = writer.write_team_v3_initial_baseline(&initial_team, &session_started) {
        eprintln!("Warning: could not write initial baseline: {}", e);
    }

    println!(
        "v3 training '{}': {} epochs x {} games/epoch (5 slots, spatial-aware)",
        session_name, epochs, games_per_epoch
    );
    println!("Using {} CPU threads via rayon", rayon::current_num_threads());

    let mut history: Vec<EpochSummary> = Vec::with_capacity(epochs);
    let mut scale_factor: f32 = 1.0;
    let mut rejection_streak: usize = 0;

    for epoch in 1..=epochs {
        let opponent_epoch = champion_epoch;
        let opponent = champion;
        let mut rng = rand::thread_rng();
        let candidate = mutate_team_v3(&champion, &mut rng, scale_factor);
        let mutated_slots = v3_diff_slots(&champion, &candidate);

        let eval = evaluate_team_policies_v3(&opponent, &candidate, games_per_epoch);
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
            EarlyStop::Worse => "worse".to_string(), EarlyStop::Better => "better".to_string(),
        });
        let stop_str = eval.early_stop.map(|s| match s {
            EarlyStop::Worse => " [EARLY STOP: worse]", EarlyStop::Better => " [EARLY STOP: better]",
        }).unwrap_or("");
        let slot_label: String = mutated_slots.iter()
            .map(|i| format!("{}#{}", policy::TEAM_SLOT_NAMES[*i], i))
            .collect::<Vec<_>>().join(",");

        println!(
            "epoch-{:03} {} diff={:+.3} z={:.2} games={}/{} scale={:.3} mutated=[{}] champion={}{}",
            epoch,
            if accepted { "ACCEPTED" } else { "rejected" },
            eval.goal_diff, eval.z_score, eval.games, games_per_epoch,
            scale_factor, slot_label, current_champion_epoch, stop_str,
        );

        if let Err(e) = writer.write_team_v3_epoch(
            epoch, opponent_epoch, &opponent, &candidate,
            accepted, current_champion_epoch, &current_champion,
            &mutated_slots, &eval, &iso_now(), games_per_epoch,
        ) { eprintln!("Warning: could not write epoch {}: {}", epoch, e); }

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
    let _ = writer.write_team_v3_summary(
        &session_started, &finished_at, epochs, games_per_epoch,
        champion_epoch, &champion, &history,
    );
    let _ = writer.write_team_v3_best(champion_epoch, &champion, session_name);
    crate::svg::write_training_svg(
        &writer.session_dir().join("training-progress.svg"),
        &history, champion_epoch,
    );

    if champion_epoch > 0 {
        println!("\nEvaluating final champion against v3 session baseline ({} games)...", games_per_epoch);
        let final_eval = evaluate_team_policies_v3(&initial_team, &champion, games_per_epoch);
        println!(
            "vs session baseline: champion={:.3} baseline={:.3} diff={:+.3} z={:.2}",
            final_eval.candidate_avg_goals, final_eval.baseline_avg_goals,
            final_eval.goal_diff, final_eval.z_score,
        );
        let genesis_path = policies_dir.join("baseline-genesis.json");
        if let Ok(g) = read_team_baseline_v3(&genesis_path) {
            let ge = evaluate_team_policies_v3(&g.player_params, &champion, games_per_epoch);
            println!(
                "vs v3 genesis:       champion={:.3} genesis={:.3}  diff={:+.3} z={:.2}",
                ge.candidate_avg_goals, ge.baseline_avg_goals, ge.goal_diff, ge.z_score,
            );
        }
        if final_eval.candidate_won {
            match update_team_v3_baseline(
                &baseline_path, &baseline_file, &champion,
                session_name, champion_epoch, final_eval.goal_diff, &iso_now(),
            ) {
                Ok(_) => println!("v3 baseline.json updated → epoch {} diff={:+.3}", champion_epoch, final_eval.goal_diff),
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
    let h = total_secs / 3600;
    let m = (total_secs % 3600) / 60;
    let s = total_secs % 60;
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
    let v1_dir = project_root.join("data").join("policies").join("v1");
    let v2_dir = project_root.join("data").join("policies").join("v2");

    // v1-only utilities (work against the frozen v1 archive)
    if args.get(1).map(|s| s.as_str()) == Some("--regen-progress") {
        regenerate_progress_svg(&v1_dir, &v1_dir.join("baseline.json"));
        return;
    }
    if args.get(1).map(|s| s.as_str()) == Some("--matrix") {
        let games: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1000);
        run_matrix(&v1_dir, games);
        return;
    }
    if args.get(1).map(|s| s.as_str()) == Some("--matrix-v2") {
        let games: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1000);
        let policies_root = project_root.join("data").join("policies");
        run_matrix_v2(&policies_root, games);
        return;
    }
    if args.get(1).map(|s| s.as_str()) == Some("--build-opponents") {
        let policies_root = project_root.join("data").join("policies");
        build_opponents_index(&policies_root);
        return;
    }

    // ─── v3 training (when --v3 is passed) ──────────────────────────────────
    // v3-ablate: coordinate-ascent over (slot, field) with ternary search
    // Args: <games_per_probe> <session_name> [max_depth=1]
    if args.get(1).map(|s| s.as_str()) == Some("--v3-ablate") {
        let v3_dir = project_root.join("data").join("policies").join("v3");
        let games: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10000);
        let session_name: String = args.get(3).cloned().unwrap_or_else(|| "ablate-1".to_string());
        let max_depth: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1).min(10);
        run_v3_ablate(&v3_dir, games, &session_name, max_depth);
        return;
    }

    // v4-rough: 5 separate rough-calibration sessions, one per player slot.
    // v4 starts from v3 baseline and ablates each slot independently to map
    // which skills matter for which position. Output combined into v4 dir.
    if args.get(1).map(|s| s.as_str()) == Some("--v4-rough") {
        let policies_root = project_root.join("data").join("policies");
        let games: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10000);
        let max_depth: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(10).min(10);
        run_v4_rough_all(&policies_root, games, max_depth);
        return;
    }

    // v5: iterative rough calibration with convergence check.
    // Args: <games_per_probe> [max_iters=10] [convergence_z=1.5] [max_depth=8]
    // Repeats v4-style ablation; stops when an iteration's combined champion
    // is not significantly better than previous iteration's combined.
    if args.get(1).map(|s| s.as_str()) == Some("--v5-rough") {
        let policies_root = project_root.join("data").join("policies");
        let games: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(5000);
        let max_iters: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(10).min(20);
        let convergence_z: f64 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1.5);
        let max_depth: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(8).min(10);
        run_v5_iterative(&policies_root, games, max_iters, convergence_z, max_depth);
        return;
    }
    if args.get(1).map(|s| s.as_str()) == Some("--v4") {
        let v4_dir = project_root.join("data").join("policies").join("v4");
        let epochs: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(100);
        let games_per_epoch: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1000);
        let session_name: String = args.get(4).cloned().unwrap_or_else(|| "session-1".to_string());
        run_v4_training(&v4_dir, epochs, games_per_epoch, &session_name);
        return;
    }
    if args.get(1).map(|s| s.as_str()) == Some("--v3") {
        let v3_dir = project_root.join("data").join("policies").join("v3");
        let epochs: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(100);
        let games_per_epoch: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1000);
        let session_name: String = args.get(4).cloned().unwrap_or_else(|| "session-1".to_string());
        run_v3_training(&v3_dir, epochs, games_per_epoch, &session_name);
        return;
    }

    // ─── v2 training (default) ─────────────────────────────────────────────
    let policies_dir = v2_dir;
    let baseline_path = policies_dir.join("baseline.json");

    let epochs: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(100);
    let games_per_epoch: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1000);
    let session_name: &str = args.get(3).map(|s| s.as_str()).unwrap_or("session-1");

    let baseline_file = match read_team_baseline(&baseline_path) {
        Ok(b) => b,
        Err(e) => { eprintln!("Error reading v2 baseline: {}", e); std::process::exit(1); }
    };

    let initial_team: TeamPolicy = baseline_file.player_params;
    let mut champion: TeamPolicy = initial_team;
    let mut champion_epoch: usize = 0;
    let session_started = iso_now();

    ensure_team_genesis(&baseline_path, &baseline_file);

    let training_start = std::time::Instant::now();

    let writer = match SessionWriter::new(&policies_dir, session_name) {
        Ok(w) => w,
        Err(e) => { eprintln!("Error creating session dir: {}", e); std::process::exit(1); }
    };

    if let Err(e) = writer.write_team_initial_baseline(&initial_team, &session_started) {
        eprintln!("Warning: could not write initial baseline: {}", e);
    }

    println!(
        "v2 training '{}': {} epochs x {} games/epoch (5 player slots per team)",
        session_name, epochs, games_per_epoch
    );
    println!("Using {} CPU threads via rayon", rayon::current_num_threads());

    let mut history: Vec<EpochSummary> = Vec::with_capacity(epochs);
    let mut scale_factor: f32 = 1.0;
    let mut rejection_streak: usize = 0;
    const SCALE_DECAY_EVERY: usize = 20;
    const SCALE_DECAY_FACTOR: f32 = 0.75;
    const SCALE_FLOOR: f32 = 0.1;
    const SCALE_RECOVER_FACTOR: f32 = 1.5;

    for epoch in 1..=epochs {
        let opponent_epoch = champion_epoch;
        let opponent = champion;
        let mut rng = rand::thread_rng();
        let candidate = mutate_team(&champion, &mut rng, scale_factor);

        // Detect which slots changed so the epoch record can show what was tried.
        let mutated_slots: Vec<usize> = (0..5)
            .filter(|i| {
                let a = &champion[*i];
                let b = &candidate[*i];
                (a.pass_chance_pressured - b.pass_chance_pressured).abs() > 1e-9
                    || (a.pass_chance_wing - b.pass_chance_wing).abs() > 1e-9
                    || (a.pass_chance_forward - b.pass_chance_forward).abs() > 1e-9
                    || (a.pass_chance_default - b.pass_chance_default).abs() > 1e-9
                    || (a.shoot_progress_threshold - b.shoot_progress_threshold).abs() > 1e-9
                    || (a.tackle_chance - b.tackle_chance).abs() > 1e-9
                    || (a.forward_pass_min_gain - b.forward_pass_min_gain).abs() > 1e-9
                    || (a.mark_distance - b.mark_distance).abs() > 1e-9
            })
            .collect();

        let eval = evaluate_team_policies(&opponent, &candidate, games_per_epoch);
        let accepted = eval.candidate_won;
        if accepted {
            champion = candidate;
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
        let current_champion = if accepted { candidate } else { champion };

        let early_label = eval.early_stop.map(|s| match s {
            EarlyStop::Worse => "worse".to_string(),
            EarlyStop::Better => "better".to_string(),
        });
        let stop_str = eval.early_stop.map(|s| match s {
            EarlyStop::Worse => " [EARLY STOP: worse]",
            EarlyStop::Better => " [EARLY STOP: better]",
        }).unwrap_or("");

        let slot_names = policy::TEAM_SLOT_NAMES;
        let slot_label: String = mutated_slots.iter()
            .map(|i| format!("{}#{}", slot_names[*i], i))
            .collect::<Vec<_>>()
            .join(",");

        println!(
            "epoch-{:03} {} diff={:+.3} z={:.2} games={}/{} scale={:.3} mutated=[{}] champion={}{}",
            epoch,
            if accepted { "ACCEPTED" } else { "rejected" },
            eval.goal_diff,
            eval.z_score,
            eval.games,
            games_per_epoch,
            scale_factor,
            slot_label,
            current_champion_epoch,
            stop_str,
        );

        if let Err(e) = writer.write_team_epoch(
            epoch, opponent_epoch, &opponent, &candidate,
            accepted, current_champion_epoch, &current_champion,
            &mutated_slots, &eval, &iso_now(), games_per_epoch,
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

    if let Err(e) = writer.write_team_summary(
        &session_started, &finished_at, epochs, games_per_epoch,
        champion_epoch, &champion, &history,
    ) {
        eprintln!("Warning: could not write summary: {}", e);
    }

    if let Err(e) = writer.write_team_best(champion_epoch, &champion, session_name) {
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
            "\nEvaluating final champion against v2 session baseline ({} games)...",
            games_per_epoch
        );
        let final_eval = evaluate_team_policies(&initial_team, &champion, games_per_epoch);
        println!(
            "vs session baseline: champion={:.3} baseline={:.3} diff={:+.3} z={:.2}",
            final_eval.candidate_avg_goals,
            final_eval.baseline_avg_goals,
            final_eval.goal_diff,
            final_eval.z_score,
        );

        let genesis_path = policies_dir.join("baseline-genesis.json");
        if let Ok(genesis_file) = read_team_baseline(&genesis_path) {
            let genesis_eval = evaluate_team_policies(&genesis_file.player_params, &champion, games_per_epoch);
            println!(
                "vs v2 genesis:       champion={:.3} genesis={:.3}  diff={:+.3} z={:.2}",
                genesis_eval.candidate_avg_goals,
                genesis_eval.baseline_avg_goals,
                genesis_eval.goal_diff,
                genesis_eval.z_score,
            );
        }

        if final_eval.candidate_won {
            match update_team_baseline(
                &baseline_path, &baseline_file, &champion,
                session_name, champion_epoch, final_eval.goal_diff, &iso_now(),
            ) {
                Ok(_) => println!(
                    "v2 baseline.json updated — version incremented, history appended (epoch {} diff={:+.3})",
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
