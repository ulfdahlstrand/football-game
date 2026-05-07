mod common;

use training_engine::constants::{FW, H2};
use training_engine::detector::{detect_global, detect_local};
use training_engine::game::PlayerState;

#[test]
fn global_opp_avg_x_in_valid_range() {
    let game = common::make_game();
    let b = detect_global(&game, 0);
    assert!(b.opp_avg_x >= 0.0 && b.opp_avg_x <= 1.0, "opp_avg_x={} must be in [0,1]", b.opp_avg_x);
}

#[test]
fn global_opp_avg_x_changes_when_opponents_move() {
    let mut game = common::make_game();
    let b_before = detect_global(&game, 0);
    // Move all team-1 players to x=5 (very close to team-0's own goal)
    for i in 5..10 { game.pl[i].x = 5.0; }
    let b_after = detect_global(&game, 0);
    assert!(b_after.opp_avg_x < b_before.opp_avg_x,
        "opp_avg_x should drop when opponents move toward our goal");
}

#[test]
fn global_no_opponents_nearby_when_far_apart() {
    let mut game = common::make_game();
    // Push all team-1 players far to the right
    for i in 5..10 {
        game.pl[i].x = FW * 0.98;
        game.pl[i].y = H2;
    }
    let b = detect_global(&game, 0);
    assert_eq!(b.opp_press_rate, 0.0, "press rate should be 0 when opponents are far away");
}

#[test]
fn global_all_opponents_overlapping_press_rate_one() {
    let mut game = common::make_game();
    // Move all team-1 players on top of all team-0 players.
    for i in 0..5 {
        game.pl[5 + i].x = game.pl[i].x;
        game.pl[5 + i].y = game.pl[i].y;
    }
    let b = detect_global(&game, 0);
    assert_eq!(b.opp_press_rate, 1.0);
}

#[test]
fn global_space_behind_high_when_opp_is_pushed_forward() {
    let mut game = common::make_game();
    // Push all team-1 players to x = FW * 0.9
    for i in 5..10 {
        game.pl[i].x = FW * 0.9;
    }
    let b = detect_global(&game, 0);
    assert!(b.space_behind > 0.7, "space_behind={}", b.space_behind);
}

#[test]
fn local_pressure_one_when_five_opponents_nearby() {
    let mut game = common::make_game();
    // Pile all 5 team-1 players right on top of player 0
    for i in 5..10 {
        game.pl[i].x = game.pl[0].x;
        game.pl[i].y = game.pl[0].y;
    }
    let ctx = detect_local(&game, 0);
    assert_eq!(ctx.local_pressure, 1.0);
}

#[test]
fn local_pressure_zero_when_opponents_far_away() {
    let mut game = common::make_game();
    // Push all opponents to the far corner
    for i in 5..10 {
        game.pl[i].x = FW * 0.99;
        game.pl[i].y = 5.0;
    }
    let ctx = detect_local(&game, 0);
    assert_eq!(ctx.local_pressure, 0.0);
}

#[test]
fn local_ball_proximity_one_when_player_at_ball() {
    let mut game = common::make_game();
    game.pl[0].x = game.ball.x;
    game.pl[0].y = game.ball.y;
    let ctx = detect_local(&game, 0);
    assert_eq!(ctx.ball_proximity, 1.0);
}
