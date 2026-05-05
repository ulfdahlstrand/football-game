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

use policy::{mutate, mutate_team, mutate_team_v3, mutate_team_v4, mutate_team_v6,
              v6_from_v4, PolicyParams, TeamPolicy, TeamPolicyV3, TeamPolicyV4, TeamPolicyV6,
              V3Params, V4Params, V6Params};
use session::{
    ensure_genesis, ensure_team_genesis, ensure_team_v3_genesis, ensure_team_v4_genesis,
    ensure_team_v6_genesis,
    read_baseline, read_team_baseline, read_team_baseline_v3, read_team_baseline_v4,
    read_team_baseline_v6,
    update_baseline, update_team_baseline, update_team_v3_baseline, update_team_v4_baseline,
    update_team_v6_baseline,
    EpochSummary, SessionWriter,
};
use trainer::{evaluate_policies, evaluate_team_policies, evaluate_team_policies_v3,
              evaluate_team_policies_v4, evaluate_team_policies_v6, play_match, EarlyStop};
use svg::{write_training_svg, write_progress_svg, write_matrix_svg, write_tournament_svg, SessionProgress, MatrixCell};
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
    add_version(&mut opponents, policies_root, "v6");

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
            // Sensible per-slot defaults: GK glued to box, others free.
            max_distance_from_goal: if i == 4 { 0.10 } else { 1.0 },
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

fn v6_diff_slots(a: &TeamPolicyV6, b: &TeamPolicyV6) -> Vec<usize> {
    (0..5).filter(|&i| {
        let pa = serde_json::to_value(a[i]).unwrap();
        let pb = serde_json::to_value(b[i]).unwrap();
        pa != pb
    }).collect()
}

fn run_v6_training(policies_dir: &Path, epochs: usize, games_per_epoch: usize, session_name: &str) {
    let baseline_path = policies_dir.join("baseline.json");

    // Auto-bootstrap v6 from v4 if missing
    if !baseline_path.exists() {
        let v4_baseline = policies_dir.parent().unwrap().join("v4").join("baseline.json");
        if !v4_baseline.exists() {
            eprintln!("Neither v6 nor v4 baseline exists. Run --v4 first or seed v6 baseline.");
            std::process::exit(1);
        }
        std::fs::create_dir_all(policies_dir).expect("create v6 dir");
        let v4_file = read_team_baseline_v4(&v4_baseline).expect("read v4 baseline");
        let v6_params: [V6Params; 5] = [0,1,2,3,4].map(|i| v6_from_v4(&v4_file.player_params[i], i));
        let bootstrap = serde_json::json!({
            "name": "v6-baseline", "version": 1,
            "type": "team-policy-v6",
            "description": "v6 bootstrapped from v4. Spatial preferences seeded with per-slot defaults; decisions copied from v4.",
            "playerParams": v6_params,
        });
        let _ = std::fs::write(&baseline_path,
            format!("{}\n", serde_json::to_string_pretty(&bootstrap).unwrap()));
        println!("v6 bootstrapped from v4 at {}", baseline_path.display());
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
            max_distance_from_goal: if i == 4 { 0.10 } else { 1.0 },
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
            EarlyStop::Worse => "worse".to_string(), EarlyStop::Better => "better".to_string(), EarlyStop::Indecisive => "indecisive".to_string(),
        });
        let stop_str = eval.early_stop.map(|s| match s {
            EarlyStop::Worse => " [EARLY STOP: worse]", EarlyStop::Better => " [EARLY STOP: better]", EarlyStop::Indecisive => " [INDECISIVE: futile]",
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
    if args.get(1).map(|s| s.as_str()) == Some("--v6-tournament") {
        let games_per_match: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10000);
        run_v6_tournament(&project_root, games_per_match);
        return;
    }
    if args.get(1).map(|s| s.as_str()) == Some("--legacy-tournament") {
        // Full round-robin: all v6 teams + v1/v2/v3/v4 baselines (converted to v6 format).
        // Usage: --legacy-tournament [games_per_match]
        let games_per_match: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1000);
        run_legacy_tournament(&project_root, games_per_match);
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
    if args.get(1).map(|s| s.as_str()) == Some("--v6-team-train") {
        let team_name: String = args.get(2).cloned().unwrap_or_else(|| "granite-athletic".to_string());
        let variant = if args.iter().any(|s| s == "--full") { AnnealVariant::Full }
                      else if args.iter().any(|s| s == "--quick") { AnnealVariant::Quick }
                      else { AnnealVariant::Short };
        run_v6_team_train(&project_root, &team_name, variant);
        return;
    }
    if args.get(1).map(|s| s.as_str()) == Some("--v6-from-v4") {
        let team_name: String = args.get(2).cloned().unwrap_or_else(|| "glacier-fc".to_string());
        let variant = if args.iter().any(|s| s == "--full") { AnnealVariant::Full }
                      else if args.iter().any(|s| s == "--quick") { AnnealVariant::Quick }
                      else { AnnealVariant::Short };
        run_v6_from_v4(&project_root, &team_name, variant);
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
            EarlyStop::Indecisive => "indecisive".to_string(),
        });
        let stop_str = eval.early_stop.map(|s| match s {
            EarlyStop::Worse => " [EARLY STOP: worse]",
            EarlyStop::Better => " [EARLY STOP: better]",
            EarlyStop::Indecisive => " [INDECISIVE: futile]",
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
    run_team_anneal_with_prefix(team_dir, team_name, initial, stages, "", true)
}

fn run_team_anneal_with_prefix(
    team_dir: &Path,
    team_name: &str,
    initial: TeamPolicyV6,
    stages: &[(usize, usize)],
    session_prefix: &str,
    write_initial_baseline: bool,
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

fn run_legacy_tournament(project_root: &Path, games_per_match: usize) {
    let teams_dir  = project_root.join("data").join("teams");
    let policies_dir = project_root.join("data").join("policies");
    let matrices_dir = project_root.join("data").join("matrices");
    std::fs::create_dir_all(&matrices_dir).expect("create matrices dir");

    // ── helpers: convert older formats → TeamPolicyV6 ─────────────────────
    let policy_to_v6 = |pp: &PolicyParams, slot: usize| -> V6Params {
        let v3 = V3Params { base: *pp, ..Default::default() };
        let v4 = V4Params { v3, ..Default::default() };
        v6_from_v4(&v4, slot)
    };
    let v3_to_v6 = |v3: &V3Params, slot: usize| -> V6Params {
        let v4 = V4Params { v3: *v3, ..Default::default() };
        v6_from_v4(&v4, slot)
    };

    // ── 1. load current v6 teams ───────────────────────────────────────────
    let mut teams: Vec<(String, TeamPolicyV6)> = vec![];
    if let Ok(entries) = std::fs::read_dir(&teams_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() { continue; }
            let baseline = path.join("baseline.json");
            if !baseline.exists() { continue; }
            let team_name = path.file_name().unwrap().to_string_lossy().into_owned();
            match read_team_baseline_v6(&baseline) {
                Ok(b) => teams.push((team_name, b.player_params)),
                Err(e) => eprintln!("  ! skip {}: {}", team_name, e),
            }
        }
    }
    teams.sort_by(|a, b| a.0.cmp(&b.0));

    // ── 2. load legacy vX baselines and convert ────────────────────────────
    // v1 — single PolicyParams, replicate to 5 slots
    let v1_path = policies_dir.join("v1").join("baseline.json");
    if let Ok(f) = read_baseline(&v1_path) {
        let team: TeamPolicyV6 = std::array::from_fn(|slot| policy_to_v6(&f.parameters, slot));
        teams.push(("v1-baseline".to_string(), team));
        println!("  loaded v1-baseline");
    } else {
        eprintln!("  ! cannot read v1 baseline (skipping)");
    }

    // v2 — [PolicyParams; 5]
    let v2_path = policies_dir.join("v2").join("baseline.json");
    if let Ok(f) = read_team_baseline(&v2_path) {
        let team: TeamPolicyV6 = std::array::from_fn(|slot| policy_to_v6(&f.player_params[slot], slot));
        teams.push(("v2-baseline".to_string(), team));
        println!("  loaded v2-baseline");
    } else {
        eprintln!("  ! cannot read v2 baseline (skipping)");
    }

    // v3 — [V3Params; 5]
    let v3_path = policies_dir.join("v3").join("baseline.json");
    if let Ok(f) = read_team_baseline_v3(&v3_path) {
        let team: TeamPolicyV6 = std::array::from_fn(|slot| v3_to_v6(&f.player_params[slot], slot));
        teams.push(("v3-baseline".to_string(), team));
        println!("  loaded v3-baseline");
    } else {
        eprintln!("  ! cannot read v3 baseline (skipping)");
    }

    // v4 — [V4Params; 5]
    let v4_path = policies_dir.join("v4").join("baseline.json");
    if let Ok(f) = read_team_baseline_v4(&v4_path) {
        let team: TeamPolicyV6 = std::array::from_fn(|slot| v6_from_v4(&f.player_params[slot], slot));
        teams.push(("v4-baseline".to_string(), team));
        println!("  loaded v4-baseline");
    } else {
        eprintln!("  ! cannot read v4 baseline (skipping)");
    }

    // ── 3. run round-robin ─────────────────────────────────────────────────
    let n = teams.len();
    if n < 2 {
        eprintln!("Need at least 2 participants. Found {}.", n);
        std::process::exit(1);
    }

    println!("\n=== LEGACY ROUND-ROBIN TOURNAMENT ===");
    println!("Participants: {}", n);
    println!("Games per matchup: {}", games_per_match);
    println!();
    for (name, _) in &teams { println!("  {}", name); }
    println!();

    crate::game::CLUSTER_START.store(false, std::sync::atomic::Ordering::Relaxed);

    let total_pairs = n * (n - 1) / 2;
    let mut pair_idx = 0;
    let mut matrix:   Vec<Vec<f64>> = vec![vec![0.0; n]; n];
    let mut z_matrix: Vec<Vec<f64>> = vec![vec![0.0; n]; n];
    let mut pts:      Vec<Vec<f64>> = vec![vec![0.0; n]; n];
    let mut team_wins:   Vec<u64> = vec![0; n];
    let mut team_draws:  Vec<u64> = vec![0; n];
    let mut team_losses: Vec<u64> = vec![0; n];
    let tournament_start = std::time::Instant::now();

    for i in 0..n {
        for j in (i + 1)..n {
            pair_idx += 1;
            let eval = evaluate_team_policies_v6(&teams[i].1, &teams[j].1, games_per_match);
            matrix[i][j]   = -eval.goal_diff;
            matrix[j][i]   =  eval.goal_diff;
            z_matrix[i][j] = -eval.z_score;
            z_matrix[j][i] =  eval.z_score;
            let i_wins = eval.losses; let j_wins = eval.wins; let draws = eval.draws;
            let i_pts = i_wins as f64 * 3.0 + draws as f64;
            let j_pts = j_wins as f64 * 3.0 + draws as f64;
            pts[i][j] = i_pts; pts[j][i] = j_pts;
            team_wins[i] += i_wins; team_wins[j] += j_wins;
            team_draws[i] += draws; team_draws[j] += draws;
            team_losses[i] += j_wins; team_losses[j] += i_wins;
            println!("  [{}/{}] {:20} vs {:20}: pts={:.0}-{:.0} (W{}D{}L{}) diff={:+.0}",
                pair_idx, total_pairs, teams[i].0, teams[j].0,
                i_pts, j_pts, i_wins, draws, j_wins, eval.goal_diff);
        }
    }

    // ── 4. standings (primary: points, secondary: goal-diff) ──────────────
    let mut totals: Vec<(usize, f64, f64, f64)> = (0..n).map(|i| {
        let total_pts  = pts[i].iter().sum::<f64>();
        let total_diff = matrix[i].iter().sum::<f64>();
        let total_z    = z_matrix[i].iter().sum::<f64>();
        (i, total_pts, total_diff, total_z)
    }).collect();
    totals.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then(b.2.partial_cmp(&a.2).unwrap()));
    let totals_compat: Vec<(usize, f64, f64)> = totals.iter().map(|(i,_,gd,z)| (*i,*gd,*z)).collect();

    println!("\n=== STANDINGS ===");
    println!("{:>4}  {:22}  {:>6}  {:>4}  {:>4}  {:>4}  {:>10}", "rank", "team", "pts", "W", "D", "L", "goal-diff");
    for (rank, (i, total_pts, total_diff, _)) in totals.iter().enumerate() {
        println!("{:>4}  {:22}  {:>6.0}  {:>4}  {:>4}  {:>4}  {:>+10.0}",
            rank + 1, teams[*i].0, total_pts, team_wins[*i], team_draws[*i], team_losses[*i], total_diff);
    }

    // ── 5. write output ───────────────────────────────────────────────────
    let timestamp = iso_now().replace(':', "").replace('-', "").chars().take(15).collect::<String>();
    let out_dir = matrices_dir.join(format!("legacy-rr-{}", timestamp));
    std::fs::create_dir_all(&out_dir).expect("create output dir");

    let team_names: Vec<String> = teams.iter().map(|(n, _)| n.clone()).collect();
    let matrix_doc = serde_json::json!({
        "type": "legacy-round-robin",
        "createdAt": iso_now(),
        "gamesPerMatch": games_per_match,
        "teams": team_names,
        "goalDiffMatrix": matrix,
        "zScoreMatrix": z_matrix,
        "pointsMatrix": pts,
        "rankings": totals.iter().enumerate().map(|(rank, (i, tp, gd, z))| serde_json::json!({
            "rank": rank + 1, "team": teams[*i].0,
            "points": tp, "goalDiff": gd, "zSum": z,
            "wins": team_wins[*i], "draws": team_draws[*i], "losses": team_losses[*i],
        })).collect::<Vec<_>>(),
    });
    let _ = std::fs::write(out_dir.join("matrix.json"),
        format!("{}\n", serde_json::to_string_pretty(&matrix_doc).unwrap()));

    let team_name_refs: Vec<&str> = teams.iter().map(|(n, _)| n.as_str()).collect();
    write_tournament_svg(&out_dir.join("matrix.svg"), &team_name_refs, &matrix, &z_matrix, &totals_compat, games_per_match);

    let mut md = String::new();
    md.push_str(&format!("# Legacy round-robin — {}\n\n", iso_now()));
    md.push_str(&format!("Games per matchup: {}\n\n", games_per_match));
    md.push_str("## Standings\n\n");
    md.push_str("| Rank | Team | Pts | W | D | L | Goal diff |\n|------|------|-----|---|---|---|-----------|\n");
    for (rank, (i, total_pts, total_diff, _)) in totals.iter().enumerate() {
        md.push_str(&format!("| {} | {} | {:.0} | {} | {} | {} | {:+.0} |\n",
            rank + 1, teams[*i].0, total_pts, team_wins[*i], team_draws[*i], team_losses[*i], total_diff));
    }
    md.push_str("\n## Points matrix (row earned vs column)\n\n|     |");
    for (name, _) in &teams { md.push_str(&format!(" {} |", name)); }
    md.push('\n');
    md.push_str("|-----|");
    for _ in &teams { md.push_str("----|"); }
    md.push('\n');
    for i in 0..n {
        md.push_str(&format!("| **{}** |", teams[i].0));
        for j in 0..n {
            if i == j { md.push_str(" — |"); }
            else { md.push_str(&format!(" {:+.0} |", matrix[i][j])); }
        }
        md.push('\n');
    }
    let _ = std::fs::write(out_dir.join("summary.md"), md);

    let elapsed = tournament_start.elapsed();
    println!("\n=== TOURNAMENT COMPLETE in {:.0}s ===", elapsed.as_secs_f64());
    println!("Results → {}", out_dir.display());
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
        let (px, py) = compute_v6_preferred_xy(&params[i], own_goal_x);

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

    let final_team = run_team_anneal_with_prefix(&team_dir, team_name, initial, stages, &session_prefix, false);

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

fn run_v6_from_v4(project_root: &Path, team_name: &str, variant: AnnealVariant) {
    let v4_baseline_path = project_root.join("data/policies/v4/baseline.json");
    let v4_file = read_team_baseline_v4(&v4_baseline_path)
        .expect("cannot read v4 baseline");

    let initial: TeamPolicyV6 = std::array::from_fn(|slot| {
        v6_from_v4(&v4_file.player_params[slot], slot)
    });

    let team_dir = project_root.join("data/teams").join(team_name);
    std::fs::create_dir_all(&team_dir).expect("create team dir");

    let team_desc_idx = TEAM_NAMES.iter().position(|n| *n == team_name);
    let team_desc = team_desc_idx
        .and_then(|i| TEAM_DESCRIPTIONS.get(i))
        .copied()
        .unwrap_or(team_name);

    let label = match variant { AnnealVariant::Quick => "QUICK", AnnealVariant::Short => "SHORT", AnnealVariant::Full => "FULL" };
    let stages: &[(usize, usize)] = match variant {
        AnnealVariant::Full  => ANNEAL_STAGES_FULL,
        AnnealVariant::Quick => ANNEAL_STAGES_QUICK,
        AnnealVariant::Short => ANNEAL_STAGES_SHORT,
    };

    println!("=== V6 FROM V4 BASELINE: {} ===", team_name);
    println!("Source: {}", v4_baseline_path.display());
    println!("Variant: {} stages={:?}", label, stages);
    println!("Converting {} player slots via v6_from_v4...", initial.len());

    let final_team = run_team_anneal(&team_dir, team_name, initial, stages);
    write_team_info_md(&team_dir, team_name, team_desc, &final_team);
    write_team_layout_svg(&team_dir.join("layout.svg"), team_name, team_desc, &final_team);
    println!("\n{} trained and saved.", team_name);
}
