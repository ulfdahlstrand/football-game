use std::path::Path;

use training_engine::policy::{self, TeamPolicyV6};
use training_engine::session::read_team_baseline_v6;
use training_engine::trainer::evaluate_team_policies_v6;

use super::anneal::{run_team_anneal, ANNEAL_STAGES_QUICK};
use super::params::{
    ablate_v6_field_ternary, decision_full_path, decision_json_field,
    read_v6_field_from_json, v6_get_field, v6_set_field, ParamScope, V6_LITE_FIELDS,
};
use super::util::{iso_now, save_team_artifacts, write_json_pretty};

/// Block coordinate descent: sweep → combine → lock → repeat.
pub fn run_param_optimize(
    project_root: &Path,
    team_name: &str,
    ablation_games: usize,
    eval_games: usize,
    max_rounds: usize,
    max_better: usize,
    scope: ParamScope,
) {
    let team_dir = project_root.join("data/teams").join(team_name);
    let opt_dir  = team_dir.join(format!("param-optimize-{}", scope.label()));
    std::fs::create_dir_all(&opt_dir).expect("create param-optimize dir");

    let orig_file = read_team_baseline_v6(&team_dir.join("baseline.json")).expect("read baseline");
    let original: TeamPolicyV6 = orig_file.player_params;

    let mut working = original;
    let mut locked: Vec<String> = Vec::new();
    let mut round = 0usize;
    let mut total_point_gain = 0.0f64;
    let scope_params = scope.params();

    training_engine::game::CLUSTER_START.store(true, std::sync::atomic::Ordering::Relaxed);

    println!("=== PARAM OPTIMIZE: {} ({} scope) ===", team_name, scope.label());
    println!("Method: block coordinate descent (sweep → combine → lock → repeat)");
    println!("Ablation: {}g | Eval: {}g | Anneal: QUICK", ablation_games, eval_games);
    println!("Total params in scope: {}", scope_params.len());
    println!();

    loop {
        round += 1;
        let round_dir = opt_dir.join(format!("round-{:02}", round));
        std::fs::create_dir_all(&round_dir).expect("create round dir");

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

        let mut better_with_score: Vec<(String, f64)> = Vec::new();

        for (i, &(param, lo, hi)) in remaining.iter().enumerate() {
            let folder_name: String = param.replace('.', "__");
            let param_dir = round_dir.join(&folder_name);
            std::fs::create_dir_all(&param_dir).expect("create param dir");
            let field = param.to_string();

            println!("  [{}/{}] sweeping {}", i + 1, remaining.len(), param);

            let mut candidate = working;
            let mut accepted = 0usize;
            for slot in 0..5 {
                accepted += ablate_v6_field_ternary(
                    &mut candidate, slot, &field, lo, hi, 4, ablation_games, 1.5,
                );
            }

            let anneal_dir = param_dir.join("anneal");
            std::fs::create_dir_all(&anneal_dir).expect("create anneal dir");
            let champion = run_team_anneal(&anneal_dir, team_name, candidate, ANNEAL_STAGES_QUICK);

            let eval = evaluate_team_policies_v6(&original, &champion, eval_games);
            let pt_diff = eval.candidate_points - eval.baseline_points;
            let verdict = if pt_diff > 0.0 { "better ✓" } else if pt_diff < 0.0 { "worse" } else { "draw" };
            println!("    → ptdiff={:+.0} W{}D{}L{} | {}", pt_diff, eval.wins, eval.draws, eval.losses, verdict);

            let r = serde_json::json!({
                "param": param, "round": round,
                "ternaryAccepted": accepted,
                "pointDiff": pt_diff,
                "candidatePoints": eval.candidate_points,
                "baselinePoints": eval.baseline_points,
                "wins": eval.wins, "draws": eval.draws, "losses": eval.losses,
                "runAt": iso_now(),
            });
            write_json_pretty(&param_dir.join("result.json"), &r);

            if pt_diff > 0.0 {
                better_with_score.push((param.to_string(), pt_diff));
            }
        }

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
            write_json_pretty(&combo_dir.join("result.json"), &combo_r);

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

        let best_active: Vec<String> = (0..n)
            .filter(|i| best_label.contains(better_params[*i].as_str()))
            .map(|i| better_params[i].clone())
            .collect();

        println!("\n  ✓ Round {} winner: '{}' (ptdiff={:+.0})", round, best_label, best_pt_diff);
        println!("  Locking: {:?}", best_active);

        for p in &best_active { locked.push(p.clone()); }
        working = best_policy;
        total_point_gain += best_pt_diff;

        let round_summary = serde_json::json!({
            "round": round, "bestCombo": &best_label,
            "lockedThisRound": &best_active,
            "pointDiffVsOriginal": best_pt_diff,
            "totalPointGain": total_point_gain,
            "allLocked": &locked,
            "runAt": iso_now(),
        });
        write_json_pretty(&round_dir.join("summary.json"), &round_summary);

        println!("  Total gain so far: {:+.0} pts vs original", total_point_gain);
        println!("╚═══════════════════════════════════════════════════════════╝\n");

        if round >= max_rounds {
            println!("Reached max rounds ({}). Stopping.", max_rounds);
            break;
        }
    }

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
    write_json_pretty(&final_path, &final_doc);

    println!("=== OPTIMIZATION COMPLETE ===");
    println!("Rounds: {} | Total pt gain vs original: {:+.0}", round, total_point_gain);
    println!("Locked params: {}", if locked.is_empty() { "none".to_string() } else { locked.join(", ") });
    println!("Final policy → {}", final_path.display());
    println!("\nTo apply: copy optimized-baseline.json to baseline.json and run --v6-team-train");
}

/// Sweep: ternär ablation per parameter per slot → QUICK anneal → punkt-eval.
pub fn run_param_sweep(project_root: &Path, team_name: &str, ablation_games: usize, eval_games: usize) {
    let team_dir = project_root.join("data").join("teams").join(team_name);
    let baseline_path = team_dir.join("baseline.json");

    let baseline_file = match read_team_baseline_v6(&baseline_path) {
        Ok(b) => b,
        Err(e) => { eprintln!("Cannot read baseline for {}: {}", team_name, e); std::process::exit(1); }
    };
    let original: TeamPolicyV6 = baseline_file.player_params;

    let sweep_dir = team_dir.join("param-sweep");
    std::fs::create_dir_all(&sweep_dir).expect("create param-sweep dir");

    // Decision-params med fullständig dot-path
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

    training_engine::game::CLUSTER_START.store(true, std::sync::atomic::Ordering::Relaxed);
    const Z_ACCEPT_TERNARY: f64 = 1.5;
    const TERNARY_DEPTH: usize = 4;

    struct SweepResult {
        field: String,
        original_vals: Vec<f32>,
        ternary_vals:  Vec<f32>,
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

        let original_vals: Vec<f32> = (0..5).map(|s| v6_get_field(&original[s], field)).collect();
        println!("  original: {:?}", original_vals.iter().map(|v| format!("{:.4}", v)).collect::<Vec<_>>());

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

        println!("  running QUICK anneal from ternary result...");
        let anneal_dir = param_dir.join("anneal");
        std::fs::create_dir_all(&anneal_dir).expect("create anneal dir");
        let final_team = run_team_anneal(&anneal_dir, team_name, candidate, ANNEAL_STAGES_QUICK);

        let eval = evaluate_team_policies_v6(&original, &final_team, eval_games);
        let point_diff = eval.candidate_points - eval.baseline_points;
        let verdict = if point_diff > 0.0 { "NEW REGION BETTER" } else if point_diff < 0.0 { "ORIGINAL BETTER" } else { "DRAW" };
        println!("  eval: pts={:.0}-{:.0} W{}D{}L{} | {}",
            eval.candidate_points, eval.baseline_points,
            eval.wins, eval.draws, eval.losses, verdict);

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
        write_json_pretty(&param_dir.join("result.json"), &r);

        results.push(SweepResult {
            field: field.to_string(),
            original_vals, ternary_vals,
            point_diff,
            wins: eval.wins, draws: eval.draws, losses: eval.losses,
        });
        println!();
    }

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

/// Test alla 2^N kombinationer av sweep-förbättringar.
pub fn run_param_combine(project_root: &Path, team_name: &str, params: &[String], eval_games: usize) {
    let team_dir   = project_root.join("data/teams").join(team_name);
    let sweep_dir  = team_dir.join("param-sweep");
    let combine_dir = sweep_dir.join("combine");
    std::fs::create_dir_all(&combine_dir).expect("create combine dir");

    let orig_baseline = read_team_baseline_v6(&team_dir.join("baseline.json"))
        .expect("read original baseline");
    let original = orig_baseline.player_params;

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

    training_engine::game::CLUSTER_START.store(true, std::sync::atomic::Ordering::Relaxed);

    struct ComboResult {
        mask: usize,
        label: String,
        point_diff: f64,
        wins: u64, draws: u64, losses: u64,
    }
    let mut results: Vec<ComboResult> = Vec::new();

    for mask in 0..n_combinations {
        let active: Vec<&str> = params.iter().enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .map(|(_, p)| p.as_str())
            .collect();
        let label = if active.is_empty() { "original".to_string() } else { active.join("+") };

        println!("[{}/{}] {}", mask + 1, n_combinations, label);

        // Bygg startpolicy: original + per-slot overrides från varje vald params anneal-resultat.
        let mut start = original;
        for (i, param) in params.iter().enumerate() {
            if mask & (1 << i) == 0 { continue; }
            let jfield = decision_json_field(param);
            let anneal_slots = anneal_jsons[i]["playerParams"].as_array()
                .expect("playerParams array");
            for slot in 0..5 {
                let val = anneal_slots[slot]["decisions"][jfield]
                    .as_f64()
                    .unwrap_or_else(|| panic!("missing {} in slot {}", jfield, slot)) as f32;
                v6_set_field(&mut start[slot], decision_full_path(param), val);
            }
        }

        let run_dir = combine_dir.join(format!("{:0>width$b}-{}", mask, label, width = n));
        std::fs::create_dir_all(&run_dir).expect("create combo dir");
        let champion = run_team_anneal(&run_dir, team_name, start, ANNEAL_STAGES_QUICK);

        let eval = evaluate_team_policies_v6(&original, &champion, eval_games);
        let point_diff = eval.candidate_points - eval.baseline_points;
        println!("  → pts={:.0}-{:.0} W{}D{}L{} diff={:+.0}",
            eval.candidate_points, eval.baseline_points,
            eval.wins, eval.draws, eval.losses, point_diff);

        let r = serde_json::json!({
            "mask": mask, "label": &label,
            "activeParams": &active,
            "pointDiff": point_diff,
            "candidatePoints": eval.candidate_points,
            "baselinePoints": eval.baseline_points,
            "wins": eval.wins, "draws": eval.draws, "losses": eval.losses,
            "evalGames": eval_games, "runAt": iso_now(),
        });
        write_json_pretty(&run_dir.join("result.json"), &r);

        results.push(ComboResult { mask, label, point_diff,
            wins: eval.wins, draws: eval.draws, losses: eval.losses });
        println!();
    }

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

/// V6 rough-cal: ternär ablation över V6_LITE_FIELDS för ett färskt lag.
pub fn run_v6_rough_team(project_root: &Path, team_name: &str, team_desc: &str, max_depth: usize, games: usize) {
    training_engine::game::CLUSTER_START.store(true, std::sync::atomic::Ordering::Relaxed);
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

    save_team_artifacts(
        &team_dir, team_name, &champion,
        &format!("{}: deterministic ternary-ablation rough calibration ({} fields)", team_name, V6_LITE_FIELDS.len() * 5),
        "ternary-ablation-lite",
        1,
    );
    let _ = team_desc; // currently lookup_team_desc inside save_team_artifacts is preferred
}
