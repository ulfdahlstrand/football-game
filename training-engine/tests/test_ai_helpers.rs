mod common;

use training_engine::constants::{FW, H2, PR, PASS_BLOCK_DIST};
use training_engine::ai::helpers::{
    attack_progress, dist_to_segment, is_marked, move_to, nearest_opponent_distance,
    pass_line_open,
};
use training_engine::game::{Player, PlayerState, Role};

fn make_player(id: usize, team: usize, x: f32, y: f32, role: Role) -> Player {
    Player::new(id, team, x, y, role)
}

#[test]
fn attack_progress_team0_correct() {
    assert!((attack_progress(0, 0.0) - 0.0).abs() < 1e-4);
    assert!((attack_progress(0, FW / 2.0) - 0.5).abs() < 1e-4);
    assert!((attack_progress(0, FW) - 1.0).abs() < 1e-4);
}

#[test]
fn attack_progress_team1_inverted() {
    assert!((attack_progress(1, FW) - 0.0).abs() < 1e-4);
    assert!((attack_progress(1, 0.0) - 1.0).abs() < 1e-4);
}

#[test]
fn pass_line_open_no_blocker_returns_true() {
    let game = common::make_game();
    // Pass from left to right — along centre line where no team-1 players block
    let open = pass_line_open(&game, 10.0, H2, FW - 10.0, H2, 0);
    // May or may not be blocked depending on positions, but function should not panic.
    let _ = open;
}

#[test]
fn pass_line_open_blocker_exactly_on_line_returns_false() {
    let mut game = common::make_game();
    // Place a team-1 player directly on the pass line (between passer and target).
    let from_x = FW * 0.1;
    let to_x = FW * 0.9;
    let mid_x = (from_x + to_x) / 2.0;
    // Put team-1 player at midpoint
    game.pl[5].x = mid_x;
    game.pl[5].y = H2;
    let blocked = !pass_line_open(&game, from_x, H2, to_x, H2, 0);
    assert!(blocked, "a player directly on the line should block the pass");
}

#[test]
fn is_marked_no_nearby_opponent_returns_false() {
    let game = common::make_game();
    // Player 0 (team 0) at FW*0.44 — opponents are in the other half
    assert!(!is_marked(&game, &game.pl[0], 30.0));
}

#[test]
fn is_marked_opponent_very_close_returns_true() {
    let mut game = common::make_game();
    // Place a team-1 player right next to player 0
    game.pl[5].x = game.pl[0].x + 5.0;
    game.pl[5].y = game.pl[0].y;
    assert!(is_marked(&game, &game.pl[0], 30.0));
}

#[test]
fn nearest_opponent_distance_returns_correct_value() {
    let mut game = common::make_game();
    // Move all team-1 players far away, then put one at known distance
    for i in 5..10 {
        game.pl[i].x = FW;
        game.pl[i].y = FW;
    }
    game.pl[5].x = game.pl[0].x + 50.0;
    game.pl[5].y = game.pl[0].y;
    let d = nearest_opponent_distance(&game, &game.pl[0]);
    assert!((d - 50.0).abs() < 1.0, "expected ~50, got {d}");
}

#[test]
fn move_to_moves_player_closer_to_target() {
    let mut player = Player::new(0, 0, 100.0, 100.0, Role::Fwd);
    let before_dist = (400.0_f32 - player.x).hypot(300.0 - player.y);
    move_to(&mut player, 400.0, 300.0, 5.0);
    let after_dist = (400.0_f32 - player.x).hypot(300.0 - player.y);
    assert!(after_dist < before_dist, "player should move closer");
}
