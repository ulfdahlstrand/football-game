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
