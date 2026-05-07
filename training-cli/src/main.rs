mod cli;

use std::path::Path;

use training_engine::policy;

use cli::anneal::{AnnealVariant, SingleStageMode};
use cli::params::ParamScope;
use cli::teams::lookup_team_desc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("");

    fn parse_variant(args: &[String]) -> AnnealVariant {
        if args.iter().any(|s| s == "--full") { AnnealVariant::Full }
        else if args.iter().any(|s| s == "--quick") { AnnealVariant::Quick }
        else { AnnealVariant::Short }
    }

    fn arg<T: std::str::FromStr>(args: &[String], i: usize, default: T) -> T {
        args.get(i).and_then(|s| s.parse().ok()).unwrap_or(default)
    }

    fn arg_str(args: &[String], i: usize, default: &str) -> String {
        args.get(i).cloned().unwrap_or_else(|| default.to_string())
    }

    match cmd {
        "--v6" => {
            let v6_dir = project_root.join("data").join("policies").join("v6");
            let epochs: usize = arg(&args, 2, 100);
            let games_per_epoch: usize = arg(&args, 3, 1000);
            let session_name = arg_str(&args, 4, "session-1");
            cli::training::run_v6_training(&v6_dir, epochs, games_per_epoch, &session_name);
        }

        "--v6-population" => {
            let num_teams: usize = arg(&args, 2, 10);
            let variant = parse_variant(&args);
            let skip: usize = args.iter().position(|s| s == "--skip")
                .and_then(|i| args.get(i + 1)).and_then(|s| s.parse().ok()).unwrap_or(0);
            cli::training::run_v6_population(project_root, num_teams, variant, skip);
        }

        "--v6-rough-team" => {
            let team_name = arg_str(&args, 2, "forge-fc");
            let max_depth: usize = arg(&args, 3, 3);
            let games: usize = arg(&args, 4, 1000);
            let desc = lookup_team_desc(&team_name);
            cli::optimize::run_v6_rough_team(project_root, &team_name, desc, max_depth, games);
        }

        "--score-probe" => {
            let team_a = arg_str(&args, 2, "granite-athletic");
            let team_b = arg_str(&args, 3, "nebula-rangers");
            let n: usize = arg(&args, 4, 100);
            cli::tournament::run_score_probe(project_root, &team_a, &team_b, n);
        }

        "--v6-tournament" => {
            let games_per_match: usize = arg(&args, 2, 10000);
            cli::tournament::run_v6_tournament(project_root, games_per_match);
        }

        "--param-sweep" => {
            let team_name = arg_str(&args, 2, "tempest-united");
            let ablation_games: usize = arg(&args, 3, 500);
            let eval_games: usize = arg(&args, 4, 1000);
            cli::optimize::run_param_sweep(project_root, &team_name, ablation_games, eval_games);
        }

        "--param-optimize" => {
            let team_name = arg_str(&args, 2, "tempest-united");
            let ablation_games: usize = arg(&args, 3, 500);
            let eval_games: usize = arg(&args, 4, 1000);
            let max_rounds: usize = arg(&args, 5, 3);
            let max_better: usize = arg(&args, 6, 3);
            let scope = args.get(7).map(|s| ParamScope::parse(s)).unwrap_or(ParamScope::Decision);
            cli::optimize::run_param_optimize(project_root, &team_name, ablation_games, eval_games, max_rounds, max_better, scope);
        }

        "--param-combine" => {
            let team_name = arg_str(&args, 2, "tempest-united");
            let eval_games: usize = arg(&args, 3, 1000);
            let params_arg = arg_str(&args, 4, "mark_distance,pass_chance_pressured,pass_chance_forward");
            let params: Vec<String> = params_arg.split(',').map(|s| s.trim().to_string()).collect();
            cli::optimize::run_param_combine(project_root, &team_name, &params, eval_games);
        }

        "--v6-team-svgs" => {
            cli::layout::regenerate_all_team_svgs(project_root);
        }

        "--single-stage-slot" => {
            let team_name = arg_str(&args, 2, "granite-athletic");
            let slot: usize = arg(&args, 3, 0);
            let epochs: usize = arg(&args, 4, 2000);
            let games: usize = arg(&args, 5, 200);
            if slot > 4 { eprintln!("slot must be 0..=4"); std::process::exit(1); }
            cli::anneal::run_single_stage(project_root, &team_name, SingleStageMode::SlotOnly(slot), epochs, games);
        }

        "--single-stage-gk" => {
            let team_name = arg_str(&args, 2, "granite-athletic");
            let epochs: usize = arg(&args, 3, 2000);
            let games: usize = arg(&args, 4, 200);
            cli::anneal::run_single_stage(project_root, &team_name, SingleStageMode::GkOnly, epochs, games);
        }

        "--single-stage" => {
            let team_name = arg_str(&args, 2, "granite-athletic");
            let epochs: usize = arg(&args, 3, 100);
            let games: usize = arg(&args, 4, 500);
            cli::anneal::run_single_stage(project_root, &team_name, SingleStageMode::Full, epochs, games);
        }

        "--v6-team-train" => {
            let team_name = arg_str(&args, 2, "granite-athletic");
            let variant = parse_variant(&args);
            cli::training::run_v6_team_train(project_root, &team_name, variant);
        }

        "--gk-train" => {
            let epochs: usize = arg(&args, 2, 100);
            let games_per_epoch: usize = arg(&args, 3, 1_000_000);
            cli::training::run_gk_train_all(project_root, epochs, games_per_epoch);
        }

        "--v6-test-continue" => {
            training_engine::game::CLUSTER_START.store(true, std::sync::atomic::Ordering::Relaxed);
            let v6_dir = project_root.join("data").join("policies").join("v6");
            let epochs: usize = arg(&args, 2, 50);
            let games_per_epoch: usize = arg(&args, 3, 10000);
            let session_name = arg_str(&args, 4, "test-cont");
            println!("[v6-test-continue] CLUSTER_START enabled — continuing from current v6 baseline");
            cli::training::run_v6_training(&v6_dir, epochs, games_per_epoch, &session_name);
        }

        "--v6-test" => {
            training_engine::game::CLUSTER_START.store(true, std::sync::atomic::Ordering::Relaxed);
            let v6_dir = project_root.join("data").join("policies").join("v6");
            let epochs: usize = arg(&args, 2, 50);
            let games_per_epoch: usize = arg(&args, 3, 1000);

            std::fs::create_dir_all(&v6_dir).expect("create v6 dir");
            let baseline_path = v6_dir.join("baseline.json");
            if baseline_path.exists() {
                let _ = std::fs::rename(&baseline_path, v6_dir.join("baseline-prev.json"));
            }
            let fresh: [policy::V6Params; 5] = [0,1,2,3,4].map(policy::v6_default_for_slot);
            let bootstrap = serde_json::json!({
                "name": "v6-baseline", "version": 1,
                "type": "team-policy-v6",
                "description": "Fresh v6 defaults seeded by --v6-test for clustered positioning experiment.",
                "playerParams": fresh,
            });
            cli::util::write_json_pretty(&baseline_path, &bootstrap);
            let _ = std::fs::remove_dir_all(v6_dir.join("sessions").join("test"));
            let _ = std::fs::remove_file(v6_dir.join("baseline-genesis.json"));

            println!("[v6-test] CLUSTER_START enabled — all field players begin at FW*0.25 / FW*0.75, y=H2");
            println!("[v6-test] v6 baseline reset to per-slot defaults; running clean session 'test'");
            cli::training::run_v6_training(&v6_dir, epochs, games_per_epoch, "test");
        }

        "--v7-team-test" => {
            let team_name = arg_str(&args, 2, "aurora-fc");
            let n_matches: usize = arg(&args, 3, 100);
            cli::v7::run_v7_team_test(project_root, &team_name, n_matches);
        }

        "--v7-tournament" => {
            let games_per_match: usize = arg(&args, 2, 1000);
            cli::v7::run_v7_tournament(project_root, games_per_match);
        }

        _ => {
            eprintln!("Unknown command. Use --v6-team-train, --single-stage, --v6-tournament, --v7-team-test, --v7-tournament, etc.");
            std::process::exit(1);
        }
    }
}
