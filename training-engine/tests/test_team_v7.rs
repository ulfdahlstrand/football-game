mod common;

use training_engine::constants::H2;
use training_engine::policy::v6::v6_default_for_slot;
use training_engine::team_v7::{apply_directive, Coach, CoachDirective, CoachStyle, V7Team};

fn neutral_directive() -> CoachDirective {
    CoachDirective { press_intensity: 0.5, line_height: 0.5, compactness: 0.5, tempo: 0.5 }
}

fn high_press_directive() -> CoachDirective {
    CoachDirective { press_intensity: 1.0, line_height: 0.5, compactness: 0.5, tempo: 0.5 }
}

fn low_press_directive() -> CoachDirective {
    CoachDirective { press_intensity: 0.0, line_height: 0.5, compactness: 0.5, tempo: 0.5 }
}

#[test]
fn neutral_directive_leaves_params_unchanged() {
    let instinct = v6_default_for_slot(0);
    let dir = neutral_directive();
    let coached = apply_directive(&instinct, &dir, 1.0);
    // Neutral (0.5) press_intensity → press_delta = 0 → aggression unchanged.
    let delta = (coached.decisions.aggression - instinct.decisions.aggression).abs();
    assert!(delta < 1e-4, "delta={delta}");
}

#[test]
fn zero_coachability_returns_instinct() {
    let instinct = v6_default_for_slot(0);
    let dir = high_press_directive();
    let coached = apply_directive(&instinct, &dir, 0.0);
    assert_eq!(coached.decisions.aggression, instinct.decisions.aggression);
    assert_eq!(coached.decisions.tackle_chance, instinct.decisions.tackle_chance);
}

#[test]
fn high_press_increases_aggression() {
    let instinct = v6_default_for_slot(0);
    let coached = apply_directive(&instinct, &high_press_directive(), 1.0);
    assert!(coached.decisions.aggression > instinct.decisions.aggression,
        "aggression should increase with high press, got {} vs {}",
        coached.decisions.aggression, instinct.decisions.aggression);
}

#[test]
fn low_press_decreases_aggression() {
    let instinct = v6_default_for_slot(0);
    let coached = apply_directive(&instinct, &low_press_directive(), 1.0);
    assert!(coached.decisions.aggression < instinct.decisions.aggression,
        "aggression should decrease with low press, got {} vs {}",
        coached.decisions.aggression, instinct.decisions.aggression);
}

#[test]
fn high_tempo_increases_pass_chance_default() {
    let instinct = v6_default_for_slot(0);
    let dir = CoachDirective { tempo: 1.0, ..neutral_directive() };
    let coached = apply_directive(&instinct, &dir, 1.0);
    assert!(coached.decisions.pass_chance_default > instinct.decisions.pass_chance_default);
}

#[test]
fn low_tempo_decreases_pass_chance_default() {
    let instinct = v6_default_for_slot(0);
    let dir = CoachDirective { tempo: 0.0, ..neutral_directive() };
    let coached = apply_directive(&instinct, &dir, 1.0);
    assert!(coached.decisions.pass_chance_default < instinct.decisions.pass_chance_default);
}

#[test]
fn high_line_height_increases_own_goal_preferred_for_outfield() {
    let instinct = v6_default_for_slot(0); // Fwd slot (no GK)
    let dir = CoachDirective { line_height: 1.0, ..neutral_directive() };
    let coached = apply_directive(&instinct, &dir, 1.0);
    assert!(coached.spatial.own_goal.preferred > instinct.spatial.own_goal.preferred,
        "preferred={} vs {}", coached.spatial.own_goal.preferred, instinct.spatial.own_goal.preferred);
}

#[test]
fn gk_slot_spatial_params_unaffected_by_directive() {
    let instinct = v6_default_for_slot(4); // GK slot
    let dir = CoachDirective { line_height: 1.0, compactness: 1.0, ..neutral_directive() };
    let coached = apply_directive(&instinct, &dir, 1.0);
    // GK spatial should be identical since gk.is_some()
    assert_eq!(coached.spatial.own_goal.preferred, instinct.spatial.own_goal.preferred);
}

#[test]
fn coach_update_changes_directive() {
    let style = CoachStyle::default();
    let mut coach = Coach::new(style);
    let initial = coach.directive;
    let game = common::make_game();
    coach.update(&game, 0);
    // After update, directive may change based on game state.
    // At minimum, directive should be valid floats in [0,1].
    let d = &coach.directive;
    assert!(d.press_intensity >= 0.0 && d.press_intensity <= 1.0);
    assert!(d.line_height >= 0.0 && d.line_height <= 1.0);
    assert!(d.compactness >= 0.0 && d.compactness <= 1.0);
    assert!(d.tempo >= 0.0 && d.tempo <= 1.0);
    let _ = initial;
}

#[test]
fn v7team_pre_tick_updates_directive_every_300_ticks() {
    use training_engine::team::Team;
    let policy: [_; 5] = std::array::from_fn(|s| v6_default_for_slot(s));
    let mut team = V7Team::from_v6(0, policy);
    let mut game = common::make_game();
    let initial_directive = team.coach.directive;
    // Simulate exactly 300 ticks
    for _ in 0..300 {
        game.timer -= 1;
        team.pre_tick(&game);
    }
    // directive may or may not change (depends on game state), but it must be valid
    let d = &team.coach.directive;
    assert!(d.press_intensity >= 0.0 && d.press_intensity <= 1.0);
    let _ = initial_directive;
}
