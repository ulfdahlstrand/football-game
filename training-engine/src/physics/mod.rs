#![allow(unused_imports)]
// Physics-arkitektur. Logiken ligger i sub-modulerna.
pub mod helpers;
pub mod ball;
pub mod setpieces;
pub mod rules;

// Bekväma re-exports för training_engine::physics::X
pub use helpers::{knock_player, slow_player};
pub use ball::{do_shoot, set_ball_owner, update_ball};
pub use setpieces::{
    award_set_piece, handle_ball_out, reset_kickoff, start_free_kick, start_penalty,
};
pub use rules::{step_game, tackle_player};
