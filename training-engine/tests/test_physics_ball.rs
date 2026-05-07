mod common;

use training_engine::constants::{FW, H2, SHOOT_POW, BALL_COOL};
use training_engine::game::{MatchEvent, Phase};
use training_engine::physics::ball::{do_shoot, set_ball_owner, update_ball};

#[test]
fn do_shoot_sets_ball_velocity_toward_target() {
    let mut game = common::make_game();
    // Shooter at FW/4, target to the right at FW*0.75
    let shooter_idx = 0;
    game.pl[shooter_idx].x = FW * 0.25;
    game.pl[shooter_idx].y = H2;
    do_shoot(&mut game, shooter_idx, false, FW * 0.75, H2, None, false);
    assert!(game.ball.vx > 0.0, "ball should move right");
    assert!(game.ball.vy.abs() < 0.1, "ball should move horizontally");
}

#[test]
fn do_shoot_emits_shot_event() {
    let mut game = common::make_game();
    do_shoot(&mut game, 0, false, FW, H2, None, false);
    let has_shot = game.events.iter().any(|e| matches!(e, MatchEvent::Shot { .. }));
    assert!(has_shot, "expected Shot event");
}

#[test]
fn do_shoot_pass_emits_pass_event() {
    let mut game = common::make_game();
    do_shoot(&mut game, 0, false, FW * 0.6, H2, None, true);
    let has_pass = game.events.iter().any(|e| matches!(e, MatchEvent::Pass { .. }));
    assert!(has_pass, "expected Pass event");
}

#[test]
fn do_shoot_pass_sets_last_passer() {
    let mut game = common::make_game();
    let id = game.pl[0].id;
    do_shoot(&mut game, 0, false, FW * 0.6, H2, None, true);
    assert_eq!(game.last_passer, Some(id));
}

#[test]
fn do_shoot_shot_sets_last_shooter() {
    let mut game = common::make_game();
    let id = game.pl[0].id;
    do_shoot(&mut game, 0, false, FW, H2, None, false);
    assert_eq!(game.last_shooter, Some(id));
}

#[test]
fn set_ball_owner_assigns_ball_to_player() {
    let mut game = common::make_game();
    let id = game.pl[2].id;
    let (x, y) = (game.pl[2].x, game.pl[2].y);
    set_ball_owner(&mut game, 2, x, y);
    assert_eq!(game.ball.owner, Some(id));
}

#[test]
fn update_ball_applies_friction() {
    let mut game = common::make_game();
    game.ball.vx = 10.0;
    game.ball.vy = 0.0;
    game.ball.cooldown = 0;
    let vx_before = game.ball.vx;
    update_ball(&mut game);
    assert!(game.ball.vx < vx_before, "friction should reduce speed");
}

#[test]
fn update_ball_goal_increments_score() {
    let mut game = common::make_game();
    // Place ball just inside left goal (team 1 scores).
    game.ball.x = 10.0;
    game.ball.y = H2;
    game.ball.vx = -15.0;
    game.ball.vy = 0.0;
    game.ball.cooldown = 0;
    update_ball(&mut game);
    assert_eq!(game.score[1], 1, "team 1 should score when ball enters left goal");
    assert_eq!(game.phase, Phase::Goal);
}

#[test]
fn update_ball_cooldown_decrements() {
    let mut game = common::make_game();
    game.ball.cooldown = 5;
    update_ball(&mut game);
    assert_eq!(game.ball.cooldown, 4);
}
