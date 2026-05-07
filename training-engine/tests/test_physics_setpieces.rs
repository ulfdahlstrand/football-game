mod common;

use training_engine::constants::{FW, H2};
use training_engine::game::{MatchEvent, Phase};
use training_engine::physics::setpieces::{
    award_set_piece, reset_kickoff, start_free_kick, start_penalty,
};

#[test]
fn reset_kickoff_puts_ball_at_centre() {
    let mut game = common::make_game_with_ball_owner(0, 0);
    reset_kickoff(&mut game);
    assert!((game.ball.x - FW / 2.0).abs() < 0.1, "ball.x={}", game.ball.x);
    assert!((game.ball.y - H2).abs() < 0.1, "ball.y={}", game.ball.y);
    assert_eq!(game.ball.owner, None);
    assert_eq!(game.phase, Phase::Kickoff);
}

#[test]
fn reset_kickoff_clears_free_kick_state() {
    let mut game = common::make_game();
    game.free_kick_active = true;
    game.free_kick_shooter_id = Some(0);
    reset_kickoff(&mut game);
    assert!(!game.free_kick_active);
    assert_eq!(game.free_kick_shooter_id, None);
}

#[test]
fn award_set_piece_sets_taker_and_ball_position() {
    let mut game = common::make_game();
    let taker_id = game.pl[3].id;
    award_set_piece(&mut game, taker_id, 200.0, 100.0);
    assert_eq!(game.set_piece_taker_id, Some(taker_id));
    assert!((game.ball.x - 200.0).abs() < 0.1);
    assert!((game.ball.y - 100.0).abs() < 0.1);
    assert_eq!(game.ball.owner, None);
    assert_eq!(game.phase, Phase::Playing);
}

#[test]
fn start_free_kick_emits_event_and_sets_taker() {
    let mut game = common::make_game();
    let fouled_id = game.pl[1].id; // team-0 Mid
    start_free_kick(&mut game, fouled_id, 300.0, H2);
    assert!(game.free_kick_active);
    assert_eq!(game.free_kick_shooter_id, Some(fouled_id));
    let has_fk = game.events.iter().any(|e| matches!(e, MatchEvent::FreeKick { .. }));
    assert!(has_fk, "expected FreeKick event");
}

#[test]
fn start_penalty_sets_phase_and_penalty_team() {
    let mut game = common::make_game();
    start_penalty(&mut game, 1);
    assert_eq!(game.phase, Phase::Penalty);
    assert_eq!(game.penalty_team, Some(1));
    assert!(!game.penalty_taken);
}

#[test]
fn start_penalty_increments_penalty_stat() {
    let mut game = common::make_game();
    start_penalty(&mut game, 0);
    assert_eq!(game.stats.penalties, 1);
}

#[test]
fn reset_kickoff_resets_all_player_positions() {
    let mut game = common::make_game();
    // Scramble all players
    for p in &mut game.pl {
        p.x = FW * 0.5;
        p.y = H2;
    }
    reset_kickoff(&mut game);
    // After reset, players should be back at home positions (check GK team 0, idx 4)
    let gk0 = &game.pl[4];
    assert!(gk0.x < FW * 0.2, "GK0 should be near left goal, got x={}", gk0.x);
}
