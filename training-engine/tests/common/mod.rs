#![allow(dead_code)]

use rand::SeedableRng;
use rand::rngs::SmallRng;

use training_engine::game::{Game, MatchEvent, Phase};
use training_engine::physics::ball::set_ball_owner;
use training_engine::physics::rules::step_game;
use training_engine::policy::v6::{v6_default_for_slot, V6Params};
use training_engine::team::Team;
use training_engine::team_v6::V6Team;

pub fn make_game() -> Game {
    Game::new()
}

/// Creates a game where the player at `slot` on `team` has the ball.
pub fn make_game_with_ball_owner(team: usize, slot: usize) -> Game {
    let mut game = Game::new();
    // Players are indexed 0-4 for team 0, 5-9 for team 1
    let idx = if team == 0 { slot } else { 5 + slot };
    let (x, y) = (game.pl[idx].x, game.pl[idx].y);
    set_ball_owner(&mut game, idx, x, y);
    game
}

pub fn place_player(game: &mut Game, idx: usize, x: f32, y: f32) {
    game.pl[idx].x = x;
    game.pl[idx].y = y;
    game.pl[idx].home_x = x;
    game.pl[idx].home_y = y;
}

pub fn make_v6_teams() -> [Box<dyn Team>; 2] {
    let p0: [V6Params; 5] = std::array::from_fn(|s| v6_default_for_slot(s));
    let p1: [V6Params; 5] = std::array::from_fn(|s| v6_default_for_slot(s));
    [
        Box::new(V6Team::new(0, p0)),
        Box::new(V6Team::new(1, p1)),
    ]
}

pub fn run_until_fulltime(game: &mut Game, teams: &mut [Box<dyn Team>; 2]) {
    let mut rng = SmallRng::seed_from_u64(42);
    let mut iters = 0;
    while game.phase != Phase::Fulltime && iters < 200_000 {
        step_game(game, teams, &mut rng);
        iters += 1;
    }
}

pub fn count_events<F: Fn(&MatchEvent) -> bool>(game: &Game, pred: F) -> usize {
    game.events.iter().filter(|e| pred(e)).count()
}

pub fn deterministic_rng() -> SmallRng {
    SmallRng::seed_from_u64(0)
}
