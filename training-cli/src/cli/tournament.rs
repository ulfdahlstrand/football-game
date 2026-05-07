use std::path::Path;

use training_engine::policy::TeamPolicyV6;
use training_engine::session::read_team_baseline_v6;
use training_render::write_tournament_svg;
use training_engine::trainer::evaluate_team_policies_v6;

use super::util::{iso_now, write_json_pretty};

pub fn run_v6_tournament(project_root: &Path, games_per_match: usize) {
    let teams_dir = project_root.join("data").join("teams");
    let matrices_dir = project_root.join("data").join("matrices");
    std::fs::create_dir_all(&matrices_dir).expect("create matrices dir");

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

    training_engine::game::CLUSTER_START.store(false, std::sync::atomic::Ordering::Relaxed);

    let total_pairs = n * (n - 1) / 2;
    let mut pair_idx = 0;
    let mut matrix: Vec<Vec<f64>>  = vec![vec![0.0; n]; n];
    let mut z_matrix: Vec<Vec<f64>> = vec![vec![0.0; n]; n];
    let mut pts: Vec<Vec<f64>> = vec![vec![0.0; n]; n];
    let mut team_wins:   Vec<u64> = vec![0; n];
    let mut team_draws:  Vec<u64> = vec![0; n];
    let mut team_losses: Vec<u64> = vec![0; n];

    let tournament_start = std::time::Instant::now();
    for i in 0..n {
        for j in (i + 1)..n {
            pair_idx += 1;
            let eval = evaluate_team_policies_v6(&teams[i].2, &teams[j].2, games_per_match);
            matrix[i][j] = -eval.goal_diff;
            matrix[j][i] =  eval.goal_diff;
            z_matrix[i][j] = -eval.z_score;
            z_matrix[j][i] =  eval.z_score;

            let i_wins  = eval.losses;
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

    let mut totals: Vec<(usize, f64, f64, f64)> = (0..n).map(|i| {
        let total_pts:  f64 = pts[i].iter().sum();
        let total_diff: f64 = matrix[i].iter().sum();
        let total_z:    f64 = z_matrix[i].iter().sum();
        (i, total_pts, total_diff, total_z)
    }).collect();
    totals.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap()
        .then(b.2.partial_cmp(&a.2).unwrap()));
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
    write_json_pretty(&out_dir.join("matrix.json"), &matrix_doc);

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

/// `--score-probe`: skriv ut faktisk score-fördelning för N matcher mellan två lag.
pub fn run_score_probe(project_root: &Path, team_a: &str, team_b: &str, n: usize) {
    let teams_dir = project_root.join("data").join("teams");
    let load = |name: &str| -> TeamPolicyV6 {
        let p = teams_dir.join(name).join("baseline.json");
        read_team_baseline_v6(&p)
            .unwrap_or_else(|e| panic!("cannot load {}: {}", name, e))
            .player_params
    };
    let pa = load(team_a);
    let pb = load(team_b);
    let mut rng = rand::thread_rng();
    let mut score_counts: std::collections::HashMap<(u32,u32), u32> = std::collections::HashMap::new();
    for i in 0..n {
        let _seed: u64 = rand::Rng::gen(&mut rng);
        let swap = i % 2 == 1;
        let (g0, g1) = {
            let mut g = training_engine::game::Game::new();
            let mut teams: [Box<dyn training_engine::team::Team>; 2] = [
                Box::new(training_engine::team_v6::V6Team::new(0, if swap { pb } else { pa })),
                Box::new(training_engine::team_v6::V6Team::new(1, if swap { pa } else { pb })),
            ];
            while g.phase != training_engine::game::Phase::Fulltime {
                training_engine::physics::step_game(&mut g, &mut teams, &mut rng);
            }
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
}
