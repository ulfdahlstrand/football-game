use rand::Rng;

use crate::ai::helpers::{move_to, pass_line_open};
use crate::constants::*;
use crate::game::{Game, PlayerState};

/// Gemensamma GK-parametrar — implementeras både av `TickHooks` (classic-tick)
/// och `GkDecisionParams` (v6-tick) så att GK-logiken kan delas.
pub trait GkParams {
    fn dive_chance(&self) -> f32;
    fn dive_commit_dist(&self) -> f32;
    fn risk_clearance(&self) -> f32;
    fn distribution_zone(&self) -> f32;
    fn pass_target_dist(&self) -> f32;
}

impl GkParams for crate::brain::TickHooks {
    fn dive_chance(&self) -> f32 { self.gk_dive_chance }
    fn dive_commit_dist(&self) -> f32 { self.gk_dive_commit_dist }
    fn risk_clearance(&self) -> f32 { self.gk_risk_clearance }
    fn distribution_zone(&self) -> f32 { self.gk_distribution_zone }
    fn pass_target_dist(&self) -> f32 { self.gk_pass_target_dist }
}

impl GkParams for crate::policy::v6::GkDecisionParams {
    fn dive_chance(&self) -> f32 { self.gk_dive_chance }
    fn dive_commit_dist(&self) -> f32 { self.gk_dive_commit_dist }
    fn risk_clearance(&self) -> f32 { self.gk_risk_clearance }
    fn distribution_zone(&self) -> f32 { self.gk_distribution_zone }
    fn pass_target_dist(&self) -> f32 { self.gk_pass_target_dist }
}

/// X-koordinat för GK:s baslinje (med PR-offset).
pub fn gk_line_x(team: usize) -> f32 {
    if team == 0 { FIELD_LINE + PR * 1.5 } else { FW - FIELD_LINE - PR * 1.5 }
}

/// X-koordinat för själva mållinjen (utan PR-offset).
pub fn gk_goal_x(team: usize) -> f32 {
    if team == 0 { FIELD_LINE } else { FW - FIELD_LINE }
}

/// GK med boll: håll kort, distribuera sedan.
/// Returnerar true om någon åtgärd vidtogs (anropare ska då returnera).
pub fn distribute_ball<P: GkParams>(
    game: &mut Game,
    player_idx: usize,
    params: &P,
    rng: &mut (impl Rng + ?Sized),
) -> bool {
    let p_id = game.pl[player_idx].id;
    let p_team = game.pl[player_idx].team;

    if game.pl[player_idx].gk_hold_timer > 0 {
        game.pl[player_idx].gk_hold_timer -= 1;
        return true;
    }
    let opponents_on_own_half = game.pl.iter().any(|q| {
        q.team != p_team && q.state == PlayerState::Active
            && if p_team == 0 { q.x < FW / 2.0 } else { q.x > FW / 2.0 }
    });
    let extended = game.pl[player_idx].gk_hold_extended;
    if opponents_on_own_half && extended < GK_MAX_HOLD_EXTRA
        && rng.gen::<f32>() > params.risk_clearance()
    {
        game.pl[player_idx].gk_hold_timer = 5;
        game.pl[player_idx].gk_hold_extended += 5;
        return true;
    }
    game.pl[player_idx].gk_hold_extended = 0;
    game.gk_has_ball[p_team] = false;
    let gk_x = game.pl[player_idx].x;
    let gk_y = game.pl[player_idx].y;
    let pass_target = game.pl.iter()
        .filter(|q| q.team == p_team && q.id != p_id && q.state == PlayerState::Active)
        .filter(|q| (q.x - gk_x).hypot(q.y - gk_y) <= params.pass_target_dist())
        .filter(|q| pass_line_open(game, gk_x, gk_y, q.x, q.y, p_team))
        .min_by(|a, b| {
            let da = (a.x - gk_x).hypot(a.y - gk_y);
            let db = (b.x - gk_x).hypot(b.y - gk_y);
            da.partial_cmp(&db).unwrap()
        })
        .map(|q| (q.x, q.y));
    if let Some((tx, ty)) = pass_target {
        crate::physics::do_shoot(game, player_idx, false, tx, ty, None, false);
    } else {
        let target_y = if params.distribution_zone() > 0.5 {
            if gk_y < H2 { PR * 2.0 } else { FH - PR * 2.0 }
        } else { H2 };
        crate::physics::do_shoot(game, player_idx, false, FW / 2.0, target_y, None, false);
    }
    true
}

/// Försöker initiera en dykning mot inkommande skott. No-op om villkoren inte uppfylls.
pub fn maybe_start_dive<P: GkParams>(
    game: &mut Game,
    player_idx: usize,
    params: &P,
    rng: &mut (impl Rng + ?Sized),
) {
    if game.pl[player_idx].gk_dive_timer != 0 || game.ball.owner.is_some() { return; }
    let p_team = game.pl[player_idx].team;
    let goal_x = gk_goal_x(p_team);
    let is_incoming = if p_team == 0 { game.ball.vx < -8.0 } else { game.ball.vx > 8.0 };
    let dist_to_goal = (game.pl[player_idx].x - goal_x).abs();
    if is_incoming && dist_to_goal < params.dive_commit_dist()
        && rng.gen::<f32>() < params.dive_chance()
    {
        let frames_until_goal = if game.ball.vx.abs() > 0.1 {
            (goal_x - game.ball.x) / game.ball.vx
        } else { 0.0 };
        let predicted_y = game.ball.y + game.ball.vy * frames_until_goal.max(0.0);
        let jitter = GK_DIVE_JITTER * (1.0 - dist_to_goal / params.dive_commit_dist());
        let effective_y = predicted_y + (rng.gen::<f32>() * 2.0 - 1.0) * jitter;
        game.pl[player_idx].gk_dive_dir = Some(effective_y < H2);
        game.pl[player_idx].gk_dive_timer = GK_DIVE_DUR;
    }
}

/// Fortsätter en pågående dykning. Returnerar true om dive aktivt utförs
/// (anropare ska då returnera).
pub fn continue_dive(game: &mut Game, player_idx: usize) -> bool {
    if game.pl[player_idx].gk_dive_timer <= 0 { return false; }
    let dive_up = game.pl[player_idx].gk_dive_dir.unwrap_or(true);
    let dive_y = if dive_up { H2 - GH / 2.0 + PR } else { H2 + GH / 2.0 - PR };
    let cur_x = game.pl[player_idx].x;
    move_to(&mut game.pl[player_idx], cur_x, dive_y, CSPEED * 3.5);
    game.pl[player_idx].gk_dive_timer -= 1;
    if game.pl[player_idx].gk_dive_timer <= 0 {
        let caught = game.ball.owner.is_none()
            && (game.pl[player_idx].x - game.ball.x)
                .hypot(game.pl[player_idx].y - game.ball.y) < PR + BR + 8.0;
        if !caught { game.pl[player_idx].gk_dive_timer = -GK_DIVE_DUR; }
    }
    true
}
