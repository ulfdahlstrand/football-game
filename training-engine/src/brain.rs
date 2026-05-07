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
