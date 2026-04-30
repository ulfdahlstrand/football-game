use rayon::prelude::*;
use rand::SeedableRng;
use rand::rngs::SmallRng;

use crate::game::{Game, Phase};
use crate::physics::step_game;
use crate::policy::{PolicyParams, TeamPolicy, TeamPolicyV3};

const WINDOW_MIN_GAMES: usize = 100;
const WINDOW_CHECK_EVERY: usize = 25;
const Z_EARLY_REJECT: f64 = 2.5;
const Z_EARLY_ACCEPT: f64 = 2.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EarlyStop {
    Worse,
    Better,
}

#[derive(Debug, Clone)]
pub struct EvalResult {
    pub games: usize,
    pub max_games: usize,
    pub elapsed_ms: u128,
    pub baseline_avg_goals: f64,
    pub candidate_avg_goals: f64,
    pub goal_diff: f64,
    pub candidate_won: bool,
    pub early_stop: Option<EarlyStop>,
    pub z_score: f64,
    pub avg_passes: f64,
    pub pass_completion_rate: f64,
    pub avg_shots: f64,
    pub avg_goals: f64,
    pub avg_tackles: f64,
    pub tackle_success_rate: f64,
    pub avg_out_of_bounds: f64,
}

#[derive(Default)]
struct GameTotals {
    baseline_goals: u64,
    candidate_goals: u64,
    passes: u64,
    pass_completed: u64,
    shots: u64,
    goals: u64,
    tackles: u64,
    tackle_success: u64,
    out_of_bounds: u64,
}

/// Plays one full match between two policies and returns (team0_score, team1_score, stats).
/// Used by the round-robin matrix.
pub fn play_match(team0: &PolicyParams, team1: &PolicyParams, seed: u64) -> (u32, u32, crate::game::Stats) {
    let mut rng = SmallRng::seed_from_u64(seed);
    let mut game = Game::new(*team0, *team1);
    while game.phase != Phase::Fulltime {
        step_game(&mut game, &mut rng);
    }
    (game.score[0], game.score[1], game.stats.clone())
}

/// `swap` = true means candidate plays as team 0 (left) instead of team 1.
/// Running half the games swapped removes any positional asymmetry.
fn run_one_game(baseline: &PolicyParams, candidate: &PolicyParams, seed: u64, swap: bool) -> (u32, u32, crate::game::Stats) {
    let mut rng = SmallRng::seed_from_u64(seed);
    let (p0, p1) = if swap { (*candidate, *baseline) } else { (*baseline, *candidate) };
    let mut game = Game::new(p0, p1);

    while game.phase != Phase::Fulltime {
        step_game(&mut game, &mut rng);
    }

    // Always return (baseline_goals, candidate_goals)
    if swap {
        (game.score[1], game.score[0], game.stats.clone())
    } else {
        (game.score[0], game.score[1], game.stats.clone())
    }
}

fn run_one_team_v3_game(baseline: &TeamPolicyV3, candidate: &TeamPolicyV3, seed: u64, swap: bool) -> (u32, u32, crate::game::Stats) {
    let mut rng = SmallRng::seed_from_u64(seed);
    let (t0, t1) = if swap { (candidate, baseline) } else { (baseline, candidate) };
    let mut game = Game::for_team_battle_v3(t0, t1);

    while game.phase != Phase::Fulltime {
        step_game(&mut game, &mut rng);
    }

    if swap {
        (game.score[1], game.score[0], game.stats.clone())
    } else {
        (game.score[0], game.score[1], game.stats.clone())
    }
}

/// Team-vs-team variant: each side has 5 per-position policies.
fn run_one_team_game(baseline: &TeamPolicy, candidate: &TeamPolicy, seed: u64, swap: bool) -> (u32, u32, crate::game::Stats) {
    let mut rng = SmallRng::seed_from_u64(seed);
    let (t0, t1) = if swap { (candidate, baseline) } else { (baseline, candidate) };
    let mut game = Game::for_team_battle(t0, t1);

    while game.phase != Phase::Fulltime {
        step_game(&mut game, &mut rng);
    }

    // Always return (baseline_goals, candidate_goals)
    if swap {
        (game.score[1], game.score[0], game.stats.clone())
    } else {
        (game.score[0], game.score[1], game.stats.clone())
    }
}

fn compute_z_score(diffs: &[f64]) -> f64 {
    let n = diffs.len();
    if n < 2 { return 0.0; }
    let mean = diffs.iter().sum::<f64>() / n as f64;
    let variance = diffs.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
    let se = (variance / n as f64).sqrt();
    if se < 1e-12 { 0.0 } else { mean / se }
}

/// v3 evaluation: same shape as `evaluate_team_policies` but uses V3Params.
pub fn evaluate_team_policies_v3(
    baseline: &TeamPolicyV3,
    candidate: &TeamPolicyV3,
    max_games: usize,
) -> EvalResult {
    let start = std::time::Instant::now();
    let mut totals = GameTotals::default();
    let mut diffs: Vec<f64> = Vec::with_capacity(max_games);
    let mut early_stop: Option<EarlyStop> = None;

    let mut games_run = 0usize;
    let mut chunk_start_seed = rand::random::<u64>();

    while games_run < max_games {
        let remaining = max_games - games_run;
        let chunk_size = WINDOW_CHECK_EVERY.min(remaining);

        let seeds: Vec<(u64, bool)> = (0..chunk_size)
            .map(|i| {
                let seed = chunk_start_seed.wrapping_add(i as u64);
                let swap = (games_run + i) % 2 == 1;
                (seed, swap)
            })
            .collect();
        chunk_start_seed = chunk_start_seed.wrapping_add(chunk_size as u64);

        let chunk_results: Vec<(u32, u32, crate::game::Stats)> = seeds
            .into_par_iter()
            .map(|(seed, swap)| run_one_team_v3_game(baseline, candidate, seed, swap))
            .collect();

        for (b_goals, c_goals, stats) in chunk_results {
            totals.baseline_goals += b_goals as u64;
            totals.candidate_goals += c_goals as u64;
            totals.passes += stats.passes as u64;
            totals.pass_completed += stats.pass_completed as u64;
            totals.shots += stats.shots as u64;
            totals.goals += stats.goals as u64;
            totals.tackles += stats.tackles as u64;
            totals.tackle_success += stats.tackle_success as u64;
            totals.out_of_bounds += stats.out_of_bounds as u64;
            diffs.push(c_goals as f64 - b_goals as f64);
        }

        games_run += chunk_size;

        if games_run >= WINDOW_MIN_GAMES {
            let z = compute_z_score(&diffs);
            if z < -(Z_EARLY_REJECT) { early_stop = Some(EarlyStop::Worse); break; }
            if z > Z_EARLY_ACCEPT { early_stop = Some(EarlyStop::Better); break; }
        }
    }

    let n = games_run as f64;
    let elapsed_ms = start.elapsed().as_millis();
    let goal_diff = totals.candidate_goals as f64 - totals.baseline_goals as f64;
    let z_score = compute_z_score(&diffs);

    EvalResult {
        games: games_run,
        max_games,
        elapsed_ms,
        baseline_avg_goals: (totals.baseline_goals as f64 / n * 1000.0).round() / 1000.0,
        candidate_avg_goals: (totals.candidate_goals as f64 / n * 1000.0).round() / 1000.0,
        goal_diff: (goal_diff * 1000.0).round() / 1000.0,
        candidate_won: goal_diff > 0.0,
        early_stop,
        z_score: (z_score * 1000.0).round() / 1000.0,
        avg_passes: (totals.passes as f64 / n * 100.0).round() / 100.0,
        pass_completion_rate: if totals.passes > 0 {
            (totals.pass_completed as f64 / totals.passes as f64 * 1000.0).round() / 1000.0
        } else { 0.0 },
        avg_shots: (totals.shots as f64 / n * 100.0).round() / 100.0,
        avg_goals: (totals.goals as f64 / n * 100.0).round() / 100.0,
        avg_tackles: (totals.tackles as f64 / n * 100.0).round() / 100.0,
        tackle_success_rate: if totals.tackles > 0 {
            (totals.tackle_success as f64 / totals.tackles as f64 * 1000.0).round() / 1000.0
        } else { 0.0 },
        avg_out_of_bounds: (totals.out_of_bounds as f64 / n * 100.0).round() / 100.0,
    }
}

/// Same evaluation logic as `evaluate_policies` but for v2 team-vs-team
/// matchups where each side has 5 per-position policies.
pub fn evaluate_team_policies(
    baseline: &TeamPolicy,
    candidate: &TeamPolicy,
    max_games: usize,
) -> EvalResult {
    let start = std::time::Instant::now();
    let mut totals = GameTotals::default();
    let mut diffs: Vec<f64> = Vec::with_capacity(max_games);
    let mut early_stop: Option<EarlyStop> = None;

    let mut games_run = 0usize;
    let mut chunk_start_seed = rand::random::<u64>();

    while games_run < max_games {
        let remaining = max_games - games_run;
        let chunk_size = WINDOW_CHECK_EVERY.min(remaining);

        let seeds: Vec<(u64, bool)> = (0..chunk_size)
            .map(|i| {
                let seed = chunk_start_seed.wrapping_add(i as u64);
                let swap = (games_run + i) % 2 == 1;
                (seed, swap)
            })
            .collect();
        chunk_start_seed = chunk_start_seed.wrapping_add(chunk_size as u64);

        let chunk_results: Vec<(u32, u32, crate::game::Stats)> = seeds
            .into_par_iter()
            .map(|(seed, swap)| run_one_team_game(baseline, candidate, seed, swap))
            .collect();

        for (b_goals, c_goals, stats) in chunk_results {
            totals.baseline_goals += b_goals as u64;
            totals.candidate_goals += c_goals as u64;
            totals.passes += stats.passes as u64;
            totals.pass_completed += stats.pass_completed as u64;
            totals.shots += stats.shots as u64;
            totals.goals += stats.goals as u64;
            totals.tackles += stats.tackles as u64;
            totals.tackle_success += stats.tackle_success as u64;
            totals.out_of_bounds += stats.out_of_bounds as u64;
            diffs.push(c_goals as f64 - b_goals as f64);
        }

        games_run += chunk_size;

        if games_run >= WINDOW_MIN_GAMES {
            let z = compute_z_score(&diffs);
            if z < -(Z_EARLY_REJECT) {
                early_stop = Some(EarlyStop::Worse);
                break;
            }
            if z > Z_EARLY_ACCEPT {
                early_stop = Some(EarlyStop::Better);
                break;
            }
        }
    }

    let n = games_run as f64;
    let elapsed_ms = start.elapsed().as_millis();
    let goal_diff = totals.candidate_goals as f64 - totals.baseline_goals as f64;
    let z_score = compute_z_score(&diffs);

    EvalResult {
        games: games_run,
        max_games,
        elapsed_ms,
        baseline_avg_goals: (totals.baseline_goals as f64 / n * 1000.0).round() / 1000.0,
        candidate_avg_goals: (totals.candidate_goals as f64 / n * 1000.0).round() / 1000.0,
        goal_diff: (goal_diff * 1000.0).round() / 1000.0,
        candidate_won: goal_diff > 0.0,
        early_stop,
        z_score: (z_score * 1000.0).round() / 1000.0,
        avg_passes: (totals.passes as f64 / n * 100.0).round() / 100.0,
        pass_completion_rate: if totals.passes > 0 {
            (totals.pass_completed as f64 / totals.passes as f64 * 1000.0).round() / 1000.0
        } else { 0.0 },
        avg_shots: (totals.shots as f64 / n * 100.0).round() / 100.0,
        avg_goals: (totals.goals as f64 / n * 100.0).round() / 100.0,
        avg_tackles: (totals.tackles as f64 / n * 100.0).round() / 100.0,
        tackle_success_rate: if totals.tackles > 0 {
            (totals.tackle_success as f64 / totals.tackles as f64 * 1000.0).round() / 1000.0
        } else { 0.0 },
        avg_out_of_bounds: (totals.out_of_bounds as f64 / n * 100.0).round() / 100.0,
    }
}

pub fn evaluate_policies(
    baseline: &PolicyParams,
    candidate: &PolicyParams,
    max_games: usize,
) -> EvalResult {
    let start = std::time::Instant::now();
    let mut totals = GameTotals::default();
    let mut diffs: Vec<f64> = Vec::with_capacity(max_games);
    let mut early_stop: Option<EarlyStop> = None;

    let mut games_run = 0usize;
    let mut chunk_start_seed = rand::random::<u64>();

    while games_run < max_games {
        let remaining = max_games - games_run;
        let chunk_size = WINDOW_CHECK_EVERY.min(remaining);

        let seeds: Vec<(u64, bool)> = (0..chunk_size)
            .map(|i| {
                let seed = chunk_start_seed.wrapping_add(i as u64);
                let swap = (games_run + i) % 2 == 1;
                (seed, swap)
            })
            .collect();
        chunk_start_seed = chunk_start_seed.wrapping_add(chunk_size as u64);

        let chunk_results: Vec<(u32, u32, crate::game::Stats)> = seeds
            .into_par_iter()
            .map(|(seed, swap)| run_one_game(baseline, candidate, seed, swap))
            .collect();

        for (b_goals, c_goals, stats) in chunk_results {
            totals.baseline_goals += b_goals as u64;
            totals.candidate_goals += c_goals as u64;
            totals.passes += stats.passes as u64;
            totals.pass_completed += stats.pass_completed as u64;
            totals.shots += stats.shots as u64;
            totals.goals += stats.goals as u64;
            totals.tackles += stats.tackles as u64;
            totals.tackle_success += stats.tackle_success as u64;
            totals.out_of_bounds += stats.out_of_bounds as u64;
            diffs.push(c_goals as f64 - b_goals as f64);
        }

        games_run += chunk_size;

        if games_run >= WINDOW_MIN_GAMES {
            let z = compute_z_score(&diffs);
            if z < -(Z_EARLY_REJECT) {
                early_stop = Some(EarlyStop::Worse);
                break;
            }
            if z > Z_EARLY_ACCEPT {
                early_stop = Some(EarlyStop::Better);
                break;
            }
        }
    }

    let n = games_run as f64;
    let elapsed_ms = start.elapsed().as_millis();
    let goal_diff = totals.candidate_goals as f64 - totals.baseline_goals as f64;
    let z_score = compute_z_score(&diffs);

    EvalResult {
        games: games_run,
        max_games,
        elapsed_ms,
        baseline_avg_goals: (totals.baseline_goals as f64 / n * 1000.0).round() / 1000.0,
        candidate_avg_goals: (totals.candidate_goals as f64 / n * 1000.0).round() / 1000.0,
        goal_diff: (goal_diff * 1000.0).round() / 1000.0,
        candidate_won: goal_diff > 0.0,
        early_stop,
        z_score: (z_score * 1000.0).round() / 1000.0,
        avg_passes: (totals.passes as f64 / n * 100.0).round() / 100.0,
        pass_completion_rate: if totals.passes > 0 {
            (totals.pass_completed as f64 / totals.passes as f64 * 1000.0).round() / 1000.0
        } else { 0.0 },
        avg_shots: (totals.shots as f64 / n * 100.0).round() / 100.0,
        avg_goals: (totals.goals as f64 / n * 100.0).round() / 100.0,
        avg_tackles: (totals.tackles as f64 / n * 100.0).round() / 100.0,
        tackle_success_rate: if totals.tackles > 0 {
            (totals.tackle_success as f64 / totals.tackles as f64 * 1000.0).round() / 1000.0
        } else { 0.0 },
        avg_out_of_bounds: (totals.out_of_bounds as f64 / n * 100.0).round() / 100.0,
    }
}
