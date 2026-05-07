mod common;

use rand::SeedableRng;
use rand::rngs::SmallRng;
use training_engine::policy::v6::{
    mutate_team_v6, mutate_v6, v6_default_for_slot, GkDecisionParams, V6Params,
};
use training_engine::policy::v7::mutate_v7;
use training_engine::team_v7::CoachStyle;
use training_engine::policy::v7::V7TeamParams;

fn rng() -> SmallRng {
    SmallRng::seed_from_u64(42)
}

#[test]
fn mutate_v6_produces_different_params() {
    let original = v6_default_for_slot(0);
    // Run several mutations with different seeds — at least one should produce a change.
    let changed = (0u64..10).any(|seed| {
        let mut r = rand::rngs::SmallRng::seed_from_u64(seed);
        let m = mutate_v6(&original, &mut r, 1.0);
        m.decisions.pass_chance_default != original.decisions.pass_chance_default
            || m.decisions.tackle_chance != original.decisions.tackle_chance
            || m.spatial.ball.preferred != original.spatial.ball.preferred
            || m.spatial.own_goal.preferred != original.spatial.own_goal.preferred
            || m.decisions.aggression != original.decisions.aggression
    });
    assert!(changed, "at least one of 10 mutations should change some param");
}

#[test]
fn mutate_v6_all_fields_in_valid_range() {
    let original = v6_default_for_slot(0);
    let mutated = mutate_v6(&original, &mut rng(), 1.0);
    let d = &mutated.decisions;
    assert!(d.pass_chance_default >= 0.005 && d.pass_chance_default <= 0.2);
    assert!(d.tackle_chance >= 0.01 && d.tackle_chance <= 0.22);
    assert!(d.shoot_progress_threshold >= 0.55 && d.shoot_progress_threshold <= 0.9);
    assert!(d.aggression >= 0.0 && d.aggression <= 2.0);
    assert!(d.risk_appetite >= 0.0 && d.risk_appetite <= 1.0);
}

#[test]
fn mutate_v6_scale_zero_gives_minimal_change() {
    let original = v6_default_for_slot(1);
    // scale is clamped to 0.05 minimum, so there may be tiny changes
    let mutated = mutate_v6(&original, &mut rng(), 0.0);
    let delta = (mutated.decisions.aggression - original.decisions.aggression).abs();
    assert!(delta < 0.5, "scale≈0 should produce tiny changes, got delta={delta}");
}

#[test]
fn mutate_team_v6_changes_at_least_one_slot() {
    let team: [V6Params; 5] = std::array::from_fn(|s| v6_default_for_slot(s));
    let mutated = mutate_team_v6(&team, &mut rng(), 1.0);
    let any_changed = (0..5).any(|i| {
        team[i].decisions.pass_chance_default != mutated[i].decisions.pass_chance_default
            || team[i].spatial.ball.preferred != mutated[i].spatial.ball.preferred
    });
    assert!(any_changed, "at least one slot should be mutated");
}

#[test]
fn v6_default_slot4_has_gk_params() {
    let p = v6_default_for_slot(4);
    assert!(p.gk.is_some(), "slot 4 (GK) should have GkDecisionParams");
}

#[test]
fn v6_default_slot0_has_no_gk_params() {
    let p = v6_default_for_slot(0);
    assert!(p.gk.is_none(), "slot 0 (Fwd) should not have GkDecisionParams");
}

#[test]
fn mutate_v7_coachability_clamped_to_range() {
    let base: [V6Params; 5] = std::array::from_fn(|s| v6_default_for_slot(s));
    let params = V7TeamParams { instinct: base, coachability: [0.5; 5], coach_style: CoachStyle::default() };
    let mutated = mutate_v7(&params, &mut rng(), 1.0);
    for c in mutated.coachability {
        assert!(c >= 0.05 && c <= 0.95, "coachability={c} out of [0.05, 0.95]");
    }
}

#[test]
fn mutate_v7_coach_style_fields_in_range() {
    let base: [V6Params; 5] = std::array::from_fn(|s| v6_default_for_slot(s));
    let params = V7TeamParams { instinct: base, coachability: [0.5; 5], coach_style: CoachStyle::default() };
    let mutated = mutate_v7(&params, &mut rng(), 1.0);
    let s = &mutated.coach_style;
    assert!(s.press_response >= 0.0 && s.press_response <= 1.0);
    assert!(s.depth_response >= 0.0 && s.depth_response <= 1.0);
    assert!(s.compactness_base >= 0.0 && s.compactness_base <= 1.0);
    assert!(s.tempo_base >= 0.0 && s.tempo_base <= 1.0);
}
