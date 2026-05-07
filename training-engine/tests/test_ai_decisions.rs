mod common;

use training_engine::ai::decisions::cpu_find_pass;
use training_engine::game::PlayerState;
use training_engine::physics::ball::set_ball_owner;
use training_engine::policy::v6::PolicyParams;

fn give_ball(game: &mut training_engine::game::Game, player_idx: usize) {
    let (x, y) = (game.pl[player_idx].x, game.pl[player_idx].y);
    set_ball_owner(game, player_idx, x, y);
}

#[test]
fn cpu_find_pass_finds_open_teammate() {
    let mut game = common::make_game();
    give_ball(&mut game, 0);
    let params = PolicyParams::default();
    let result = cpu_find_pass(&game, 0, &params);
    assert!(result.is_some(), "should find a pass option with open teammates");
}

#[test]
fn cpu_find_pass_returns_none_when_all_marked() {
    let mut game = common::make_game();
    give_ball(&mut game, 0);
    let params = PolicyParams { mark_distance: 99999.0, ..PolicyParams::default() };
    let result = cpu_find_pass(&game, 0, &params);
    assert!(result.is_none(), "no pass when all teammates are marked");
}

#[test]
fn cpu_find_pass_respects_forward_pass_min_gain() {
    let mut game = common::make_game();
    give_ball(&mut game, 0);
    let params = PolicyParams { forward_pass_min_gain: 9999.0, ..PolicyParams::default() };
    let _ = cpu_find_pass(&game, 0, &params); // should not panic
}

#[test]
fn cpu_find_pass_target_is_different_from_carrier() {
    let mut game = common::make_game();
    give_ball(&mut game, 0);
    let carrier_id = game.pl[0].id;
    let params = PolicyParams::default();
    let result = cpu_find_pass(&game, 0, &params);
    if let Some(r) = result {
        assert_ne!(r.target_id, carrier_id, "cannot pass to yourself");
    }
}

#[test]
fn cpu_find_pass_all_teammates_inactive_returns_none() {
    let mut game = common::make_game();
    give_ball(&mut game, 0);
    for i in 1..5 {
        game.pl[i].state = PlayerState::Knocked;
    }
    let params = PolicyParams::default();
    let result = cpu_find_pass(&game, 0, &params);
    assert!(result.is_none(), "no pass when all teammates are knocked");
}

#[test]
fn cpu_find_pass_target_is_on_same_team() {
    let mut game = common::make_game();
    give_ball(&mut game, 0);
    let params = PolicyParams::default();
    if let Some(r) = cpu_find_pass(&game, 0, &params) {
        let target_team = game.pl.iter().find(|p| p.id == r.target_id).map(|p| p.team);
        assert_eq!(target_team, Some(0), "pass target must be on same team");
    }
}
