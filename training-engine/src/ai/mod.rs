#![allow(unused_imports)]
// AI tick-arkitektur. Logiken ligger i sub-modulerna.
pub mod helpers;
pub mod movement;
pub mod decisions;
pub mod v6;

// Bekväma re-exports för training_engine::ai::X
pub use helpers::{
    attack_progress, dist_to_segment, is_marked, move_to,
    nearest_opponent_distance, pass_line_open,
};
pub use decisions::{classic_tick, cpu_find_pass, PassResult};
pub use v6::v6_tick;
