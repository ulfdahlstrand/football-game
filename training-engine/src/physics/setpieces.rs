use crate::constants::*;
use crate::game::{Game, PlayerState, Role};
use crate::physics::ball::set_ball_owner;

pub fn award_set_piece(game: &mut Game, taker_id: usize, sx: f32, sy: f32) {
    game.ball.x = sx; game.ball.y = sy;
    game.ball.vx = 0.0; game.ball.vy = 0.0;
    game.ball.owner = None; game.ball.mega = false;
    game.ball.cooldown = 0;
    game.set_piece_taker_id = Some(taker_id);
    game.set_piece_x = sx;
    game.set_piece_y = sy;
    game.set_piece_timer = 120;
    game.phase = crate::game::Phase::Playing;
}

pub fn start_free_kick(game: &mut Game, fouled_id: usize, fx: f32, fy: f32) {
    let team = game.pl.iter().find(|p| p.id == fouled_id).map(|p| p.team).unwrap_or(0);
    award_set_piece(game, fouled_id, fx, fy);
    game.free_kick_shooter_id = Some(fouled_id);
    game.free_kick_active = true;
    game.stats.free_kicks += 1;
    game.events.push(crate::game::MatchEvent::FreeKick { team, x: fx, y: fy });
}

fn goal_line_teams(x: f32) -> (usize, usize) {
    if x < FW / 2.0 { (1, 0) } else { (0, 1) }
}

fn find_role_player(game: &Game, team: usize, role: Role) -> Option<usize> {
    game.pl.iter()
        .find(|p| p.team == team && p.state == PlayerState::Active && p.role == role)
        .or_else(|| game.pl.iter().find(|p| p.team == team && p.state == PlayerState::Active))
        .map(|p| p.id)
}

fn restart_goal_kick(game: &mut Game, team: usize) {
    let keeper_id = find_role_player(game, team, Role::Gk);
    if let Some(kid) = keeper_id {
        let sx = if team == 0 { FIELD_LINE + PR * 2.3 } else { FW - FIELD_LINE - PR * 2.3 };
        game.gk_has_ball[team] = false;
        award_set_piece(game, kid, sx, H2);
    }
    game.stats.out_of_bounds += 1;
}

fn restart_kick_in(game: &mut Game, team: usize, bx: f32, by: f32) {
    let (tx, ty) = (bx.clamp(PR, FW - PR), if by < H2 { PR } else { FH - PR });
    let taker_id: Option<usize> = {
        game.pl.iter()
            .filter(|p| p.team == team && p.state == PlayerState::Active && p.role != Role::Gk)
            .min_by(|a, b| {
                let da = (a.x - tx).hypot(a.y - ty);
                let db = (b.x - tx).hypot(b.y - ty);
                da.partial_cmp(&db).unwrap()
            })
            .map(|p| p.id)
    };
    if let Some(tid) = taker_id {
        award_set_piece(game, tid, tx, ty);
    }
    game.stats.out_of_bounds += 1;
}

fn restart_corner(game: &mut Game, team: usize, bx: f32, by: f32) {
    let corner_x = if bx < FW / 2.0 { FIELD_LINE + PR } else { FW - FIELD_LINE - PR };
    let corner_y = if by < H2 { PR } else { FH - PR };
    let taker_id = find_role_player(game, team, Role::Mid)
        .or_else(|| find_role_player(game, team, Role::Fwd));
    if let Some(tid) = taker_id {
        award_set_piece(game, tid, corner_x, corner_y);
    }
    game.stats.out_of_bounds += 1;
    game.stats.corners += 1;
    game.events.push(crate::game::MatchEvent::Corner { team });
}

pub fn handle_ball_out(game: &mut Game) {
    let bx = game.ball.x;
    let by = game.ball.y;
    let last_touch = game.ball.last_touch_team;

    if by - BR <= 0.0 || by + BR >= FH {
        let restart_team = match last_touch { Some(0) => 1, _ => 0 };
        restart_kick_in(game, restart_team, bx, by);
        return;
    }
    if bx - BR <= FIELD_LINE || bx + BR >= FW - FIELD_LINE {
        let (attacking, defending) = goal_line_teams(bx);
        if last_touch == Some(attacking) {
            restart_goal_kick(game, defending);
        } else {
            restart_corner(game, attacking, bx, by);
        }
    }
}

pub fn start_penalty(game: &mut Game, team: usize) {
    let shooter_id = if team == 0 {
        Some(game.pl[0].id)
    } else {
        find_role_player(game, team, Role::Fwd)
            .or_else(|| find_role_player(game, team, Role::Mid))
    };
    let min_cooldown = FOUL_PAUSE + SET_PIECE_DELAY;
    for p in &mut game.pl {
        p.state = PlayerState::Active;
        p.knock_timer = 0;
        p.jump_timer = 0;
        if p.tackle_cooldown < min_cooldown { p.tackle_cooldown = min_cooldown; }
        if let Some(sid) = shooter_id {
            if p.id == sid { continue; }
        }
        let opp_gk = p.role == Role::Gk && p.team != team;
        if !opp_gk {
            if team == 0 && p.x > FW - FIELD_LINE - PENALTY_AREA_W - 20.0 {
                p.x = FW - FIELD_LINE - PENALTY_AREA_W - 20.0;
            } else if team == 1 && p.x < FIELD_LINE + PENALTY_AREA_W + 20.0 {
                p.x = FIELD_LINE + PENALTY_AREA_W + 20.0;
            }
        }
    }
    if let Some(sid) = shooter_id {
        let sidx = game.pl.iter().position(|p| p.id == sid).unwrap();
        let sx = if team == 0 { FW - FIELD_LINE - PENALTY_SPOT_D } else { FIELD_LINE + PENALTY_SPOT_D };
        set_ball_owner(game, sidx, sx, H2);
    }
    game.phase = crate::game::Phase::Penalty;
    game.penalty_team = Some(team);
    game.penalty_taken = false;
    game.stats.penalties += 1;
}

pub fn reset_kickoff(game: &mut Game) {
    let init_players = crate::game::make_players();
    for (p, s) in game.pl.iter_mut().zip(init_players.iter()) {
        p.x = s.x; p.y = s.y; p.vx = 0.0; p.vy = 0.0;
        p.state = PlayerState::Active; p.knock_timer = 0;
        p.tackle_cooldown = 0; p.jump_timer = 0;
        p.ai_jitter_x = 0.0; p.ai_jitter_y = 0.0; p.ai_jitter_timer = 0;
        p.slow_timer = 0; p.gk_dive_dir = None; p.gk_dive_timer = 0;
        p.gk_hold_timer = 0; p.gk_hold_extended = 0;
    }
    game.ball.x = FW / 2.0; game.ball.y = H2;
    game.ball.vx = 0.0; game.ball.vy = 0.0;
    game.ball.owner = None; game.ball.mega = false;
    game.ball.cooldown = 0; game.ball.last_touch_team = None;
    game.phase = crate::game::Phase::Kickoff;
    game.set_piece_timer = 0;
    game.penalty_team = None;
    game.penalty_taken = false;
    game.free_kick_active = false;
    game.free_kick_shooter_id = None;
    game.gk_has_ball = [false; 2];
    game.set_piece_taker_id = None;
}
