use crate::constants::*;
use crate::game::{Game, PlayerState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldZone {
    OwnPenaltyArea,
    OwnHalf,
    Midfield,
    OppHalf,
    OppPenaltyArea,
}

impl FieldZone {
    pub fn index(&self) -> usize {
        match self {
            FieldZone::OwnPenaltyArea => 0,
            FieldZone::OwnHalf => 1,
            FieldZone::Midfield => 2,
            FieldZone::OppHalf => 3,
            FieldZone::OppPenaltyArea => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corridor {
    Top,    // y in [0, FH/3)
    Middle, // y in [FH/3, 2*FH/3]
    Bottom, // y in (2*FH/3, FH]
}

impl Corridor {
    pub fn index(&self) -> i32 {
        match self {
            Corridor::Top => -1,
            Corridor::Middle => 0,
            Corridor::Bottom => 1,
        }
    }
}

/// Computed at the start of a v3 player's tick. All v3 params can reason
/// about these. v1/v2 ignore the whole module.
#[derive(Clone, Copy, Debug)]
pub struct SpatialFeatures {
    // Edge distances (positive, in pixels)
    pub dist_left_edge: f32,
    pub dist_right_edge: f32,
    pub dist_top_edge: f32,
    pub dist_bottom_edge: f32,
    pub dist_nearest_edge: f32,

    // Goal distances (own goal = where this player defends)
    pub dist_own_goal: f32,
    pub dist_opp_goal: f32,
    pub angle_to_opp_goal: f32,    // radians
    pub vec_to_opp_goal: (f32, f32),

    // Player relations
    pub nearest_teammate_id: Option<usize>,
    pub nearest_teammate_dist: f32,
    pub nearest_opp_id: Option<usize>,
    pub nearest_opp_dist: f32,
    /// Avg distance to all active teammates (excluding self).
    pub avg_teammate_dist: f32,
    /// Number of opponents within `pressure_radius`.
    pub opp_within_pressure: usize,

    // Zone classification
    pub zone: FieldZone,
    pub corridor: Corridor,

    /// True iff a straight line from player to opp_goal is blocked by an
    /// opponent within `block_dist` of the line.
    pub direct_shot_blocked: bool,
    /// Number of opponents intercepting the line to the ball owner (if any).
    pub lane_to_ball_blockers: usize,
}

const PRESSURE_RADIUS_DEFAULT: f32 = 72.0;
const BLOCK_DIST_DEFAULT: f32 = 25.0;

/// Compute features for the player at `idx`. Pure function over `game`.
pub fn compute_features(game: &Game, idx: usize) -> SpatialFeatures {
    compute_features_with(game, idx, PRESSURE_RADIUS_DEFAULT, BLOCK_DIST_DEFAULT)
}

pub fn compute_features_with(game: &Game, idx: usize, pressure_radius: f32, block_dist: f32) -> SpatialFeatures {
    let p = &game.pl[idx];
    let (px, py) = (p.x, p.y);

    // Edges
    let dist_left = px;
    let dist_right = FW - px;
    let dist_top = py;
    let dist_bottom = FH - py;
    let dist_nearest_edge = dist_left.min(dist_right).min(dist_top).min(dist_bottom);

    // Goals
    let (own_gx, own_gy) = if p.team == 0 { (0.0, H2) } else { (FW, H2) };
    let (opp_gx, opp_gy) = if p.team == 0 { (FW, H2) } else { (0.0, H2) };
    let dist_own_goal = ((px - own_gx).powi(2) + (py - own_gy).powi(2)).sqrt();
    let dist_opp_goal = ((px - opp_gx).powi(2) + (py - opp_gy).powi(2)).sqrt();
    let dx = opp_gx - px;
    let dy = opp_gy - py;
    let angle_to_opp_goal = dy.atan2(dx);
    let mag = (dx * dx + dy * dy).sqrt().max(1e-6);
    let vec_to_opp_goal = (dx / mag, dy / mag);

    // Player relations
    let mut nearest_teammate_id: Option<usize> = None;
    let mut nearest_teammate_dist = f32::INFINITY;
    let mut nearest_opp_id: Option<usize> = None;
    let mut nearest_opp_dist = f32::INFINITY;
    let mut sum_teammate_dist = 0.0_f32;
    let mut count_teammate = 0usize;
    let mut opp_within_pressure = 0usize;

    for q in &game.pl {
        if q.id == p.id || q.state != PlayerState::Active { continue; }
        let d = ((q.x - px).powi(2) + (q.y - py).powi(2)).sqrt();
        if q.team == p.team {
            sum_teammate_dist += d;
            count_teammate += 1;
            if d < nearest_teammate_dist { nearest_teammate_dist = d; nearest_teammate_id = Some(q.id); }
        } else {
            if d < nearest_opp_dist { nearest_opp_dist = d; nearest_opp_id = Some(q.id); }
            if d < pressure_radius { opp_within_pressure += 1; }
        }
    }
    let avg_teammate_dist = if count_teammate > 0 { sum_teammate_dist / count_teammate as f32 } else { 0.0 };

    // Zone
    let in_penalty_y = (py - H2).abs() <= GH / 2.0 + 38.0;
    let zone = if p.team == 0 {
        if in_penalty_y && px <= PENALTY_AREA_W { FieldZone::OwnPenaltyArea }
        else if in_penalty_y && px >= FW - PENALTY_AREA_W { FieldZone::OppPenaltyArea }
        else if px < FW * 0.33 { FieldZone::OwnHalf }
        else if px > FW * 0.67 { FieldZone::OppHalf }
        else { FieldZone::Midfield }
    } else {
        if in_penalty_y && px >= FW - PENALTY_AREA_W { FieldZone::OwnPenaltyArea }
        else if in_penalty_y && px <= PENALTY_AREA_W { FieldZone::OppPenaltyArea }
        else if px > FW * 0.67 { FieldZone::OwnHalf }
        else if px < FW * 0.33 { FieldZone::OppHalf }
        else { FieldZone::Midfield }
    };

    // Corridor (vertical thirds)
    let corridor = if py < FH / 3.0 { Corridor::Top }
                   else if py > FH * 2.0 / 3.0 { Corridor::Bottom }
                   else { Corridor::Middle };

    // Direct shot blocked?
    let direct_shot_blocked = is_segment_blocked_by_opponent(
        game, p.team, px, py, opp_gx, opp_gy, block_dist
    );

    // Lane to ball owner
    let lane_to_ball_blockers = if let Some(owner_id) = game.ball.owner {
        if let Some(owner) = game.pl.iter().find(|q| q.id == owner_id) {
            count_blockers_in_lane(game, p.team, px, py, owner.x, owner.y, block_dist)
        } else { 0 }
    } else { 0 };

    SpatialFeatures {
        dist_left_edge: dist_left,
        dist_right_edge: dist_right,
        dist_top_edge: dist_top,
        dist_bottom_edge: dist_bottom,
        dist_nearest_edge,
        dist_own_goal,
        dist_opp_goal,
        angle_to_opp_goal,
        vec_to_opp_goal,
        nearest_teammate_id,
        nearest_teammate_dist: if nearest_teammate_dist.is_finite() { nearest_teammate_dist } else { 0.0 },
        nearest_opp_id,
        nearest_opp_dist: if nearest_opp_dist.is_finite() { nearest_opp_dist } else { 0.0 },
        avg_teammate_dist,
        opp_within_pressure,
        zone,
        corridor,
        direct_shot_blocked,
        lane_to_ball_blockers,
    }
}

/// Distance from a point to a line segment.
pub fn dist_to_segment(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let vx = bx - ax;
    let vy = by - ay;
    let len2 = (vx * vx + vy * vy).max(1e-6);
    let t = (((px - ax) * vx + (py - ay) * vy) / len2).clamp(0.0, 1.0);
    let sx = ax + vx * t;
    let sy = ay + vy * t;
    ((px - sx).powi(2) + (py - sy).powi(2)).sqrt()
}

pub fn is_segment_blocked_by_opponent(game: &Game, team: usize, ax: f32, ay: f32, bx: f32, by: f32, block_dist: f32) -> bool {
    game.pl.iter().any(|q| {
        if q.team == team || q.state != PlayerState::Active { return false; }
        dist_to_segment(q.x, q.y, ax, ay, bx, by) < block_dist
    })
}

pub fn count_blockers_in_lane(game: &Game, team: usize, ax: f32, ay: f32, bx: f32, by: f32, block_dist: f32) -> usize {
    game.pl.iter().filter(|q| {
        q.team != team && q.state == PlayerState::Active
            && dist_to_segment(q.x, q.y, ax, ay, bx, by) < block_dist
    }).count()
}
