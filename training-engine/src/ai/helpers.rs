use crate::constants::*;
use crate::game::{Game, Player, PlayerState};
use crate::math::norm;

pub fn move_to(p: &mut Player, tx: f32, ty: f32, speed: f32) {
    let (nx, ny) = norm(tx - p.x, ty - p.y);
    p.x = (p.x + nx * speed).clamp(PR, FW - PR);
    p.y = (p.y + ny * speed).clamp(PR, FH - PR);
}

pub fn team_dir(team: usize) -> f32 {
    if team == 0 { 1.0 } else { -1.0 }
}

pub fn attack_progress(team: usize, x: f32) -> f32 {
    if team == 0 { x / FW } else { 1.0 - x / FW }
}

pub fn opp_goal_point(team: usize) -> (f32, f32) {
    if team == 0 { (FW + GD, H2) } else { (-GD, H2) }
}

pub fn own_goal_point(team: usize) -> (f32, f32) {
    if team == 0 { (0.0, H2) } else { (FW, H2) }
}

pub fn side_of(home_y: f32) -> f32 {
    if home_y < H2 { -1.0 } else { 1.0 }
}

pub fn wing_y(home_y: f32) -> f32 {
    if side_of(home_y) < 0.0 { 58.0 } else { FH - 58.0 }
}

pub fn point_between(ax: f32, ay: f32, bx: f32, by: f32, t: f32) -> (f32, f32) {
    (ax + (bx - ax) * t, ay + (by - ay) * t)
}

pub fn dist_to_segment(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let vx = bx - ax;
    let vy = by - ay;
    let len2 = vx * vx + vy * vy;
    let len2 = if len2 < 1e-9 { 1.0 } else { len2 };
    let t = (((px - ax) * vx + (py - ay) * vy) / len2).clamp(0.0, 1.0);
    let sx = ax + vx * t;
    let sy = ay + vy * t;
    (px - sx).hypot(py - sy)
}

pub fn pass_line_open(game: &Game, fx: f32, fy: f32, tx: f32, ty: f32, team: usize) -> bool {
    !game.pl.iter().any(|q| {
        if q.team == team || q.state != PlayerState::Active { return false; }
        dist_to_segment(q.x, q.y, fx, fy, tx, ty) < PASS_BLOCK_DIST
    })
}

pub fn is_marked(game: &Game, p: &Player, threshold: f32) -> bool {
    game.pl.iter().any(|q| {
        q.team != p.team && q.state == PlayerState::Active
            && (q.x - p.x).hypot(q.y - p.y) < threshold
    })
}

pub fn nearest_opponent_distance(game: &Game, p: &Player) -> f32 {
    game.pl.iter()
        .filter(|q| q.team != p.team && q.state == PlayerState::Active)
        .map(|q| (q.x - p.x).hypot(q.y - p.y))
        .fold(f32::INFINITY, f32::min)
}
