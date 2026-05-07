use crate::constants::SLOW_FACTOR;
use crate::game::Player;

/// Normaliserar en 2D-vektor. Returnerar (0,0) om vektorn är degenererad.
pub fn norm(dx: f32, dy: f32) -> (f32, f32) {
    let m = dx.hypot(dy);
    if m < 1e-9 { (0.0, 0.0) } else { (dx / m, dy / m) }
}

/// Hastighetsmultiplikator för en spelare som är slowed.
pub fn slow_factor(p: &Player) -> f32 {
    if p.slow_timer > 0 { SLOW_FACTOR } else { 1.0 }
}
