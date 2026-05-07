//! training-engine: ren simulerings- och träningsmotor.
//!
//! Innehåller fotbollssimulering (game/physics/ai), träningsinfrastruktur
//! (trainer/session) och AI-policy (policy/team/team_v6/team_v7). Inget
//! rendering-beroende — SVG och annan visualisering ligger i den
//! separata `training-render`-craten.

pub mod constants;
pub mod game;
pub mod math;
pub mod brain;
pub mod policy;
pub mod spatial;
pub mod ai;
pub mod gk;
pub mod physics;
pub mod team;
pub mod team_v6;
pub mod team_v7;
pub mod detector;
#[cfg(not(target_arch = "wasm32"))]
pub mod trainer;
#[cfg(not(target_arch = "wasm32"))]
pub mod session;
