use rand::Rng;
use rand_distr::Normal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyParams {
    pub pass_chance_pressured: f32,
    pub pass_chance_wing: f32,
    pub pass_chance_forward: f32,
    pub pass_chance_default: f32,
    pub shoot_progress_threshold: f32,
    pub tackle_chance: f32,
    pub forward_pass_min_gain: f32,
    pub mark_distance: f32,
}

impl Default for PolicyParams {
    fn default() -> Self {
        Self {
            pass_chance_pressured: 0.16,
            pass_chance_wing: 0.07,
            pass_chance_forward: 0.04,
            pass_chance_default: 0.055,
            shoot_progress_threshold: 0.76,
            tackle_chance: 0.08,
            forward_pass_min_gain: 8.0,
            mark_distance: 48.0,
        }
    }
}

struct ParamBounds {
    lo: f32,
    hi: f32,
    scale: f32,
    integer: bool,
}

const BOUNDS: [(&str, ParamBounds); 8] = [
    ("pass_chance_pressured", ParamBounds { lo: 0.02, hi: 0.4,  scale: 0.035, integer: false }),
    ("pass_chance_wing",      ParamBounds { lo: 0.01, hi: 0.25, scale: 0.025, integer: false }),
    ("pass_chance_forward",   ParamBounds { lo: 0.005,hi: 0.18, scale: 0.018, integer: false }),
    ("pass_chance_default",   ParamBounds { lo: 0.005,hi: 0.2,  scale: 0.018, integer: false }),
    ("shoot_progress",        ParamBounds { lo: 0.55, hi: 0.9,  scale: 0.035, integer: false }),
    ("tackle_chance",         ParamBounds { lo: 0.01, hi: 0.22, scale: 0.025, integer: false }),
    ("forward_pass_min_gain", ParamBounds { lo: 0.0,  hi: 18.0, scale: 2.0,   integer: true  }),
    ("mark_distance",         ParamBounds { lo: 25.0, hi: 85.0, scale: 5.0,   integer: true  }),
];

fn clamp(v: f32, lo: f32, hi: f32) -> f32 {
    v.max(lo).min(hi)
}

fn round4(v: f32) -> f32 {
    (v * 10000.0).round() / 10000.0
}

/// `scale` multiplies the Gaussian std-dev for each parameter.
/// Values below 1.0 produce more conservative mutations (local search).
pub fn mutate(p: &PolicyParams, rng: &mut impl Rng, scale: f32) -> PolicyParams {
    let mut next = *p;
    let scale = scale.max(0.05).min(2.0);

    macro_rules! maybe_mutate {
        ($field:ident, $idx:expr) => {
            // 50% skip probability → ~4 params changed on average instead of ~5.6
            if rng.gen::<f32>() > 0.5 {
                let b = &BOUNDS[$idx].1;
                let dist = Normal::new(0.0f32, b.scale * scale).unwrap();
                let delta: f32 = rng.sample(dist);
                let v = clamp(next.$field + delta, b.lo, b.hi);
                next.$field = if b.integer { v.round() } else { round4(v) };
            }
        };
    }

    maybe_mutate!(pass_chance_pressured,   0);
    maybe_mutate!(pass_chance_wing,        1);
    maybe_mutate!(pass_chance_forward,     2);
    maybe_mutate!(pass_chance_default,     3);
    maybe_mutate!(shoot_progress_threshold,4);
    maybe_mutate!(tackle_chance,           5);
    maybe_mutate!(forward_pass_min_gain,   6);
    maybe_mutate!(mark_distance,           7);

    next
}

/// V3 introduces additional params layered on top of the classic v1/v2
/// PolicyParams. v3 players use spatial features (edge distance, zones,
/// pass-lane blockers, etc.) modulated by these weights.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V3Params {
    pub base: PolicyParams,

    // ─── Behavioral modulators ───────────────────────────────────────────
    /// 0..2, scales tackle_chance. 1.0 = classic behavior.
    pub aggression: f32,
    /// Pass-search radius in pixels. f32::INFINITY = consider all teammates.
    pub vision_radius: f32,
    /// 0..1, willingness to attempt risky passes/shots.
    pub risk_appetite: f32,
    /// Bias multiplier for cpu_find_pass score toward this teammate id.
    /// `chemistry_target_id < 0` disables the bonus.
    pub chemistry_target_id: i32,
    pub chemistry_bonus: f32,

    // ─── Spatial-awareness weights ───────────────────────────────────────
    /// 0..1. Higher = repel more strongly from edges when not on the ball.
    pub edge_avoidance: f32,
    /// Pixel radius at which an opponent counts as "pressuring" us.
    /// Used to reweight pass-vs-carry decisions and tackle willingness.
    pub pressure_radius: f32,
    /// 0..1. Pull toward opp goal when team has the ball but we don't.
    pub goal_attraction: f32,
    /// -1=top, 0=center, 1=bottom — preferred horizontal corridor.
    /// Used as soft attractor for off-ball positioning.
    pub corridor_preference: i32,
    /// Per-FieldZone tackle_chance multiplier.
    /// Index order: [OwnPenaltyArea, OwnHalf, Midfield, OppHalf, OppPenaltyArea].
    pub zone_aggression: [f32; 5],
    /// 0..1. How strongly we avoid sending passes through blocked lanes.
    /// Higher = more conservative, prefers safer routes.
    pub block_avoidance: f32,
    /// Pixel block-distance threshold for considering a lane blocked.
    pub block_distance: f32,
    /// 0..1. When true (>0), reduces shoot threshold if direct shot lane
    /// to goal is open.
    pub clear_shot_bonus: f32,
}

impl Default for V3Params {
    fn default() -> Self {
        Self {
            base: PolicyParams::default(),
            aggression: 1.0,
            vision_radius: f32::INFINITY,
            risk_appetite: 0.5,
            chemistry_target_id: -1,
            chemistry_bonus: 0.0,

            edge_avoidance: 0.0,
            pressure_radius: 72.0,
            goal_attraction: 0.0,
            corridor_preference: 0,
            zone_aggression: [1.0; 5],
            block_avoidance: 0.5,
            block_distance: 25.0,
            clear_shot_bonus: 0.0,
        }
    }
}

/// A team policy is 5 player slots: [fwd, mid_top, mid_bottom, def, gk].
/// Slot index = player.id % 5 for both teams.
pub type TeamPolicy = [PolicyParams; 5];

/// v3 team policy: 5 V3Params, one per slot.
pub type TeamPolicyV3 = [V3Params; 5];

/// V4 layers two new dimensions on top of v3:
/// 1. Pass-direction multipliers (offensive/defensive/neutral) on top of the
///    existing pass-chance to bias which directions the AI prefers.
/// 2. Goalkeeper freedom: 0 = locked to goal line (v3 behavior), 1 = full
///    roaming up to half-line.
///
/// All new fields default to "v3-equivalent" so a V4Params built from a
/// V3Params (with defaults on the new fields) plays identically to v3.
fn default_max_distance_from_goal() -> f32 { 1.0 }

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V4Params {
    pub v3: V3Params,
    /// [0, 2], default 1.0. Multiplied with the pass-chance when the best
    /// pass candidate would move the ball toward opponent goal.
    pub pass_dir_offensive: f32,
    /// [0, 2], default 1.0. Multiplied when best pass is backward.
    pub pass_dir_defensive: f32,
    /// [0, 2], default 1.0. Multiplied when best pass is sideways/cross.
    pub pass_dir_neutral: f32,
    /// DEPRECATED. Replaced by `max_distance_from_goal`. Kept for backward
    /// compat with old baseline.json files. Not used in logic.
    #[serde(default)]
    pub gk_freedom: f32,
    /// [0, 1], default 1.0. Per-slot roaming cap. Limits how far forward (toward
    /// opponent goal) this player may move from their own goal line.
    /// 0 = stuck on goal line; 1 = full freedom up to opponent goal.
    /// Replaces gk_freedom semantically and applies to ALL players.
    #[serde(default = "default_max_distance_from_goal")]
    pub max_distance_from_goal: f32,
}

impl Default for V4Params {
    fn default() -> Self {
        Self {
            v3: V3Params::default(),
            pass_dir_offensive: 1.0,
            pass_dir_defensive: 1.0,
            pass_dir_neutral: 1.0,
            gk_freedom: 0.0,
            max_distance_from_goal: 1.0,
        }
    }
}

pub type TeamPolicyV4 = [V4Params; 5];

/// Mutate ONE V4Params instance: 50% chance to mutate the underlying v3
/// (which itself recurses to base + modulators), plus independent ~30%
/// chance per new v4 field.
pub fn mutate_v4(p: &V4Params, rng: &mut impl Rng, scale: f32) -> V4Params {
    let mut next = *p;
    let scale = scale.max(0.05).min(2.0);

    if rng.gen::<f32>() < 0.5 {
        next.v3 = mutate_v3(&p.v3, rng, scale);
    }

    macro_rules! perturb {
        ($field:expr, $sigma:expr, $lo:expr, $hi:expr) => {
            if rng.gen::<f32>() < 0.3 {
                let dist = Normal::new(0.0f32, $sigma * scale).unwrap();
                let delta: f32 = rng.sample(dist);
                $field = ($field + delta).max($lo).min($hi);
            }
        };
    }

    perturb!(next.pass_dir_offensive, 0.15, 0.0, 2.0);
    perturb!(next.pass_dir_defensive, 0.15, 0.0, 2.0);
    perturb!(next.pass_dir_neutral, 0.15, 0.0, 2.0);
    perturb!(next.max_distance_from_goal, 0.08, 0.0, 1.0);

    next
}

pub fn mutate_team_v4(team: &TeamPolicyV4, rng: &mut impl Rng, scale: f32) -> TeamPolicyV4 {
    let mut next = *team;
    let mut any = false;
    for i in 0..5 {
        if rng.gen::<f32>() < 0.4 {
            next[i] = mutate_v4(&team[i], rng, scale);
            any = true;
        }
    }
    if !any {
        let i = rng.gen_range(0..5);
        next[i] = mutate_v4(&team[i], rng, scale);
    }
    next
}

/// Mutate ONE V3Params instance. With p_base=0.5 we mutate the underlying
/// classic params (same logic as v1/v2), and we ALSO mutate v3 modulator
/// fields (aggression, risk_appetite, edge_avoidance, etc.) with their own
/// scales and bounds. ~3 fields touched per call on average.
pub fn mutate_v3(p: &V3Params, rng: &mut impl Rng, scale: f32) -> V3Params {
    let mut next = *p;
    let scale = scale.max(0.05).min(2.0);

    // 50% chance to perturb the classic base params
    if rng.gen::<f32>() < 0.5 {
        next.base = mutate(&p.base, rng, scale);
    }

    // v3 modulator fields — each independently 30% chance to mutate
    macro_rules! perturb {
        ($field:expr, $sigma:expr, $lo:expr, $hi:expr) => {
            if rng.gen::<f32>() < 0.3 {
                let dist = Normal::new(0.0f32, $sigma * scale).unwrap();
                let delta: f32 = rng.sample(dist);
                $field = ($field + delta).max($lo).min($hi);
            }
        };
    }

    perturb!(next.aggression, 0.15, 0.0, 2.0);
    perturb!(next.risk_appetite, 0.10, 0.0, 1.0);
    perturb!(next.edge_avoidance, 0.10, 0.0, 1.0);
    perturb!(next.pressure_radius, 8.0, 30.0, 150.0);
    perturb!(next.goal_attraction, 0.10, 0.0, 1.0);
    perturb!(next.block_avoidance, 0.10, 0.0, 1.0);
    perturb!(next.block_distance, 3.0, 10.0, 60.0);
    perturb!(next.clear_shot_bonus, 0.10, 0.0, 1.0);

    // corridor_preference: occasional ±1 step
    if rng.gen::<f32>() < 0.15 {
        next.corridor_preference = (next.corridor_preference + if rng.gen::<bool>() { 1 } else { -1 }).clamp(-1, 1);
    }
    // zone_aggression — independently mutate each zone, lighter probability
    for v in next.zone_aggression.iter_mut() {
        if rng.gen::<f32>() < 0.2 {
            let dist = Normal::new(0.0f32, 0.10 * scale).unwrap();
            let delta: f32 = rng.sample(dist);
            *v = (*v + delta).max(0.3).min(2.0);
        }
    }
    next
}

/// Mutate a v3 team. ~2 of 5 positions get mutated per call (40% each, with
/// at-least-one guarantee), each position via `mutate_v3`.
pub fn mutate_team_v3(team: &TeamPolicyV3, rng: &mut impl Rng, scale: f32) -> TeamPolicyV3 {
    let mut next = *team;
    let mut any_mutated = false;
    for i in 0..5 {
        if rng.gen::<f32>() < 0.4 {
            next[i] = mutate_v3(&team[i], rng, scale);
            any_mutated = true;
        }
    }
    if !any_mutated {
        let i = rng.gen_range(0..5);
        next[i] = mutate_v3(&team[i], rng, scale);
    }
    next
}

pub const TEAM_SLOT_NAMES: [&str; 5] = ["fwd", "mid", "mid", "def", "gk"];

/// Mutate a team policy. On average ~2 of 5 positions get a mutation
/// (each independently 40%, with at-least-one guarantee).
pub fn mutate_team(team: &TeamPolicy, rng: &mut impl Rng, scale: f32) -> TeamPolicy {
    let mut next = *team;
    let mut any_mutated = false;
    for i in 0..5 {
        if rng.gen::<f32>() < 0.4 {
            next[i] = mutate(&team[i], rng, scale);
            any_mutated = true;
        }
    }
    if !any_mutated {
        let i = rng.gen_range(0..5);
        next[i] = mutate(&team[i], rng, scale);
    }
    next
}

pub fn within_bounds(p: &PolicyParams) -> bool {
    p.pass_chance_pressured >= 0.02 && p.pass_chance_pressured <= 0.4
        && p.pass_chance_wing >= 0.01 && p.pass_chance_wing <= 0.25
        && p.pass_chance_forward >= 0.005 && p.pass_chance_forward <= 0.18
        && p.pass_chance_default >= 0.005 && p.pass_chance_default <= 0.2
        && p.shoot_progress_threshold >= 0.55 && p.shoot_progress_threshold <= 0.9
        && p.tackle_chance >= 0.01 && p.tackle_chance <= 0.22
        && p.forward_pass_min_gain >= 0.0 && p.forward_pass_min_gain <= 18.0
        && p.mark_distance >= 25.0 && p.mark_distance <= 85.0
}

// ════════════════════════════════════════════════════════════════════════════
// V6: Spatial preference architecture
// ════════════════════════════════════════════════════════════════════════════
//
// V6 replaces hand-coded positional logic (attack/defend targets, role-based
// home positions, edge_avoidance, max_distance_from_goal etc.) with a single
// uniform model: each player has 5 distance preferences (min/max/preferred)
// and positions themselves to minimize the combined cost. Decisions
// (passing/shooting/tackling) stay as separate decision params.

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DistancePref {
    pub min: f32,
    pub max: f32,
    pub preferred: f32,
}

impl DistancePref {
    pub fn new(min: f32, preferred: f32, max: f32) -> Self {
        Self { min, max, preferred }
    }
    /// Cost for a measured distance vs this preference. Quadratic around
    /// preferred (normalized by range) plus soft penalty outside [min, max].
    pub fn cost(&self, d: f32) -> f32 {
        let range = (self.max - self.min).max(1.0);
        let normalized = (d - self.preferred) / range;
        let base = normalized * normalized;
        let below = (self.min - d).max(0.0);
        let above = (d - self.max).max(0.0);
        base + below * below * 0.01 + above * above * 0.01
    }
    pub fn clamp_self(&mut self) {
        if self.preferred < self.min { self.preferred = self.min; }
        if self.preferred > self.max { self.preferred = self.max; }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct V6Spatial {
    /// Distance from OWN goal. Low = defenders stay close. High = forwards push up.
    pub own_goal: DistancePref,
    /// Distance from TOP sideline (y=0). Low = top wing. High = bottom wing.
    pub side: DistancePref,
    /// Distance from ball position.
    pub ball: DistancePref,
    /// Distance to nearest teammate.
    pub teammate: DistancePref,
    /// Distance to nearest opponent.
    pub opponent: DistancePref,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionParams {
    pub pass_chance_pressured: f32,
    pub pass_chance_wing: f32,
    pub pass_chance_forward: f32,
    pub pass_chance_default: f32,
    pub shoot_progress_threshold: f32,
    pub tackle_chance: f32,
    pub forward_pass_min_gain: f32,
    pub mark_distance: f32,
    pub aggression: f32,
    pub risk_appetite: f32,
    pub pass_dir_offensive: f32,
    pub pass_dir_defensive: f32,
    pub pass_dir_neutral: f32,
}

impl Default for DecisionParams {
    fn default() -> Self {
        Self {
            pass_chance_pressured: 0.16,
            pass_chance_wing: 0.07,
            pass_chance_forward: 0.04,
            pass_chance_default: 0.055,
            shoot_progress_threshold: 0.76,
            tackle_chance: 0.08,
            forward_pass_min_gain: 8.0,
            mark_distance: 48.0,
            aggression: 1.0,
            risk_appetite: 0.5,
            pass_dir_offensive: 1.0,
            pass_dir_defensive: 1.0,
            pass_dir_neutral: 1.0,
        }
    }
}

impl DecisionParams {
    /// Used to plug into classic_tick's PolicyParams hooks (passing/shooting).
    pub fn as_policy_params(&self) -> PolicyParams {
        PolicyParams {
            pass_chance_pressured: self.pass_chance_pressured,
            pass_chance_wing: self.pass_chance_wing,
            pass_chance_forward: self.pass_chance_forward,
            pass_chance_default: self.pass_chance_default,
            shoot_progress_threshold: self.shoot_progress_threshold,
            tackle_chance: self.tackle_chance,
            forward_pass_min_gain: self.forward_pass_min_gain,
            mark_distance: self.mark_distance,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct V6Params {
    pub spatial: V6Spatial,
    pub decisions: DecisionParams,
}

pub type TeamPolicyV6 = [V6Params; 5];

/// Sensible per-slot bootstrap defaults. Index 0=fwd, 1=mid-top, 2=mid-bot, 3=def, 4=gk.
/// All values are for team 0; team 1 inherits same params (own-goal direction handled by physics).
pub fn v6_default_for_slot(slot: usize) -> V6Params {
    let spatial = match slot {
        0 => V6Spatial { // FWD
            own_goal: DistancePref::new(150.0, 380.0, 700.0),
            side:     DistancePref::new(60.0,  260.0, 460.0),
            ball:     DistancePref::new(20.0,  100.0, 280.0),
            teammate: DistancePref::new(40.0,  110.0, 220.0),
            opponent: DistancePref::new(30.0,  100.0, 260.0),
        },
        1 => V6Spatial { // MID-TOP
            own_goal: DistancePref::new(100.0, 290.0, 500.0),
            side:     DistancePref::new(40.0,  175.0, 280.0),
            ball:     DistancePref::new(20.0,  120.0, 280.0),
            teammate: DistancePref::new(40.0,  100.0, 220.0),
            opponent: DistancePref::new(20.0,  85.0,  220.0),
        },
        2 => V6Spatial { // MID-BOT
            own_goal: DistancePref::new(100.0, 290.0, 500.0),
            side:     DistancePref::new(240.0, 345.0, 480.0),
            ball:     DistancePref::new(20.0,  120.0, 280.0),
            teammate: DistancePref::new(40.0,  100.0, 220.0),
            opponent: DistancePref::new(20.0,  85.0,  220.0),
        },
        3 => V6Spatial { // DEF
            own_goal: DistancePref::new(40.0,  140.0, 320.0),
            side:     DistancePref::new(60.0,  260.0, 460.0),
            ball:     DistancePref::new(30.0,  150.0, 320.0),
            teammate: DistancePref::new(50.0,  120.0, 220.0),
            opponent: DistancePref::new(15.0,  55.0,  150.0),
        },
        _ => V6Spatial { // GK (slot 4)
            own_goal: DistancePref::new(0.0,   30.0,  80.0),
            side:     DistancePref::new(180.0, 260.0, 340.0),
            ball:     DistancePref::new(0.0,   180.0, 600.0),
            teammate: DistancePref::new(40.0,  130.0, 280.0),
            opponent: DistancePref::new(20.0,  90.0,  280.0),
        },
    };
    V6Params { spatial, decisions: DecisionParams::default() }
}

/// Mutate a single DistancePref. Each of {min, max, preferred} has ~30% chance.
fn mutate_distance_pref(p: &DistancePref, lo: f32, hi: f32, rng: &mut impl Rng, scale: f32) -> DistancePref {
    let mut n = *p;
    let sigma = (hi - lo) * 0.05 * scale;
    let dist = Normal::new(0.0f32, sigma).unwrap();
    if rng.gen::<f32>() < 0.3 { n.min = clamp(n.min + rng.sample(dist), lo, hi); }
    if rng.gen::<f32>() < 0.3 { n.max = clamp(n.max + rng.sample(dist), lo, hi); }
    if rng.gen::<f32>() < 0.3 { n.preferred = clamp(n.preferred + rng.sample(dist), lo, hi); }
    if n.min > n.max { let t = n.min; n.min = n.max; n.max = t; }
    n.clamp_self();
    DistancePref { min: round4(n.min), max: round4(n.max), preferred: round4(n.preferred) }
}

pub fn mutate_v6(p: &V6Params, rng: &mut impl Rng, scale: f32) -> V6Params {
    let scale = scale.max(0.05).min(2.0);
    let mut next = *p;
    // Spatial — each pref has its own coordinate range.
    next.spatial.own_goal = mutate_distance_pref(&next.spatial.own_goal,  0.0, 900.0, rng, scale);
    next.spatial.side     = mutate_distance_pref(&next.spatial.side,      0.0, 520.0, rng, scale);
    next.spatial.ball     = mutate_distance_pref(&next.spatial.ball,      0.0, 700.0, rng, scale);
    next.spatial.teammate = mutate_distance_pref(&next.spatial.teammate,  0.0, 400.0, rng, scale);
    next.spatial.opponent = mutate_distance_pref(&next.spatial.opponent,  0.0, 400.0, rng, scale);

    // Decisions — reuse the existing classic mutation bounds.
    macro_rules! perturb_dec {
        ($field:expr, $sigma:expr, $lo:expr, $hi:expr) => {
            if rng.gen::<f32>() < 0.25 {
                let dist = Normal::new(0.0f32, $sigma * scale).unwrap();
                $field = clamp($field + rng.sample(dist), $lo, $hi);
                $field = round4($field);
            }
        };
    }
    perturb_dec!(next.decisions.pass_chance_pressured, 0.035, 0.02, 0.4);
    perturb_dec!(next.decisions.pass_chance_wing,      0.025, 0.01, 0.25);
    perturb_dec!(next.decisions.pass_chance_forward,   0.018, 0.005, 0.18);
    perturb_dec!(next.decisions.pass_chance_default,   0.018, 0.005, 0.2);
    perturb_dec!(next.decisions.shoot_progress_threshold, 0.035, 0.55, 0.9);
    perturb_dec!(next.decisions.tackle_chance,         0.025, 0.01, 0.22);
    perturb_dec!(next.decisions.forward_pass_min_gain, 2.0, 0.0, 18.0);
    perturb_dec!(next.decisions.mark_distance,         5.0, 25.0, 85.0);
    perturb_dec!(next.decisions.aggression,            0.10, 0.0, 2.0);
    perturb_dec!(next.decisions.risk_appetite,         0.08, 0.0, 1.0);
    perturb_dec!(next.decisions.pass_dir_offensive,    0.15, 0.0, 2.0);
    perturb_dec!(next.decisions.pass_dir_defensive,    0.15, 0.0, 2.0);
    perturb_dec!(next.decisions.pass_dir_neutral,      0.15, 0.0, 2.0);
    next
}

pub fn mutate_team_v6(team: &TeamPolicyV6, rng: &mut impl Rng, scale: f32) -> TeamPolicyV6 {
    let mut next = *team;
    let mut any = false;
    for i in 0..5 {
        if rng.gen::<f32>() < 0.4 {
            next[i] = mutate_v6(&team[i], rng, scale);
            any = true;
        }
    }
    if !any {
        let i = rng.gen_range(0..5);
        next[i] = mutate_v6(&team[i], rng, scale);
    }
    next
}

/// Build a V6Params from V4Params: copy decision-relevant fields, infer spatial
/// preferences from V4's max_distance_from_goal and slot-default fallback.
pub fn v6_from_v4(v4: &V4Params, slot: usize) -> V6Params {
    let mut v6 = v6_default_for_slot(slot);
    v6.decisions.pass_chance_pressured = v4.v3.base.pass_chance_pressured;
    v6.decisions.pass_chance_wing      = v4.v3.base.pass_chance_wing;
    v6.decisions.pass_chance_forward   = v4.v3.base.pass_chance_forward;
    v6.decisions.pass_chance_default   = v4.v3.base.pass_chance_default;
    v6.decisions.shoot_progress_threshold = v4.v3.base.shoot_progress_threshold;
    v6.decisions.tackle_chance         = v4.v3.base.tackle_chance;
    v6.decisions.forward_pass_min_gain = v4.v3.base.forward_pass_min_gain;
    v6.decisions.mark_distance         = v4.v3.base.mark_distance;
    v6.decisions.aggression            = v4.v3.aggression;
    v6.decisions.risk_appetite         = v4.v3.risk_appetite;
    v6.decisions.pass_dir_offensive    = v4.pass_dir_offensive;
    v6.decisions.pass_dir_defensive    = v4.pass_dir_defensive;
    v6.decisions.pass_dir_neutral      = v4.pass_dir_neutral;
    // NOTE: v4.max_distance_from_goal is intentionally NOT mapped — per-slot
    // defaults (especially the tight GK cap) are more sensible starting points.
    let _ = v4.max_distance_from_goal;
    v6
}
