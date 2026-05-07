use rand::Rng;

use crate::constants::*;
use crate::game::{Game, Player, PlayerState, Role};
use crate::ai::helpers::{
    move_to, own_goal_point, point_between, dist_to_segment,
    pass_line_open, team_dir, wing_y,
};
use crate::math::slow_factor;

pub fn shape_x_with_ball(home_x: f32, ball_x: f32, strength: f32) -> f32 {
    (home_x + (ball_x - FW / 2.0) * strength).clamp(PR, FW - PR)
}

pub fn get_loose_ball_support(game: &Game, p: &Player) -> (f32, f32) {
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

pub fn defensive_block_target(p: &Player, cx: f32, cy: f32) -> (f32, f32) {
    let (gx, gy) = own_goal_point(p.team);
    let t = if p.role == Role::Def { 0.38 } else { 0.55 };
    point_between(gx, gy, cx, cy, t)
}

pub fn best_interception_target(game: &Game, p: &Player, carrier_id: usize, cx: f32, cy: f32) -> Option<(f32, f32)> {
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

pub fn natural_target(p: &mut Player, tx: f32, ty: f32, amp: f32, rng: &mut (impl Rng + ?Sized)) -> (f32, f32) {
    p.ai_jitter_timer -= 1;
    if p.ai_jitter_timer <= 0 {
        p.ai_jitter_x = (rng.gen::<f32>() * 2.0 - 1.0) * amp;
        p.ai_jitter_y = (rng.gen::<f32>() * 2.0 - 1.0) * amp;
        p.ai_jitter_timer = 35 + rng.gen_range(0..55);
    }
    (
        (tx + p.ai_jitter_x).clamp(PR, FW - PR),
        (ty + p.ai_jitter_y).clamp(PR, FH - PR),
    )
}

pub fn get_attack_target(game: &Game, p: &Player) -> (f32, f32) {
    let (cx, cy) = match game.ball.owner.and_then(|id| game.pl.get(id)) {
        Some(c) => (c.x, c.y),
        None => return (p.home_x, p.home_y),
    };
    let bx = game.ball.x;
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
            let lane_x = ((follow_x + support_x) / 2.0).clamp(FW * 0.16, FW * 0.84);
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

pub fn get_defend_target(game: &Game, p: &Player, rng: &mut (impl Rng + ?Sized)) -> (f32, f32) {
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
            (bx2, by2.clamp(PR, FH - PR))
        }
        Role::Gk => (p.home_x, p.home_y),
    }
}

pub fn loose_ball_chaser(game: &Game) -> Option<usize> {
    game.pl.iter()
        .filter(|p| p.role != Role::Gk && p.state == PlayerState::Active)
        .min_by(|a, b| {
            let da = (a.x - game.ball.x).hypot(a.y - game.ball.y);
            let db = (b.x - game.ball.x).hypot(b.y - game.ball.y);
            da.partial_cmp(&db).unwrap()
        })
        .map(|p| p.id)
}

// ── Delade tick-helpers ──────────────────────────────────────────────────────
// Används både av `classic_tick` och `v6_tick`.

/// Spring mot bollen om denna spelare är utsedd set-piece-taker.
/// Returnerar true om åtgärd vidtogs (anropare ska då returnera).
pub fn run_set_piece_taker(game: &mut Game, player_idx: usize) -> bool {
    let p_id = game.pl[player_idx].id;
    let has_ball = game.ball.owner == Some(p_id);
    if game.set_piece_taker_id != Some(p_id) || has_ball { return false; }
    let slow = slow_factor(&game.pl[player_idx]);
    let bx = game.ball.x;
    let by = game.ball.y;
    move_to(&mut game.pl[player_idx], bx, by, CSPEED * 1.18 * slow);
    true
}

/// Utespelare retirerar mot egen halva när motståndarens GK håller boll.
/// No-op om spelaren själv är GK eller motståndarens GK inte håller boll.
/// Returnerar true om retirering applicerades.
pub fn outfield_retreat_for_enemy_gk(game: &mut Game, player_idx: usize) -> bool {
    let p_team = game.pl[player_idx].team;
    let p_role = game.pl[player_idx].role;
    if p_role == Role::Gk { return false; }
    let enemy_team = 1 - p_team;
    if !game.gk_has_ball[enemy_team] { return false; }
    let retreat_x = if p_team == 0 {
        game.pl[player_idx].x.min(FW / 2.0 - PR)
    } else {
        game.pl[player_idx].x.max(FW / 2.0 + PR)
    };
    let cur_y = game.pl[player_idx].y;
    move_to(&mut game.pl[player_idx], retreat_x, cur_y, CSPEED * 0.9);
    true
}

/// Förflyttar spelaren mot bollens predikterade position med leading.
/// `speed_mult` skalar farten — t.ex. 1.0 för classic-tick, slow_factor för v6.
pub fn chase_loose_ball(game: &mut Game, player_idx: usize, speed_mult: f32) {
    let bvx = game.ball.vx;
    let bvy = game.ball.vy;
    let lead = 18.0_f32.min(bvx.hypot(bvy) * 1.4);
    let tx = (game.ball.x + bvx * lead).clamp(PR, FW - PR);
    let ty = (game.ball.y + bvy * lead).clamp(PR, FH - PR);
    move_to(&mut game.pl[player_idx], tx, ty, CSPEED * 1.18 * speed_mult);
}
