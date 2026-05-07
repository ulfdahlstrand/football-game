mod common;

use training_engine::constants::TACKLE_COOL;
use training_engine::game::{MatchEvent, Phase, PlayerState};
use training_engine::physics::rules::tackle_player;

#[test]
fn tackle_success_strips_ball_from_carrier() {
    let mut game = common::make_game_with_ball_owner(0, 0);
    let carrier_idx = 0;
    let tackler_idx = 5; // team-1 player
    // Place them adjacent
    game.pl[tackler_idx].x = game.pl[carrier_idx].x + 10.0;
    game.pl[tackler_idx].y = game.pl[carrier_idx].y;
    let result = tackle_player(&mut game, tackler_idx, carrier_idx);
    assert!(result);
    assert_eq!(game.ball.owner, None, "ball should be freed after successful tackle");
    assert_eq!(game.stats.tackle_success, 1);
}

#[test]
fn tackle_sets_cooldown_on_tackler() {
    let mut game = common::make_game_with_ball_owner(0, 0);
    let tackler_idx = 5;
    game.pl[tackler_idx].x = game.pl[0].x + 10.0;
    game.pl[tackler_idx].y = game.pl[0].y;
    tackle_player(&mut game, tackler_idx, 0);
    assert_eq!(game.pl[tackler_idx].tackle_cooldown, TACKLE_COOL);
}

#[test]
fn tackle_on_jumping_target_knocks_tackler() {
    let mut game = common::make_game();
    let target_idx = 0;
    let tackler_idx = 5;
    game.pl[target_idx].jump_timer = 10; // target is jumping
    let result = tackle_player(&mut game, tackler_idx, target_idx);
    assert!(result);
    assert_eq!(game.pl[tackler_idx].state, PlayerState::Knocked);
}

#[test]
fn tackle_emits_tackle_event_on_success() {
    let mut game = common::make_game_with_ball_owner(0, 0);
    let carrier_idx = 0;
    let tackler_idx = 5;
    game.pl[tackler_idx].x = game.pl[carrier_idx].x + 10.0;
    game.pl[tackler_idx].y = game.pl[carrier_idx].y;
    tackle_player(&mut game, tackler_idx, carrier_idx);
    let has_tackle = game.events.iter().any(|e| matches!(e, MatchEvent::Tackle { .. }));
    assert!(has_tackle, "expected Tackle event");
}

#[test]
fn off_ball_tackle_in_own_penalty_area_awards_penalty() {
    let mut game = common::make_game();
    let tackler_idx = 3; // team-0 Def player
    let target_idx = 5;  // team-1 player
    // Move tackler into team-0's own penalty area (x near 0)
    game.pl[tackler_idx].x = 50.0;
    game.pl[tackler_idx].y = training_engine::constants::H2;
    game.pl[target_idx].x = 60.0;
    game.pl[target_idx].y = training_engine::constants::H2;
    // Ensure no ball owner (off-ball tackle)
    game.ball.owner = None;
    tackle_player(&mut game, tackler_idx, target_idx);
    assert_eq!(game.phase, Phase::Penalty, "off-ball tackle in own area should give penalty");
}

#[test]
fn off_ball_tackle_outside_penalty_area_awards_free_kick() {
    let mut game = common::make_game();
    let tackler_idx = 0; // team-0 Fwd (far from own goal)
    let target_idx = 5;  // team-1
    // Put them in the middle of the pitch
    game.pl[tackler_idx].x = training_engine::constants::FW * 0.5;
    game.pl[tackler_idx].y = training_engine::constants::H2;
    game.pl[target_idx].x = training_engine::constants::FW * 0.5 + 10.0;
    game.pl[target_idx].y = training_engine::constants::H2;
    game.ball.owner = None;
    tackle_player(&mut game, tackler_idx, target_idx);
    let has_foul = game.events.iter().any(|e| matches!(e, MatchEvent::Foul { is_penalty: false, .. }));
    assert!(has_foul, "off-ball tackle outside area should give free kick");
}
