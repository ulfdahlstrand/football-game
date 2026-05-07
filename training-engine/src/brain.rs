use rand::Rng;

use crate::game::Game;
use crate::policy::V6Params;

/// Hooks passed from v6_tick into classic_tick for on-ball decisions.
#[derive(Clone, Copy, Debug)]
pub struct TickHooks {
    pub pass_dir_mult: [f32; 3],
    pub gk_freedom: f32,
    pub max_distance_from_goal: f32,
    pub gk_dive_chance: f32,
    pub gk_dive_commit_dist: f32,
    pub gk_risk_clearance: f32,
    pub gk_distribution_zone: f32,
    pub gk_pass_target_dist: f32,
}

impl Default for TickHooks {
    fn default() -> Self {
        Self {
            pass_dir_mult: [1.0, 1.0, 1.0],
            gk_freedom: 0.0,
            max_distance_from_goal: 1.0,
            gk_dive_chance: 0.9,
            gk_dive_commit_dist: 160.0,
            gk_risk_clearance: 0.5,
            gk_distribution_zone: 0.0,
            gk_pass_target_dist: 200.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PlayerBrain {
    V6(V6Params),
}

impl PlayerBrain {
    pub fn base_params(&self) -> crate::policy::PolicyParams {
        match self {
            PlayerBrain::V6(p) => p.decisions.as_policy_params(),
        }
    }

    pub fn version_label(&self) -> &'static str {
        match self {
            PlayerBrain::V6(_) => "v6",
        }
    }
}

impl Default for PlayerBrain {
    fn default() -> Self {
        PlayerBrain::V6(V6Params::default())
    }
}

pub fn tick_player(game: &mut Game, player_idx: usize, rng: &mut impl Rng) {
    let brain = game.pl[player_idx].brain;
    match brain {
        PlayerBrain::V6(p) => {
            crate::ai::v6_tick(game, player_idx, &p, rng);
        }
    }
}
