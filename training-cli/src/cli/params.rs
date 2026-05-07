use serde_json::Value;

use training_engine::policy::{TeamPolicyV6, V6Params};
use training_engine::trainer::evaluate_team_policies_v6;

// ── Specs ────────────────────────────────────────────────────────────────────

pub const ALL_DECISION_PARAMS: &[(&str, f32, f32)] = &[
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

pub const ALL_SPATIAL_PARAMS: &[(&str, f32, f32)] = &[
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

pub const V6_LITE_FIELDS: &[(&str, f32, f32)] = &[
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

// ── Param-namnmappning ───────────────────────────────────────────────────────

/// JSON camelCase-fältnamn för en decision-param (snake → camel).
pub fn decision_json_field(param: &str) -> &'static str {
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

/// Kort decision-namn → full dot-path (`pass_chance_pressured` → `decisions.pass_chance_pressured`).
pub fn decision_full_path(short: &str) -> &'static str {
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParamScope { Decision, Spatial, All }

impl ParamScope {
    pub fn parse(s: &str) -> Self {
        match s {
            "spatial" | "Spatial" => ParamScope::Spatial,
            "all" | "All"         => ParamScope::All,
            _                     => ParamScope::Decision,
        }
    }
    pub fn params(self) -> Vec<(&'static str, f32, f32)> {
        let mut out: Vec<(&'static str, f32, f32)> = Vec::new();
        if matches!(self, ParamScope::Decision | ParamScope::All) {
            for (n, lo, hi) in ALL_DECISION_PARAMS {
                out.push((decision_full_path(n), *lo, *hi));
            }
        }
        if matches!(self, ParamScope::Spatial | ParamScope::All) {
            for &spec in ALL_SPATIAL_PARAMS { out.push(spec); }
        }
        out
    }
    pub fn label(self) -> &'static str {
        match self {
            ParamScope::Decision => "decision",
            ParamScope::Spatial  => "spatial",
            ParamScope::All      => "all",
        }
    }
}

// ── Setters ──────────────────────────────────────────────────────────────────

/// Sätt ett namngivet decision-param på alla 5 slots.
pub fn set_decision_param_all_slots(team: &mut TeamPolicyV6, param: &str, value: f32) {
    for slot in team.iter_mut() {
        set_decision_slot(slot, param, value);
    }
}

/// Sätt ett namngivet decision-param på en slot.
pub fn set_decision_slot(slot: &mut V6Params, param: &str, value: f32) {
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

// ── Dot-path get/set (decisions + spatial) ───────────────────────────────────

pub fn v6_get_field(p: &V6Params, path: &str) -> f32 {
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

pub fn v6_set_field(p: &mut V6Params, path: &str, value: f32) {
    fn set_min(d: &mut training_engine::policy::DistancePref, v: f32) {
        d.min = v.min(d.max);
        if d.preferred < d.min { d.preferred = d.min; }
    }
    fn set_max(d: &mut training_engine::policy::DistancePref, v: f32) {
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

/// Läs ett värde från anneal-result JSON för en slot via dot-path.
pub fn read_v6_field_from_json(slot_json: &Value, dot_path: &str) -> Option<f64> {
    let parts: Vec<&str> = dot_path.split('.').collect();
    match parts.as_slice() {
        ["decisions", field] => {
            let camel = decision_json_field(field);
            slot_json["decisions"][camel].as_f64()
        }
        ["spatial", dim, sub] => {
            let dim_camel = match *dim {
                "own_goal" => "ownGoal",
                "side"     => "side",
                "ball"     => "ball",
                "teammate" => "teammate",
                "opponent" => "opponent",
                _ => return None,
            };
            slot_json["spatial"][dim_camel][sub].as_f64()
        }
        _ => None,
    }
}

// ── Ternär ablation ──────────────────────────────────────────────────────────

/// Ternär ablation av ett fält på en slot. Returnerar antal accepterade förändringar.
pub fn ablate_v6_field_ternary(
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
