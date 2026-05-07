use rand::Rng;

use crate::constants::*;
use crate::game::{Game, PlayerState, Role};
use crate::policy::PolicyParams;
use crate::ai::helpers::{
    attack_progress, is_marked, move_to, nearest_opponent_distance,
    opp_goal_point, pass_line_open, team_dir, wing_y,
};
use crate::ai::movement::{
    chase_loose_ball, get_attack_target, get_defend_target, get_loose_ball_support,
    loose_ball_chaser, natural_target, outfield_retreat_for_enemy_gk, run_set_piece_taker,
};
use crate::math::slow_factor;

pub struct PassResult {
    pub target_id: usize,
    pub tx: f32,
    pub ty: f32,
}

pub fn cpu_find_pass(game: &Game, carrier_idx: usize, params: &PolicyParams) -> Option<PassResult> {
    let carrier = &game.pl[carrier_idx];
    let opp_goal_x = if carrier.team == 0 { FW } else { 0.0 };
    let mut best: Option<(f32, PassResult)> = None;

    for p in &game.pl {
        if p.team != carrier.team || p.id == carrier.id || p.state != PlayerState::Active { continue; }
        if is_marked(game, p, params.mark_distance) { continue; }
        if !pass_line_open(game, carrier.x, carrier.y, p.x, p.y, carrier.team) { continue; }

        let forward_gain = (p.x - carrier.x) * team_dir(carrier.team);
        let gain = (carrier.x - opp_goal_x).abs() - (p.x - opp_goal_x).abs();
        let width = (p.y - H2).abs();
        let in_front_of_goal = if p.role == Role::Fwd { 85.0 } else { 0.0 };
        let wing_bonus = if p.role == Role::Mid { 70.0 + width * 0.35 } else { 0.0 };
        let carrier_past_mid = (carrier.team == 0 && carrier.x > FW * 0.50)
            || (carrier.team == 1 && carrier.x < FW * 0.50);
        let cutback_bonus = if carrier.role == Role::Mid && carrier_past_mid {
            in_front_of_goal + if (p.y - H2).abs() < 75.0 { 55.0 } else { 0.0 }
        } else { 0.0 };
        let central_carrier_bonus = if (carrier.y - H2).abs() < 55.0 && p.role == Role::Mid { 110.0 } else { 0.0 };
        let min_gain = params.forward_pass_min_gain;
        let forward_bonus = if forward_gain > min_gain {
            150.0 + forward_gain * 1.15
        } else {
            forward_gain * 8.0
        };
        let dist = (p.x - carrier.x).hypot(p.y - carrier.y);
        let score = gain + forward_bonus + wing_bonus + cutback_bonus + central_carrier_bonus - dist * 0.05;

        if best.as_ref().map_or(true, |(s, _)| score > *s) {
            best = Some((score, PassResult { target_id: p.id, tx: p.x, ty: p.y }));
        }
    }

    best.map(|(_, r)| r)
}

pub fn classic_tick(
    game: &mut Game, player_idx: usize, params: &PolicyParams,
    hooks: &crate::brain::TickHooks, rng: &mut (impl Rng + ?Sized),
) {
    let p_id = game.pl[player_idx].id;
    let p_team = game.pl[player_idx].team;
    let p_role = game.pl[player_idx].role;

    let ball_owner = game.ball.owner;
    let has_ball = ball_owner == Some(p_id);
    let carrier_team = ball_owner.and_then(|id| game.pl.get(id)).map(|c| c.team);
    let team_has_ball = carrier_team == Some(p_team);

    if run_set_piece_taker(game, player_idx) { return; }

    if !has_ball {
        if let Some(c_id) = ball_owner {
            let c_team = game.pl[c_id].team;
            if c_team != p_team && game.pl[player_idx].tackle_cooldown <= 0 {
                let dist = (game.pl[player_idx].x - game.pl[c_id].x)
                    .hypot(game.pl[player_idx].y - game.pl[c_id].y);
                if dist < TACKLE_DIST && rng.gen::<f32>() < params.tackle_chance {
                    crate::physics::tackle_player(game, player_idx, c_id);
                    return;
                }
            }
        }
    }

    if p_role == Role::Gk {
        if game.pl[player_idx].gk_dive_timer < 0 { return; }

        if has_ball {
            crate::gk::distribute_ball(game, player_idx, hooks, rng);
        } else {
            crate::gk::maybe_start_dive(game, player_idx, hooks, rng);
            if crate::gk::continue_dive(game, player_idx) { return; }

            let line_x = crate::gk::gk_line_x(p_team);
            let by = game.ball.y;
            let line_y = by.max(H2 - GH / 2.0 + PR).min(H2 + GH / 2.0 - PR);

            let freedom = hooks.gk_freedom.clamp(0.0, 1.0);
            let (target_x, target_y) = if freedom < 1e-3 {
                (line_x, line_y)
            } else {
                let goal_center_x = crate::gk::gk_goal_x(p_team);
                let max_out = (FW * 0.5 - line_x).abs() * freedom;
                let dx = game.ball.x - goal_center_x;
                let dy = game.ball.y - H2;
                let dist = dx.hypot(dy).max(1.0);
                let step = max_out.min(dist - PR);
                let tx = if p_team == 0 {
                    (goal_center_x + dx / dist * step).max(line_x)
                } else {
                    (goal_center_x + dx / dist * step).min(line_x)
                };
                let ty = (H2 + dy / dist * step).clamp(PR, FH - PR);
                (tx, ty)
            };
            move_to(&mut game.pl[player_idx], target_x, target_y, CSPEED * 0.88);
        }
        return;
    }

    if outfield_retreat_for_enemy_gk(game, player_idx) { return; }

    if ball_owner.is_none() {
        let chaser_id = loose_ball_chaser(game);
        if chaser_id == Some(p_id) {
            chase_loose_ball(game, player_idx, 1.0);
        } else {
            let (sx, sy) = get_loose_ball_support(game, &game.pl[player_idx]);
            let amp = if p_role == Role::Def { 7.0 } else { 15.0 };
            let (ntx, nty) = natural_target(&mut game.pl[player_idx], sx, sy, amp, rng);
            move_to(&mut game.pl[player_idx], ntx, nty, CSPEED * 0.78);
        }
        return;
    }

    if has_ball {
        if game.free_kick_active && game.free_kick_shooter_id == Some(p_id) {
            let pass_opt = cpu_find_pass(game, player_idx, params);
            if let Some(pt) = pass_opt {
                crate::physics::do_shoot(game, player_idx, false, pt.tx, pt.ty, Some(CPU_PASS_POW), true);
            }
            return;
        }
        let (opp_gx, _) = opp_goal_point(p_team);
        let in_shoot_zone = attack_progress(p_team, game.pl[player_idx].x) > params.shoot_progress_threshold;
        let reached_half = if p_team == 0 { game.pl[player_idx].x > FW * 0.50 } else { game.pl[player_idx].x < FW * 0.50 };
        let on_wing = p_role == Role::Mid && (game.pl[player_idx].y - wing_y(game.pl[player_idx].home_y)).abs() < 54.0;
        let pressured = nearest_opponent_distance(game, &game.pl[player_idx]) < 72.0;

        let pass_chance = if pressured {
            params.pass_chance_pressured
        } else if on_wing {
            params.pass_chance_wing
        } else if p_role == Role::Fwd {
            params.pass_chance_forward
        } else {
            params.pass_chance_default
        };

        let pass_opt = cpu_find_pass(game, player_idx, params);
        let forward_pass = pass_opt.as_ref().and_then(|pt| {
            let gain = (pt.tx - game.pl[player_idx].x) * team_dir(p_team);
            if gain > params.forward_pass_min_gain { Some((pt.tx, pt.ty)) } else { None }
        });
        let safe_pass = if pressured { pass_opt.as_ref().map(|pt| (pt.tx, pt.ty)) } else { forward_pass };

        let dir_threshold: f32 = 30.0;
        let pass_chance = if let Some((tx, _ty)) = safe_pass {
            let gain = (tx - game.pl[player_idx].x) * team_dir(p_team);
            let mult = if gain > dir_threshold { hooks.pass_dir_mult[0] }
                       else if gain < -dir_threshold { hooks.pass_dir_mult[1] }
                       else { hooks.pass_dir_mult[2] };
            (pass_chance * mult).clamp(0.0, 1.0)
        } else { pass_chance };

        if p_role == Role::Mid && !reached_half {
            let dir = team_dir(p_team);
            let lane_x = (game.pl[player_idx].x + dir * 100.0).clamp(PR, FW - PR);
            let wy = wing_y(game.pl[player_idx].home_y);
            let (ntx, nty) = natural_target(&mut game.pl[player_idx], lane_x, wy, 10.0, rng);
            if forward_pass.is_some() && rng.gen::<f32>() < 0.04 {
                let (fpx, fpy) = forward_pass.unwrap();
                crate::physics::do_shoot(game, player_idx, false, fpx, fpy, Some(CPU_PASS_POW), true);
            } else {
                move_to(&mut game.pl[player_idx], ntx, nty, CSPEED * 0.92);
            }
        } else if let Some((spx, spy)) = safe_pass {
            if rng.gen::<f32>() < pass_chance && (!in_shoot_zone || p_role != Role::Fwd || rng.gen::<f32>() < 0.45) {
                crate::physics::do_shoot(game, player_idx, false, spx, spy, Some(CPU_PASS_POW), true);
            } else if in_shoot_zone && (p_role == Role::Fwd || rng.gen::<f32>() < 0.42) {
                let ty = H2 + (game.pl[player_idx].y - H2) * 0.22;
                crate::physics::do_shoot(game, player_idx, false, opp_gx, ty, None, false);
            } else {
                let press_r = if pressured { 0.09 } else if on_wing { 0.04 } else { 0.025 };
                if rng.gen::<f32>() < press_r {
                    crate::physics::do_shoot(game, player_idx, false, spx, spy, Some(CPU_PASS_POW), true);
                } else {
                    let carry_y = if p_role == Role::Mid { wing_y(game.pl[player_idx].home_y) } else { H2 + (game.ball.y - H2) * 0.22 };
                    let dir = team_dir(p_team);
                    let cx = (game.pl[player_idx].x + dir * 85.0).clamp(PR, FW - PR);
                    let amp = if p_role == Role::Mid { 10.0 } else { 18.0 };
                    let (ntx, nty) = natural_target(&mut game.pl[player_idx], cx, carry_y, amp, rng);
                    move_to(&mut game.pl[player_idx], ntx, nty, CSPEED);
                }
            }
        } else if in_shoot_zone && (p_role == Role::Fwd || rng.gen::<f32>() < 0.42) {
            let ty = H2 + (game.pl[player_idx].y - H2) * 0.22;
            crate::physics::do_shoot(game, player_idx, false, opp_gx, ty, None, false);
        } else {
            let carry_y = if p_role == Role::Mid { wing_y(game.pl[player_idx].home_y) } else { H2 + (game.ball.y - H2) * 0.22 };
            let dir = team_dir(p_team);
            let cx = (game.pl[player_idx].x + dir * 85.0).clamp(PR, FW - PR);
            let amp = if p_role == Role::Mid { 10.0 } else { 18.0 };
            let (ntx, nty) = natural_target(&mut game.pl[player_idx], cx, carry_y, amp, rng);
            move_to(&mut game.pl[player_idx], ntx, nty, CSPEED);
        }
        return;
    }

    let (tx, ty) = if team_has_ball {
        get_attack_target(game, &game.pl[player_idx])
    } else {
        get_defend_target(game, &game.pl[player_idx], rng)
    };
    let loose = if p_role == Role::Def || p_role == Role::Gk { 7.0 } else { 18.0 };
    let (ntx, nty) = natural_target(&mut game.pl[player_idx], tx, ty, loose, rng);
    let spd = (if team_has_ball { CSPEED * 0.82 } else { CSPEED }) * slow_factor(&game.pl[player_idx]);
    move_to(&mut game.pl[player_idx], ntx, nty, spd);
}
