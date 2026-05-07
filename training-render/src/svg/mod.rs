// SVG-skrivare. Logiken ligger i sub-modulerna; helpers samlade i `helpers`/`colors`.
pub mod colors;
pub mod helpers;
pub mod matrix;
pub mod tournament;
pub mod progress;
pub mod training;

// Bekväma re-exports
pub use matrix::{MatrixCell, write_matrix_svg};
pub use tournament::write_tournament_svg;
pub use progress::{SessionProgress, write_progress_svg};
pub use training::write_training_svg;
