use std::path::Path;

use rand::SeedableRng;
use rand::rngs::SmallRng;

use training_engine::game::{Game, Phase};
use training_engine::physics::step_game;
use training_engine::session::{load_v7_team, read_team_baseline_v6};
use training_engine::team::Team;
use training_engine::team_v6::V6Team;

pub fn run_v7_team_test(project_root: &Path, team_name: &str, n_matches: usize) {
    let teams_dir = project_root.join("data").join("teams");
    let team_dir = teams_dir.join(team_name);
    if !team_dir.exists() {
        eprintln!("Team not found: {}", team_name);
        std::process::exit(1);
    }

    let v7_info = match load_v7_team(&team_dir, 0) {
        Ok(t) => t,
        Err(e) => { eprintln!("Cannot load V7 team: {}", e); std::process::exit(1); }
    };
    let baseline_path = team_dir.join("baseline.json");
    let v6_policy = match read_team_baseline_v6(&baseline_path) {
        Ok(b) => b.player_params,
        Err(e) => { eprintln!("Cannot load baseline: {}", e); std::process::exit(1); }
    };

    training_engine::game::CLUSTER_START.store(false, std::sync::atomic::Ordering::Relaxed);

    println!("=== V7 TEAM TEST: {} vs itself (V6 baseline) ===", team_name);
    println!("Matches: {}", n_matches);
    println!("Coachability per slot: {:?}",
        v7_info.players.iter().map(|p| format!("{:.2}", p.coachability)).collect::<Vec<_>>());
    println!("Coach style: press={:.2} depth={:.2} compact={:.2} tempo={:.2}",
        v7_info.coach.style.press_response, v7_info.coach.style.depth_response,
        v7_info.coach.style.compactness_base, v7_info.coach.style.tempo_base);
    println!();

    let mut v7_goals = 0u64;
    let mut v6_goals = 0u64;
    let mut v7_wins = 0u64;
    let mut v6_wins = 0u64;
    let mut draws = 0u64;

    for seed in 0..n_matches as u64 {
        let mut rng = SmallRng::seed_from_u64(seed * 7919);
        let mut game = Game::new();
        let swap = seed % 2 == 1;
        let v7_fresh = load_v7_team(&team_dir, if swap { 1 } else { 0 }).unwrap();
        let (v7_slot, v6_slot) = if swap { (1usize, 0usize) } else { (0, 1) };
        let mut teams: [Box<dyn Team>; 2] = if swap {
            [Box::new(V6Team::new(0, v6_policy)), Box::new(v7_fresh)]
        } else {
            [Box::new(v7_fresh), Box::new(V6Team::new(1, v6_policy))]
        };
        while game.phase != Phase::Fulltime {
            step_game(&mut game, &mut teams, &mut rng);
        }
        let (sv7, sv6) = (game.score[v7_slot], game.score[v6_slot]);
        v7_goals += sv7 as u64;
        v6_goals += sv6 as u64;
        if sv7 > sv6 { v7_wins += 1; } else if sv6 > sv7 { v6_wins += 1; } else { draws += 1; }
    }

    let n = n_matches as f64;
    println!("Results (V7 vs V6, varannan match swappad):");
    println!("  V7 goals/match: {:.3}  |  V6 goals/match: {:.3}", v7_goals as f64 / n, v6_goals as f64 / n);
    println!("  V7 wins: {}  draws: {}  V6 wins: {}", v7_wins, draws, v6_wins);
    println!("  V7 win%: {:.1}%", v7_wins as f64 / n * 100.0);
}

pub fn run_v7_tournament(project_root: &Path, games_per_match: usize) {
    let teams_dir = project_root.join("data").join("teams");
    training_engine::game::CLUSTER_START.store(false, std::sync::atomic::Ordering::Relaxed);

    let mut teams_meta: Vec<(String, usize)> = vec![];
    if let Ok(entries) = std::fs::read_dir(&teams_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() { continue; }
            if !path.join("baseline.json").exists() { continue; }
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let idx = teams_meta.len();
            teams_meta.push((name, idx));
        }
    }
    teams_meta.sort_by(|a, b| a.0.cmp(&b.0));
    let n = teams_meta.len();

    if n < 2 {
        eprintln!("Need at least 2 teams. Found {}.", n);
        std::process::exit(1);
    }

    println!("=== V7 ROUND-ROBIN TOURNAMENT ===");
    println!("Teams: {} | Games per matchup: {}", n, games_per_match);
    println!();

    let mut points: Vec<f64> = vec![0.0; n];
    let mut goals_for: Vec<f64> = vec![0.0; n];
    let mut goals_against: Vec<f64> = vec![0.0; n];
    let total_pairs = n * (n - 1) / 2;
    let mut pair_idx = 0;

    for i in 0..n {
        for j in (i + 1)..n {
            pair_idx += 1;
            let team_dir_i = teams_dir.join(&teams_meta[i].0);
            let team_dir_j = teams_dir.join(&teams_meta[j].0);

            let mut gi = 0u64;
            let mut gj = 0u64;

            for seed in 0..games_per_match as u64 {
                let mut rng = SmallRng::seed_from_u64(seed * 6271 + i as u64 * 997 + j as u64 * 31);
                let mut game = Game::new();
                let t_i = load_v7_team(&team_dir_i, 0).unwrap();
                let t_j = load_v7_team(&team_dir_j, 1).unwrap();
                let mut teams: [Box<dyn Team>; 2] = [Box::new(t_i), Box::new(t_j)];
                while game.phase != Phase::Fulltime {
                    step_game(&mut game, &mut teams, &mut rng);
                }
                gi += game.score[0] as u64;
                gj += game.score[1] as u64;
            }

            let gi_f = gi as f64 / games_per_match as f64;
            let gj_f = gj as f64 / games_per_match as f64;
            goals_for[i] += gi_f; goals_against[i] += gj_f;
            goals_for[j] += gj_f; goals_against[j] += gi_f;

            let (pi, pj) = if gi > gj { (3.0, 0.0) }
                else if gi == gj { (1.0, 1.0) }
                else { (0.0, 3.0) };
            points[i] += pi; points[j] += pj;

            println!("  [{}/{}] {:20} vs {:20}: {:.2}-{:.2} ({:.0}p-{:.0}p)",
                pair_idx, total_pairs, teams_meta[i].0, teams_meta[j].0,
                gi_f, gj_f, pi, pj);
        }
    }

    let mut ranking: Vec<usize> = (0..n).collect();
    ranking.sort_by(|&a, &b| {
        points[b].partial_cmp(&points[a]).unwrap()
            .then((goals_for[b] - goals_against[b]).partial_cmp(&(goals_for[a] - goals_against[a])).unwrap())
    });

    println!("\n=== FINAL STANDINGS ===");
    println!("{:<4} {:<22} {:>6} {:>8} {:>8}", "#", "Team", "Pts", "GF", "GA");
    for (rank, &i) in ranking.iter().enumerate() {
        println!("{:<4} {:<22} {:>6.0} {:>8.2} {:>8.2}",
            rank + 1, teams_meta[i].0, points[i], goals_for[i], goals_against[i]);
    }
}
