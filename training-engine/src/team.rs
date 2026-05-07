use rand::RngCore;
use crate::game::Game;

pub trait Team: Send {
    fn tick_player(&mut self, game: &mut Game, player_idx: usize, rng: &mut dyn RngCore);
    fn pre_tick(&mut self, _game: &Game) {}
    fn team_id(&self) -> usize;
}
