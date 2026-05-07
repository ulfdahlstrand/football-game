mod common;

use training_engine::constants::{FW, H2, GH, PENALTY_AREA_W, BALL_COOL};
use training_engine::game::PlayerState;
use training_engine::physics::helpers::{is_in_penalty_area_for_team, knock_player, slow_player};

#[test]
fn knock_player_sets_knocked_state() {
    let mut game = common::make_game();
    knock_player(&mut game, 0, 60);
    assert_eq!(game.pl[0].state, PlayerState::Knocked);
    assert_eq!(game.pl[0].knock_timer, 60);
}

#[test]
fn knock_player_drops_ball_if_owned() {
    let mut game = common::make_game_with_ball_owner(0, 0);
    assert!(game.ball.owner.is_some());
    knock_player(&mut game, 0, 60);
    assert_eq!(game.ball.owner, None);
    assert_eq!(game.ball.cooldown, BALL_COOL);
}

#[test]
fn knock_player_no_op_if_already_knocked() {
    let mut game = common::make_game();
    knock_player(&mut game, 0, 60);
    // Knocking again should not reset timer
    knock_player(&mut game, 0, 999);
    assert_eq!(game.pl[0].knock_timer, 60, "second knock should be ignored");
}

#[test]
fn slow_player_sets_timer() {
    let mut game = common::make_game();
    slow_player(&mut game, 2, 30);
    assert_eq!(game.pl[2].slow_timer, 30);
}

#[test]
fn penalty_area_detection_correct() {
    // Team 0 penalty area is on the left (x <= PENALTY_AREA_W) near the goal height.
    assert!(is_in_penalty_area_for_team(0, 10.0, H2));
    assert!(!is_in_penalty_area_for_team(0, FW * 0.5, H2));
    assert!(is_in_penalty_area_for_team(1, FW - 10.0, H2));
    assert!(!is_in_penalty_area_for_team(1, FW * 0.5, H2));
    // Outside goal height strip
    assert!(!is_in_penalty_area_for_team(0, 10.0, 0.0));
}
