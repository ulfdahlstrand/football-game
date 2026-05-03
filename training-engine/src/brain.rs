use rand::Rng;

use crate::game::Game;
use crate::policy::{PolicyParams, V3Params, V4Params};

/// Optional hooks that classic_tick reads to alter behavior for v4 brains.
/// V1/V2/V3 brains pass the default (no-op) and behave exactly as before.
#[derive(Clone, Copy, Debug)]
pub struct TickHooks {
    /// Multipliers applied to the pass-chance based on best-target direction:
    /// [offensive, defensive, neutral]. 1.0 each = unchanged classic behavior.
    pub pass_dir_mult: [f32; 3],
    /// 0..1. 0 = goalkeeper locked to goal line (classic). 1 = GK roams up
    /// to half-line, following the ball.
    pub gk_freedom: f32,
}

impl Default for TickHooks {
    fn default() -> Self {
        Self { pass_dir_mult: [1.0, 1.0, 1.0], gk_freedom: 0.0 }
    }
}

/// Each player carries a brain that determines BOTH their parameters AND the
/// algorithm used to make decisions. This lets v1, v2 and v3 players coexist
/// in the same match.
///
/// - `V1`: legacy team-level params, classic decision algorithm
/// - `V2`: per-position params, classic decision algorithm
/// - `V3`: per-position V3Params, new algorithm with modulators
///
/// V1 and V2 share `classic_tick()` because they only differ in WHERE the
/// params come from, not what's done with them.
#[derive(Clone, Copy, Debug)]
pub enum PlayerBrain {
    V1(PolicyParams),
    V2(PolicyParams),
    V3(V3Params),
    V4(V4Params),
}

impl PlayerBrain {
    /// The classic params underlying this brain. V3 falls back to its base.
    pub fn base_params(&self) -> PolicyParams {
        match self {
            PlayerBrain::V1(p) | PlayerBrain::V2(p) => *p,
            PlayerBrain::V3(p) => p.base,
            PlayerBrain::V4(p) => p.v3.base,
        }
    }

    pub fn version_label(&self) -> &'static str {
        match self {
            PlayerBrain::V1(_) => "v1",
            PlayerBrain::V2(_) => "v2",
            PlayerBrain::V3(_) => "v3",
            PlayerBrain::V4(_) => "v4",
        }
    }
}

impl Default for PlayerBrain {
    fn default() -> Self {
        PlayerBrain::V1(PolicyParams::default())
    }
}

/// Dispatch entry point — called once per CPU player per tick.
pub fn tick_player(game: &mut Game, player_idx: usize, rng: &mut impl Rng) {
    let brain = game.pl[player_idx].brain;
    match brain {
        PlayerBrain::V1(p) | PlayerBrain::V2(p) => {
            crate::ai::classic_tick(game, player_idx, &p, &TickHooks::default(), rng);
        }
        PlayerBrain::V3(p) => {
            v3_tick(game, player_idx, &p, rng);
        }
        PlayerBrain::V4(p) => {
            v4_tick(game, player_idx, &p, rng);
        }
    }
}

/// V3 algorithm: classic decision tree + spatial-aware modulations.
///
/// We compute a `SpatialFeatures` snapshot for the current player, then use
/// it together with `V3Params` weights to derive a *dynamic* PolicyParams
/// for this single tick. Classic_tick then runs as usual on those modulated
/// params. This keeps the algorithm shape identical to v1/v2 while letting
/// v3 react to context that v1/v2 are blind to.
pub fn v3_tick(game: &mut Game, player_idx: usize, params: &V3Params, rng: &mut impl Rng) {
    let feats = crate::spatial::compute_features_with(
        game, player_idx, params.pressure_radius, params.block_distance,
    );
    let mut p = params.base;

    // 1. Aggression — global tackle scale, plus zone-specific multiplier
    let zone_mult = params.zone_aggression[feats.zone.index()];
    p.tackle_chance = (p.tackle_chance * params.aggression * zone_mult).clamp(0.01, 0.5);

    // 2. Risk appetite — shoot more eagerly with higher risk
    let risk = params.risk_appetite.clamp(0.0, 1.0);
    p.shoot_progress_threshold = (p.shoot_progress_threshold - 0.05 * (risk - 0.5)).clamp(0.5, 0.95);

    // 3. Pressure-aware passing — under heavy pressure increase pass willingness
    let pressure_factor = (feats.opp_within_pressure as f32).min(3.0) / 3.0;
    p.pass_chance_pressured = (p.pass_chance_pressured * (1.0 + 0.5 * pressure_factor)).clamp(0.02, 0.6);

    // 4. Clear-shot bonus — if the lane to goal is open and we're forward-ish,
    //    drop the shoot threshold further to take the shot.
    if !feats.direct_shot_blocked && params.clear_shot_bonus > 0.0
        && (feats.zone == crate::spatial::FieldZone::OppHalf
            || feats.zone == crate::spatial::FieldZone::OppPenaltyArea)
    {
        p.shoot_progress_threshold =
            (p.shoot_progress_threshold - 0.10 * params.clear_shot_bonus).clamp(0.4, 0.95);
    }

    // 5. Block-avoidance — when many opponents block lanes, raise forward
    //    pass minimum gain so we don't try long passes into traffic.
    if params.block_avoidance > 0.0 && feats.lane_to_ball_blockers >= 1 {
        let bump = 4.0 * params.block_avoidance * feats.lane_to_ball_blockers as f32;
        p.forward_pass_min_gain = (p.forward_pass_min_gain + bump).clamp(0.0, 30.0);
    }

    // 6. Edge avoidance — when very close to an edge while not in own half,
    //    tighten mark distance (keep formation tighter).
    if params.edge_avoidance > 0.0 && feats.dist_nearest_edge < 60.0 {
        let pull = params.edge_avoidance * (1.0 - feats.dist_nearest_edge / 60.0);
        p.mark_distance = (p.mark_distance * (1.0 - 0.25 * pull)).clamp(20.0, 90.0);
    }

    let _ = (params.vision_radius, params.chemistry_target_id, params.chemistry_bonus,
             params.corridor_preference, params.goal_attraction);
    // Vision/chemistry/corridor/goal-attraction will hook into classic_tick
    // and cpu_find_pass once we wire feature-aware pass scoring.

    crate::ai::classic_tick(game, player_idx, &p, &TickHooks::default(), rng);
}

/// V4 algorithm: v3 spatial logic + directional pass multipliers + GK freedom.
pub fn v4_tick(game: &mut Game, player_idx: usize, params: &V4Params, rng: &mut impl Rng) {
    // Reuse all of v3's modulation by computing the modulated PolicyParams the
    // same way v3_tick does, then call classic_tick with v4-specific hooks.
    let v3p = &params.v3;
    let feats = crate::spatial::compute_features_with(
        game, player_idx, v3p.pressure_radius, v3p.block_distance,
    );
    let mut p = v3p.base;

    // (same modulations as v3_tick — kept in sync intentionally)
    let zone_mult = v3p.zone_aggression[feats.zone.index()];
    p.tackle_chance = (p.tackle_chance * v3p.aggression * zone_mult).clamp(0.01, 0.5);
    let risk = v3p.risk_appetite.clamp(0.0, 1.0);
    p.shoot_progress_threshold = (p.shoot_progress_threshold - 0.05 * (risk - 0.5)).clamp(0.5, 0.95);
    let pressure_factor = (feats.opp_within_pressure as f32).min(3.0) / 3.0;
    p.pass_chance_pressured = (p.pass_chance_pressured * (1.0 + 0.5 * pressure_factor)).clamp(0.02, 0.6);
    if !feats.direct_shot_blocked && v3p.clear_shot_bonus > 0.0
        && (feats.zone == crate::spatial::FieldZone::OppHalf
            || feats.zone == crate::spatial::FieldZone::OppPenaltyArea)
    {
        p.shoot_progress_threshold =
            (p.shoot_progress_threshold - 0.10 * v3p.clear_shot_bonus).clamp(0.4, 0.95);
    }
    if v3p.block_avoidance > 0.0 && feats.lane_to_ball_blockers >= 1 {
        let bump = 4.0 * v3p.block_avoidance * feats.lane_to_ball_blockers as f32;
        p.forward_pass_min_gain = (p.forward_pass_min_gain + bump).clamp(0.0, 30.0);
    }
    if v3p.edge_avoidance > 0.0 && feats.dist_nearest_edge < 60.0 {
        let pull = v3p.edge_avoidance * (1.0 - feats.dist_nearest_edge / 60.0);
        p.mark_distance = (p.mark_distance * (1.0 - 0.25 * pull)).clamp(20.0, 90.0);
    }

    let hooks = TickHooks {
        pass_dir_mult: [
            params.pass_dir_offensive.clamp(0.0, 2.0),
            params.pass_dir_defensive.clamp(0.0, 2.0),
            params.pass_dir_neutral.clamp(0.0, 2.0),
        ],
        gk_freedom: params.gk_freedom.clamp(0.0, 1.0),
    };
    crate::ai::classic_tick(game, player_idx, &p, &hooks, rng);
}
