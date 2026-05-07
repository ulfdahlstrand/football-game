use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::policy::v6::{mutate_v6, round4, TeamPolicyV6};
use crate::team_v7::CoachStyle;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct V7TeamParams {
    pub instinct: TeamPolicyV6,
    pub coachability: [f32; 5],
    pub coach_style: CoachStyle,
}

impl V7TeamParams {
    pub fn from_v6(policy: TeamPolicyV6) -> Self {
        Self {
            instinct: policy,
            coachability: [0.5; 5],
            coach_style: CoachStyle::default(),
        }
    }
}

pub fn mutate_v7(p: &V7TeamParams, rng: &mut impl Rng, scale: f32) -> V7TeamParams {
    let scale = scale.max(0.05).min(2.0);
    let mut next = p.clone();

    for i in 0..5 {
        if rng.gen::<f32>() < 0.4 {
            next.instinct[i] = mutate_v6(&p.instinct[i], rng, scale);
        }
    }

    for i in 0..5 {
        if rng.gen::<f32>() < 0.3 {
            let sigma = 0.06 * scale;
            let delta: f32 = rng.gen::<f32>() * sigma * 2.0 - sigma;
            next.coachability[i] = round4((p.coachability[i] + delta).clamp(0.05, 0.95));
        }
    }

    macro_rules! perturb_style {
        ($field:expr, $sigma:expr) => {
            if rng.gen::<f32>() < 0.3 {
                let delta: f32 = rng.gen::<f32>() * $sigma * 2.0 * scale - $sigma * scale;
                $field = round4(($field + delta).clamp(0.0, 1.0));
            }
        };
    }
    perturb_style!(next.coach_style.press_response,    0.08);
    perturb_style!(next.coach_style.depth_response,    0.08);
    perturb_style!(next.coach_style.compactness_base,  0.07);
    perturb_style!(next.coach_style.tempo_base,        0.07);

    next
}
