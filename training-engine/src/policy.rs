use rand::Rng;
use rand_distr::Normal;
use serde::{Deserialize, Serialize};

/// Classic decision parameters. Still used by v6: v6_tick converts DecisionParams
/// to PolicyParams and passes it to classic_tick for on-ball decisions.
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

fn clamp(v: f32, lo: f32, hi: f32) -> f32 {
    v.max(lo).min(hi)
}

fn round4(v: f32) -> f32 {
    (v * 10000.0).round() / 10000.0
}

pub const TEAM_SLOT_NAMES: [&str; 5] = ["fwd", "mid", "mid", "def", "gk"];

// ════════════════════════════════════════════════════════════════════════════
// V6: Spatial preference architecture
// ════════════════════════════════════════════════════════════════════════════

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
    pub own_goal: DistancePref,
    pub side: DistancePref,
    pub ball: DistancePref,
    pub teammate: DistancePref,
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GkDecisionParams {
    pub gk_dive_chance: f32,
    pub gk_dive_commit_dist: f32,
    pub gk_risk_clearance: f32,
    pub gk_distribution_zone: f32,
    pub gk_pass_target_dist: f32,
    #[serde(default)]
    pub gk_sweeper_freedom: f32,
}

impl Default for GkDecisionParams {
    fn default() -> Self {
        Self {
            gk_dive_chance: 0.9,
            gk_dive_commit_dist: 160.0,
            gk_risk_clearance: 0.5,
            gk_distribution_zone: 0.0,
            gk_pass_target_dist: 200.0,
            gk_sweeper_freedom: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct V6Params {
    pub spatial: V6Spatial,
    pub decisions: DecisionParams,
    #[serde(default)]
    pub gk: Option<GkDecisionParams>,
}

pub type TeamPolicyV6 = [V6Params; 5];

pub fn v6_default_for_slot(slot: usize) -> V6Params {
    let spatial = match slot {
        0 => V6Spatial {
            own_goal: DistancePref::new(150.0, 380.0, 700.0),
            side:     DistancePref::new(60.0,  260.0, 460.0),
            ball:     DistancePref::new(20.0,  100.0, 280.0),
            teammate: DistancePref::new(40.0,  110.0, 220.0),
            opponent: DistancePref::new(30.0,  100.0, 260.0),
        },
        1 => V6Spatial {
            own_goal: DistancePref::new(100.0, 290.0, 500.0),
            side:     DistancePref::new(40.0,  175.0, 280.0),
            ball:     DistancePref::new(20.0,  120.0, 280.0),
            teammate: DistancePref::new(40.0,  100.0, 220.0),
            opponent: DistancePref::new(20.0,  85.0,  220.0),
        },
        2 => V6Spatial {
            own_goal: DistancePref::new(100.0, 290.0, 500.0),
            side:     DistancePref::new(240.0, 345.0, 480.0),
            ball:     DistancePref::new(20.0,  120.0, 280.0),
            teammate: DistancePref::new(40.0,  100.0, 220.0),
            opponent: DistancePref::new(20.0,  85.0,  220.0),
        },
        3 => V6Spatial {
            own_goal: DistancePref::new(40.0,  140.0, 320.0),
            side:     DistancePref::new(60.0,  260.0, 460.0),
            ball:     DistancePref::new(30.0,  150.0, 320.0),
            teammate: DistancePref::new(50.0,  120.0, 220.0),
            opponent: DistancePref::new(15.0,  55.0,  150.0),
        },
        _ => V6Spatial {
            own_goal: DistancePref::new(0.0,   30.0,  80.0),
            side:     DistancePref::new(180.0, 260.0, 340.0),
            ball:     DistancePref::new(0.0,   180.0, 600.0),
            teammate: DistancePref::new(40.0,  130.0, 280.0),
            opponent: DistancePref::new(20.0,  90.0,  280.0),
        },
    };
    let gk = if slot == 4 { Some(GkDecisionParams::default()) } else { None };
    V6Params { spatial, decisions: DecisionParams::default(), gk }
}

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
    next.spatial.own_goal = mutate_distance_pref(&next.spatial.own_goal,  0.0, 900.0, rng, scale);
    next.spatial.side     = mutate_distance_pref(&next.spatial.side,      0.0, 520.0, rng, scale);
    next.spatial.ball     = mutate_distance_pref(&next.spatial.ball,      0.0, 700.0, rng, scale);
    next.spatial.teammate = mutate_distance_pref(&next.spatial.teammate,  0.0, 400.0, rng, scale);
    next.spatial.opponent = mutate_distance_pref(&next.spatial.opponent,  0.0, 400.0, rng, scale);

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

    if let Some(ref mut gk) = next.gk {
        perturb_dec!(gk.gk_dive_chance,        0.07, 0.2, 1.0);
        perturb_dec!(gk.gk_dive_commit_dist,   15.0, 60.0, 280.0);
        perturb_dec!(gk.gk_risk_clearance,     0.08, 0.0, 1.0);
        perturb_dec!(gk.gk_distribution_zone,  0.12, 0.0, 1.0);
        perturb_dec!(gk.gk_pass_target_dist,   20.0, 80.0, 400.0);
        perturb_dec!(gk.gk_sweeper_freedom,    0.10, 0.0, 1.0);
    }
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

pub fn mutate_gk_only(team: &TeamPolicyV6, rng: &mut impl Rng, scale: f32) -> TeamPolicyV6 {
    let mut next = *team;
    next[4] = mutate_v6(&team[4], rng, scale);
    next
}

pub fn mutate_slot_only(team: &TeamPolicyV6, slot: usize, rng: &mut impl Rng, scale: f32) -> TeamPolicyV6 {
    let mut next = *team;
    next[slot] = mutate_v6(&team[slot], rng, scale);
    next
}
