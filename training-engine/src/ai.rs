use rand::Rng;

use crate::constants::*;
use crate::game::{effective_policy, Game, Player, PlayerState, Role};

fn clamp(v: f32, lo: f32, hi: f32) -> f32 {
    v.max(lo).min(hi)
}

fn norm(dx: f32, dy: f32) -> (f32, f32) {
    let m = dx.hypot(dy);
    if m < 1e-9 { (0.0, 0.0) } else { (dx / m, dy / m) }
}

pub fn move_to(p: &mut Player, tx: f32, ty: f32, speed: f32) {
    let (nx, ny) = norm(tx - p.x, ty - p.y);
    p.x = clamp(p.x + nx * speed, PR, FW - PR);
    p.y = clamp(p.y + ny * speed, PR, FH - PR);
}

fn team_dir(team: usize) -> f32 {
    if team == 0 { 1.0 } else { -1.0 }
}

pub fn attack_progress(team: usize, x: f32) -> f32 {
    if team == 0 { x / FW } else { 1.0 - x / FW }
}

fn opp_goal_point(team: usize) -> (f32, f32) {
    if team == 0 { (FW + GD, H2) } else { (-GD, H2) }
}

fn own_goal_point(team: usize) -> (f32, f32) {
    if team == 0 { (0.0, H2) } else { (FW, H2) }
}

fn side_of(home_y: f32) -> f32 {
    if home_y < H2 { -1.0 } else { 1.0 }
}

fn wing_y(home_y: f32) -> f32 {
    if side_of(home_y) < 0.0 { 58.0 } else { FH - 58.0 }
}

fn point_between(ax: f32, ay: f32, bx: f32, by: f32, t: f32) -> (f32, f32) {
    (ax + (bx - ax) * t, ay + (by - ay) * t)
}

fn dist_to_segment(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let vx = bx - ax;
    let vy = by - ay;
    let len2 = vx * vx + vy * vy;
    let len2 = if len2 < 1e-9 { 1.0 } else { len2 };
    let t = clamp(((px - ax) * vx + (py - ay) * vy) / len2, 0.0, 1.0);
    let sx = ax + vx * t;
    let sy = ay + vy * t;
    (px - sx).hypot(py - sy)
}

fn pass_line_open(game: &Game, fx: f32, fy: f32, tx: f32, ty: f32, team: usize) -> bool {
    !game.pl.iter().any(|q| {
        if q.team == team || q.state != PlayerState::Active { return false; }
        dist_to_segment(q.x, q.y, fx, fy, tx, ty) < PASS_BLOCK_DIST
    })
}

fn is_marked(game: &Game, p: &Player, threshold: f32) -> bool {
    game.pl.iter().any(|q| {
        q.team != p.team && q.state == PlayerState::Active
            && (q.x - p.x).hypot(q.y - p.y) < threshold
    })
}

fn nearest_opponent_distance(game: &Game, p: &Player) -> f32 {
    game.pl.iter()
        .filter(|q| q.team != p.team && q.state == PlayerState::Active)
        .map(|q| (q.x - p.x).hypot(q.y - p.y))
        .fold(f32::INFINITY, f32::min)
}

fn shape_x_with_ball(home_x: f32, ball_x: f32, strength: f32) -> f32 {
    clamp(home_x + (ball_x - FW / 2.0) * strength, PR, FW - PR)
}

fn get_loose_ball_support(game: &Game, p: &Player) -> (f32, f32) {
    let bx = game.ball.x;
    let by = game.ball.y;
    match p.role {
        Role::Fwd => {
            let sx = shape_x_with_ball(p.home_x, bx, 0.58);
            let x = if p.team == 0 { sx.max(FW * 0.58) } else { sx.min(FW * 0.42) };
            let y = H2 + (p.home_y - H2) * 0.45;
            (x, y)
        }
        Role::Mid => {
            let x = shape_x_with_ball(p.home_x, bx, 0.7);
            let y = H2 + (p.home_y - H2) * 0.78 + (by - H2) * 0.16;
            (x, y)
        }
        Role::Def => {
            let sx = shape_x_with_ball(p.home_x, bx, 0.42);
            let x = if p.team == 0 { sx.min(FW * 0.43) } else { sx.max(FW * 0.57) };
            let y = H2 + (by - H2) * 0.18;
            (x, y)
        }
        Role::Gk => (p.home_x, p.home_y),
    }
}

fn defensive_block_target(p: &Player, cx: f32, cy: f32) -> (f32, f32) {
    let (gx, gy) = own_goal_point(p.team);
    let t = if p.role == Role::Def { 0.38 } else { 0.55 };
    point_between(gx, gy, cx, cy, t)
}

fn best_interception_target(game: &Game, p: &Player, carrier_id: usize, cx: f32, cy: f32) -> Option<(f32, f32)> {
    let mut best: Option<(f32, f32)> = None;
    let mut best_score = f32::NEG_INFINITY;
    for opp in &game.pl {
        if opp.team == p.team || opp.id == carrier_id || opp.state != PlayerState::Active { continue; }
        let d = dist_to_segment(p.x, p.y, cx, cy, opp.x, opp.y);
        let pass_lane = dist_to_segment(opp.x, opp.y, cx, cy, p.x, p.y);
        let bonus = if opp.role == Role::Fwd { 35.0 } else { 0.0 };
        let score = -d - pass_lane * 0.35 + bonus;
        if score > best_score {
            best_score = score;
            best = Some(point_between(cx, cy, opp.x, opp.y, 0.48));
        }
    }
    best
}

fn natural_target(p: &mut Player, tx: f32, ty: f32, amp: f32, rng: &mut impl Rng) -> (f32, f32) {
    p.ai_jitter_timer -= 1;
    if p.ai_jitter_timer <= 0 {
        p.ai_jitter_x = (rng.gen::<f32>() * 2.0 - 1.0) * amp;
        p.ai_jitter_y = (rng.gen::<f32>() * 2.0 - 1.0) * amp;
        p.ai_jitter_timer = 35 + rng.gen_range(0..55);
    }
    (
        clamp(tx + p.ai_jitter_x, PR, FW - PR),
        clamp(ty + p.ai_jitter_y, PR, FH - PR),
    )
}

fn get_attack_target(game: &Game, p: &Player) -> (f32, f32) {
    let (cx, cy) = match game.ball.owner.and_then(|id| game.pl.get(id)) {
        Some(c) => (c.x, c.y),
        None => return (p.home_x, p.home_y),
    };
    let bx = game.ball.x;
    let by = game.ball.y;
    match p.role {
        Role::Fwd => {
            let sx = shape_x_with_ball(p.home_x, bx, 0.62);
            let run_x = if p.team == 0 { sx.max(FW * 0.62) } else { sx.min(FW * 0.38) };
            let open_y = if pass_line_open(game, cx, cy, run_x, H2 - 42.0, p.team) { H2 - 42.0 } else { H2 + 42.0 };
            (run_x, open_y)
        }
        Role::Mid => {
            let dir = team_dir(p.team);
            let follow_x = shape_x_with_ball(p.home_x, bx, 0.72);
            let support_x = cx + dir * 76.0;
            let lane_x = clamp((follow_x + support_x) / 2.0, FW * 0.16, FW * 0.84);
            (lane_x, wing_y(p.home_y))
        }
        Role::Def => {
            let sx = shape_x_with_ball(p.home_x, bx, 0.38);
            let x = if p.team == 0 { sx.min(FW * 0.43) } else { sx.max(FW * 0.57) };
            (x, H2)
        }
        Role::Gk => (p.home_x, p.home_y),
    }
}

fn get_defend_target(game: &Game, p: &Player, rng: &mut impl Rng) -> (f32, f32) {
    let bx = game.ball.x;
    let by = game.ball.y;
    let opp_carrier = game.ball.owner
        .and_then(|id| game.pl.get(id))
        .filter(|c| c.team != p.team);

    let own_goal_x = if p.team == 0 { 0.0 } else { FW };

    match p.role {
        Role::Fwd => {
            if let Some(c) = opp_carrier { (c.x, c.y) } else { (bx, by) }
        }
        Role::Def => {
            let (cx, cy) = opp_carrier.map(|c| (c.x, c.y)).unwrap_or((bx, by));
            defensive_block_target(p, cx, cy)
        }
        Role::Mid => {
            if let Some(c) = opp_carrier {
                if (p.x - c.x).hypot(p.y - c.y) < 190.0 {
                    let block = defensive_block_target(p, c.x, c.y);
                    let intercept = best_interception_target(game, p, c.id, c.x, c.y);
                    if intercept.is_some() && rng.gen::<f32>() < 0.45 {
                        return intercept.unwrap();
                    }
                    return block;
                }
            }
            let ratio = 0.64;
            let bx2 = own_goal_x + (bx - own_goal_x) * ratio;
            let by2 = H2 + (by - H2) * 0.38 + (p.home_y - H2) * 0.34;
            (bx2, clamp(by2, PR, FH - PR))
        }
        Role::Gk => (p.home_x, p.home_y),
    }
}

pub struct PassResult {
    pub target_id: usize,
    pub tx: f32,
    pub ty: f32,
}

pub fn cpu_find_pass(game: &Game, carrier_idx: usize) -> Option<PassResult> {
    let carrier = &game.pl[carrier_idx];
    let params = effective_policy(game, carrier_idx);
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

fn loose_ball_chaser(game: &Game) -> Option<usize> {
    game.pl.iter()
        .filter(|p| p.role != Role::Gk && p.state == PlayerState::Active)
        .min_by(|a, b| {
            let da = (a.x - game.ball.x).hypot(a.y - game.ball.y);
            let db = (b.x - game.ball.x).hypot(b.y - game.ball.y);
            da.partial_cmp(&db).unwrap()
        })
        .map(|p| p.id)
}

pub fn baseline_cpu_tick(game: &mut Game, player_idx: usize, rng: &mut impl Rng) {
    let p_id = game.pl[player_idx].id;
    let p_team = game.pl[player_idx].team;
    let p_role = game.pl[player_idx].role;

    let ball_owner = game.ball.owner;
    let has_ball = ball_owner == Some(p_id);
    let carrier_team = ball_owner.and_then(|id| game.pl.get(id)).map(|c| c.team);
    let team_has_ball = carrier_team == Some(p_team);

    // Attempt tackle
    if !has_ball {
        if let Some(c_id) = ball_owner {
            let c_team = game.pl[c_id].team;
            if c_team != p_team && game.pl[player_idx].tackle_cooldown <= 0 {
                let dist = (game.pl[player_idx].x - game.pl[c_id].x)
                    .hypot(game.pl[player_idx].y - game.pl[c_id].y);
                if dist < TACKLE_DIST && rng.gen::<f32>() < effective_policy(game, player_idx).tackle_chance {
                    crate::physics::tackle_player(game, player_idx, c_id);
                    return;
                }
            }
        }
    }

    // Goalkeeper
    if p_role == Role::Gk {
        if has_ball {
            let (_, _) = (FW / 2.0, H2);
            crate::physics::do_shoot(game, player_idx, false, FW / 2.0, H2, None, false);
        } else {
            let gx = if p_team == 0 { PR * 1.8 } else { FW - PR * 1.8 };
            let by = game.ball.y;
            let target_y = by.max(H2 - GH / 2.0 + PR).min(H2 + GH / 2.0 - PR);
            let (px, py) = (game.pl[player_idx].x, game.pl[player_idx].y);
            let _ = (px, py);
            move_to(&mut game.pl[player_idx], gx, target_y, CSPEED * 0.88);
        }
        return;
    }

    // Loose ball
    if ball_owner.is_none() {
        let chaser_id = loose_ball_chaser(game);
        if chaser_id == Some(p_id) {
            let lead = {
                let bvx = game.ball.vx;
                let bvy = game.ball.vy;
                18.0_f32.min(bvx.hypot(bvy) * 1.4)
            };
            let tx = clamp(game.ball.x + game.ball.vx * lead, PR, FW - PR);
            let ty = clamp(game.ball.y + game.ball.vy * lead, PR, FH - PR);
            move_to(&mut game.pl[player_idx], tx, ty, CSPEED * 1.18);
        } else {
            let (sx, sy) = get_loose_ball_support(game, &game.pl[player_idx]);
            let amp = if p_role == Role::Def { 7.0 } else { 15.0 };
            let (ntx, nty) = natural_target(&mut game.pl[player_idx], sx, sy, amp, rng);
            move_to(&mut game.pl[player_idx], ntx, nty, CSPEED * 0.78);
        }
        return;
    }

    // Has ball
    if has_ball {
        let params = effective_policy(game, player_idx);
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

        let pass_opt = cpu_find_pass(game, player_idx);
        let forward_pass = pass_opt.as_ref().and_then(|pt| {
            let gain = (pt.tx - game.pl[player_idx].x) * team_dir(p_team);
            if gain > params.forward_pass_min_gain { Some((pt.tx, pt.ty)) } else { None }
        });
        let safe_pass = if pressured { pass_opt.as_ref().map(|pt| (pt.tx, pt.ty)) } else { forward_pass };

        if p_role == Role::Mid && !reached_half {
            let dir = team_dir(p_team);
            let lane_x = clamp(game.pl[player_idx].x + dir * 100.0, PR, FW - PR);
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
                    let cx = clamp(game.pl[player_idx].x + dir * 85.0, PR, FW - PR);
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
            let cx = clamp(game.pl[player_idx].x + dir * 85.0, PR, FW - PR);
            let amp = if p_role == Role::Mid { 10.0 } else { 18.0 };
            let (ntx, nty) = natural_target(&mut game.pl[player_idx], cx, carry_y, amp, rng);
            move_to(&mut game.pl[player_idx], ntx, nty, CSPEED);
        }
        return;
    }

    // Support / defend
    let (tx, ty) = if team_has_ball {
        get_attack_target(game, &game.pl[player_idx])
    } else {
        get_defend_target(game, &game.pl[player_idx], rng)
    };
    let loose = if p_role == Role::Def || p_role == Role::Gk { 7.0 } else { 18.0 };
    let (ntx, nty) = natural_target(&mut game.pl[player_idx], tx, ty, loose, rng);
    let spd = if team_has_ball { CSPEED * 0.82 } else { CSPEED };
    move_to(&mut game.pl[player_idx], ntx, nty, spd);
}
