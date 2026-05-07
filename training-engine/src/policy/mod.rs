#![allow(unused_imports)]
// Policy-parametrar. Logiken ligger i sub-modulerna.
pub mod v6;
pub mod v7;

// Bekväma re-exports för training_engine::policy::X
pub use v6::{
    DecisionParams, DistancePref, GkDecisionParams, mutate_gk_only,
    mutate_slot_only, mutate_team_v6, mutate_v6, PolicyParams, TeamPolicyV6,
    TEAM_SLOT_NAMES, v6_default_for_slot, V6Params, V6Spatial,
};
pub use v7::{mutate_v7, V7TeamParams};
