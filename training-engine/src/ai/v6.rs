use rand::Rng;

use crate::constants::*;
use crate::game::{Game, PlayerState, Role};
use crate::policy::{V6Params, V6Spatial};
use crate::ai::helpers::move_to;
use crate::ai::movement::{
    chase_loose_ball, loose_ball_chaser, outfield_retreat_for_enemy_gk, run_set_piece_taker,
};
use crate::ai::decisions::classic_tick;
use crate::math::slow_factor;

pub fn nearest_active_dist(game: &Game, exclude_id: usize, want_team: Option<usize>, x: f32, y: f32) -> f32 {
    let mut best = f32::INFINITY;
    for q in &game.pl {
        if q.state != PlayerState::Active { continue; }
        if q.id == exclude_id { continue; }
        if let Some(t) = want_team { if q.team != t { continue; } }
        let d = (q.x - x).hypot(q.y - y);
        if d < best { best = d; }
    }
    if best.is_infinite() { 600.0 } else { best }
}

pub fn v6_total_cost(
    game: &Game, player_idx: usize, x: f32, y: f32,
    spatial: &V6Spatial,
) -> f32 {
    let p = &game.pl[player_idx];
    let own_goal_x = if p.team == 0 { FIELD_LINE } else { FW - FIELD_LINE };
    let d_own_goal = (x - own_goal_x).hypot(y - H2);
    let d_side = y;
    let d_ball = (x - game.ball.x).hypot(y - game.ball.y);
    let opp_team = 1 - p.team;
    let d_team = nearest_active_dist(game, p.id, Some(p.team), x, y);
    let d_opp  = nearest_active_dist(game, p.id, Some(opp_team), x, y);

    spatial.own_goal.cost(d_own_goal)
        + spatial.side.cost(d_side)
        + spatial.ball.cost(d_ball)
        + spatial.teammate.cost(d_team)
        + spatial.opponent.cost(d_opp)
}

pub fn v6_target(game: &Game, player_idx: usize, spatial: &V6Spatial) -> (f32, f32) {
    let p = &game.pl[player_idx];
    let r = 24.0_f32;
    let s = std::f32::consts::FRAC_1_SQRT_2 * r;
    let cands: [(f32, f32); 9] = [
        (p.x, p.y),
        (p.x + r, p.y), (p.x - r, p.y), (p.x, p.y + r), (p.x, p.y - r),
        (p.x + s, p.y + s), (p.x + s, p.y - s),
        (p.x - s, p.y + s), (p.x - s, p.y - s),
    ];
    let mut best = (p.x, p.y);
    let mut best_cost = f32::INFINITY;
    for (cx, cy) in cands {
        let cx = cx.clamp(PR, FW - PR);
        let cy = cy.clamp(PR, FH - PR);
        let c = v6_total_cost(game, player_idx, cx, cy, spatial);
        if c < best_cost { best_cost = c; best = (cx, cy); }
    }
    best
}

pub fn v6_tick(
    game: &mut Game, player_idx: usize, params: &V6Params, rng: &mut (impl Rng + ?Sized),
) {
    if outfield_retreat_for_enemy_gk(game, player_idx) { return; }

    let p_id = game.pl[player_idx].id;
    let p_team = game.pl[player_idx].team;
    let p_role = game.pl[player_idx].role;
    let ball_owner = game.ball.owner;
    let has_ball = ball_owner == Some(p_id);
    let mut policy = params.decisions.as_policy_params();
    let risk = params.decisions.risk_appetite.clamp(0.0, 1.0);
    policy.shoot_progress_threshold =
        (policy.shoot_progress_threshold - 0.05 * (risk - 0.5)).clamp(0.5, 0.95);

    if run_set_piece_taker(game, player_idx) { return; }

    if !has_ball {
        if let Some(c_id) = ball_owner {
            let c_team = game.pl[c_id].team;
            if c_team != p_team && game.pl[player_idx].tackle_cooldown <= 0 {
                let dist = (game.pl[player_idx].x - game.pl[c_id].x)
                    .hypot(game.pl[player_idx].y - game.pl[c_id].y);
                let chance = (policy.tackle_chance * params.decisions.aggression).clamp(0.01, 0.5);
                if dist < TACKLE_DIST && rng.gen::<f32>() < chance {
                    crate::physics::tackle_player(game, player_idx, c_id);
                    return;
                }
            }
        }
    }

    if p_role == Role::Gk {
        let gkp = params.gk.unwrap_or_default();
        let line_x = crate::gk::gk_line_x(p_team);

        if game.pl[player_idx].gk_dive_timer < 0 { return; }

        if has_ball {
            crate::gk::distribute_ball(game, player_idx, &gkp, rng);
            return;
        }

        crate::gk::maybe_start_dive(game, player_idx, &gkp, rng);
        if crate::gk::continue_dive(game, player_idx) { return; }

        let (mut tx, mut ty) = v6_target(game, player_idx, &params.spatial);

        let max_zone = (FW * 0.5 - line_x).abs();
        let max_out = params.spatial.own_goal.max
            .clamp(0.0, max_zone)
            * gkp.gk_sweeper_freedom.clamp(0.0, 1.0);

        if p_team == 0 {
            tx = tx.clamp(line_x, line_x + max_out);
        } else {
            tx = tx.clamp(line_x - max_out, line_x);
        }
        ty = ty.clamp(PR, FH - PR);

        move_to(&mut game.pl[player_idx], tx, ty, CSPEED * 0.88);
        return;
    }

    if has_ball {
        let hooks = crate::brain::TickHooks {
            pass_dir_mult: [
                params.decisions.pass_dir_offensive.clamp(0.0, 2.0),
                params.decisions.pass_dir_defensive.clamp(0.0, 2.0),
                params.decisions.pass_dir_neutral.clamp(0.0, 2.0),
            ],
            gk_freedom: 0.0,
            max_distance_from_goal: 1.0,
            ..crate::brain::TickHooks::default()
        };
        classic_tick(game, player_idx, &policy, &hooks, rng);
        return;
    }

    if ball_owner.is_none() && loose_ball_chaser(game) == Some(p_id) {
        chase_loose_ball(game, player_idx, slow_factor(&game.pl[player_idx]));
        return;
    }

    let (tx, ty) = v6_target(game, player_idx, &params.spatial);
    let speed = CSPEED * slow_factor(&game.pl[player_idx]);
    move_to(&mut game.pl[player_idx], tx, ty, speed);
}
