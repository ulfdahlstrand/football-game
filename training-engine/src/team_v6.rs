use rand::RngCore;
use crate::game::Game;
use crate::policy::TeamPolicyV6;
use crate::team::Team;

pub struct V6Team {
    id:     usize,
    policy: TeamPolicyV6,
}

impl V6Team {
    pub fn new(id: usize, policy: TeamPolicyV6) -> Self {
        Self { id, policy }
    }
}

impl Team for V6Team {
    fn tick_player(&mut self, game: &mut Game, player_idx: usize, rng: &mut dyn RngCore) {
        let slot = player_idx % 5;
        crate::ai::v6_tick(game, player_idx, &self.policy[slot], rng);
    }

    fn team_id(&self) -> usize {
        self.id
    }
}
