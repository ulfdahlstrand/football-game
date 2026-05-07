mod common;

use training_engine::constants::{FW, H2, PR};
use training_engine::ai::v6::{nearest_active_dist, v6_target, v6_total_cost};
use training_engine::game::PlayerState;
use training_engine::policy::v6::v6_default_for_slot;

#[test]
fn nearest_active_dist_ignores_knocked_players() {
    let mut game = common::make_game();
    // Knock all team-1 players (they're "opponents" for team-0)
    for i in 5..10 {
        game.pl[i].state = PlayerState::Knocked;
    }
    let d = nearest_active_dist(&game, game.pl[0].id, Some(1), game.pl[0].x, game.pl[0].y);
    assert_eq!(d, 600.0, "should return sentinel 600.0 when no active opponents");
}

#[test]
fn nearest_active_dist_finds_correct_distance() {
    let mut game = common::make_game();
    // Move all team-1 players far away, put one at known offset
    for i in 5..10 {
        game.pl[i].x = 9000.0;
        game.pl[i].y = 9000.0;
    }
    game.pl[5].x = game.pl[0].x + 100.0;
    game.pl[5].y = game.pl[0].y;
    let d = nearest_active_dist(&game, game.pl[0].id, Some(1), game.pl[0].x, game.pl[0].y);
    assert!((d - 100.0).abs() < 1.0, "expected ~100, got {d}");
}

#[test]
fn v6_target_returns_position_within_field() {
    let game = common::make_game();
    let spatial = v6_default_for_slot(0).spatial;
    let (tx, ty) = v6_target(&game, 0, &spatial);
    assert!(tx >= PR && tx <= FW - PR, "tx={tx} out of bounds");
    assert!(ty >= PR && ty <= training_engine::constants::FH - PR, "ty={ty} out of bounds");
}

#[test]
fn v6_total_cost_is_finite() {
    let game = common::make_game();
    let spatial = v6_default_for_slot(0).spatial;
    let cost = v6_total_cost(&game, 0, game.pl[0].x, game.pl[0].y, &spatial);
    assert!(cost.is_finite(), "cost should be finite");
}

#[test]
fn v6_total_cost_lower_at_preferred_own_goal_distance() {
    let game = common::make_game();
    let spatial = v6_default_for_slot(0).spatial;
    // Cost at player's current position vs a position right on the goal line
    let cost_at_player = v6_total_cost(&game, 0, game.pl[0].x, game.pl[0].y, &spatial);
    let cost_on_goal_line = v6_total_cost(&game, 0, 0.0, H2, &spatial);
    // The forward's preferred own_goal distance is 380 — they should prefer being away from goal.
    assert!(cost_at_player < cost_on_goal_line, "forward should not prefer standing on goal line");
}
