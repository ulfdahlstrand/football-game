//! training-render: SVG-rendering för training-engine.
//!
//! Engine-fri rendering — alla funktioner tar engine-data som input och
//! producerar SVG-strängar/filer. Ingen kod i `training-engine` importerar
//! härifrån, vilket gör att engine-bygget kan optimeras för rena
//! träningskörningar utan render-overhead.

pub mod svg;
pub mod team_layout;

pub use svg::{
    write_matrix_svg, MatrixCell,
    write_progress_svg, SessionProgress,
    write_tournament_svg,
    write_training_svg,
};
pub use team_layout::{compute_v6_preferred_xy, write_team_layout_svg};
