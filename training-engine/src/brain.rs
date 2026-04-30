use rand::Rng;

use crate::game::Game;
use crate::policy::{PolicyParams, V3Params};

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
}

impl PlayerBrain {
    /// The classic params underlying this brain. V3 falls back to its base.
    pub fn base_params(&self) -> PolicyParams {
        match self {
            PlayerBrain::V1(p) | PlayerBrain::V2(p) => *p,
            PlayerBrain::V3(p) => p.base,
        }
    }

    pub fn version_label(&self) -> &'static str {
        match self {
            PlayerBrain::V1(_) => "v1",
            PlayerBrain::V2(_) => "v2",
            PlayerBrain::V3(_) => "v3",
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
            crate::ai::classic_tick(game, player_idx, &p, rng);
        }
        PlayerBrain::V3(p) => {
            v3_tick(game, player_idx, &p, rng);
        }
    }
}

/// V3 algorithm: today it reuses the classic algorithm but with `aggression`
/// applied as a multiplicative scale on tackle_chance, and `risk_appetite`
/// scaling shoot/pass thresholds. The hooks for `vision_radius` and
/// `chemistry_*` are present in the params struct and ready for future logic.
///
/// This is intentionally an incremental v3 — we have the architecture in
/// place so v3 can diverge further as new strategies are explored.
pub fn v3_tick(game: &mut Game, player_idx: usize, params: &V3Params, rng: &mut impl Rng) {
    // Apply aggression / risk modulation to a temp PolicyParams instance,
    // then dispatch to classic_tick. This is the simplest divergence path
    // and will let us A/B test v2 vs v3 immediately.
    let mut p = params.base;
    p.tackle_chance = (p.tackle_chance * params.aggression).clamp(0.01, 0.5);
    let risk = params.risk_appetite.clamp(0.0, 1.0);
    // Higher risk => slightly lower threshold needed to attempt a shot
    p.shoot_progress_threshold = (p.shoot_progress_threshold - 0.05 * (risk - 0.5)).clamp(0.5, 0.95);
    crate::ai::classic_tick(game, player_idx, &p, rng);
}
