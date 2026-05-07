use rand::Rng;

use crate::constants::*;
use crate::game::{Game, Phase, Player, PlayerState, Role, make_players};

fn clamp(v: f32, lo: f32, hi: f32) -> f32 {
    v.max(lo).min(hi)
}

fn norm(dx: f32, dy: f32) -> (f32, f32) {
    let m = dx.hypot(dy);
    if m < 1e-9 { (0.0, 0.0) } else { (dx / m, dy / m) }
}

fn is_in_penalty_area_for_team(team: usize, x: f32, y: f32) -> bool {
    if (y - H2).abs() > GH / 2.0 + 38.0 { return false; }
    if team == 0 { x <= PENALTY_AREA_W } else { x >= FW - PENALTY_AREA_W }
}

fn is_in_own_penalty_area(p: &Player) -> bool {
    is_in_penalty_area_for_team(p.team, p.x, p.y)
}

fn is_jumping(p: &Player) -> bool {
    p.state == PlayerState::Active && p.jump_timer > 0
}

pub fn knock_player(game: &mut Game, idx: usize, duration: i32) {
    if game.pl[idx].state != PlayerState::Active { return; }
    game.pl[idx].state = PlayerState::Knocked;
    game.pl[idx].knock_timer = duration;
    game.pl[idx].jump_timer = 0;
    if game.ball.owner == Some(game.pl[idx].id) {
        game.ball.owner = None;
        game.ball.x = game.pl[idx].x;
        game.ball.y = game.pl[idx].y;
        game.ball.vx = 0.0;
        game.ball.vy = 0.0;
        game.ball.mega = false;
        game.ball.cooldown = BALL_COOL;
        game.stats.turnovers += 1;
    }
}

pub fn slow_player(game: &mut Game, idx: usize, dur: i32) {
    game.pl[idx].slow_timer = dur;
}

pub fn tackle_player(game: &mut Game, tackler_idx: usize, target_idx: usize) -> bool {
    if game.pl[tackler_idx].state != PlayerState::Active
        || game.pl[target_idx].state != PlayerState::Active {
        return false;
    }
    game.stats.tackles += 1;
    game.pl[tackler_idx].tackle_cooldown = TACKLE_COOL;

    let target_has_ball = game.ball.owner == Some(game.pl[target_idx].id);
    let tackler_in_own_area = is_in_own_penalty_area(&game.pl[tackler_idx]);
    let target_team = game.pl[target_idx].team;
    let tackler_team = game.pl[tackler_idx].team;

    if is_jumping(&game.pl[target_idx]) {
        knock_player(game, tackler_idx, TACKLE_MISS_DUR);
        return true;
    }

    if target_has_ball {
        // On-ball tackle: strip ball, nudge it forward in tackler's direction
        let (tx, ty) = (game.pl[tackler_idx].x, game.pl[tackler_idx].y);
        let (gx, gy) = (game.pl[target_idx].x, game.pl[target_idx].y);
        let (nx, ny) = norm(gx - tx, gy - ty);
        game.ball.owner = None;
        game.ball.x = gx;
        game.ball.y = gy;
        game.ball.vx = nx * TACKLE_BALL_NUDGE_POW;
        game.ball.vy = ny * TACKLE_BALL_NUDGE_POW;
        game.ball.cooldown = BALL_COOL;
        slow_player(game, target_idx, SLOW_DUR);
        game.stats.tackle_success += 1;
    } else {
        // Off-ball tackle: foul. No pause, no knock — slow target briefly,
        // free kick at foul spot.
        if target_team != tackler_team && tackler_in_own_area {
            game.pl[tackler_idx].fouls += 1;
            game.pl[tackler_idx].penalties_caused += 1;
            start_penalty(game, target_team);
            return true;
        }
        let fx = game.pl[target_idx].x;
        let fy = game.pl[target_idx].y;
        let fouled_id = game.pl[target_idx].id;
        slow_player(game, target_idx, SLOW_DUR * 4);
        start_free_kick(game, fouled_id, fx, fy);
        game.stats.tackle_success += 1;
        game.stats.fouls += 1;
        game.pl[tackler_idx].fouls += 1;
    }
    true
}

pub fn do_shoot(game: &mut Game, shooter_idx: usize, mega: bool, tx: f32, ty: f32, pow: Option<f32>, is_pass: bool) {
    // Indirect free kick: taker cannot shoot directly
    if game.free_kick_active && !is_pass {
        if game.free_kick_shooter_id == Some(game.pl[shooter_idx].id) { return; }
    }
    let p = pow.unwrap_or(if mega { MEGA_POW } else { SHOOT_POW });
    let (nx, ny) = norm(tx - game.pl[shooter_idx].x, ty - game.pl[shooter_idx].y);
    game.ball.vx = nx * p;
    game.ball.vy = ny * p;
    game.ball.x = game.pl[shooter_idx].x;
    game.ball.y = game.pl[shooter_idx].y;
    game.ball.owner = None;
    game.ball.mega = mega;
    game.ball.cooldown = BALL_COOL;
    game.ball.last_touch_team = Some(game.pl[shooter_idx].team);
    let shooter_id = game.pl[shooter_idx].id;
    if is_pass {
        game.last_passer = Some(shooter_id);
        game.stats.passes += 1;
    } else {
        game.last_passer = None;
        game.last_shooter = Some(shooter_id);
        game.pl[shooter_idx].shots += 1;
        game.stats.shots += 1;
    }
}

fn attribute_goal(game: &mut Game) {
    let scorer_id = game.last_shooter;
    let assister_id = game.last_passer;
    let is_penalty = game.penalty_shot_pending;
    game.penalty_shot_pending = false;
    if let Some(sid) = scorer_id {
        if let Some(p) = game.pl.iter_mut().find(|p| p.id == sid) {
            p.goals += 1;
            if is_penalty { p.penalties_scored += 1; }
        }
    }
    if let (Some(aid), Some(sid)) = (assister_id, scorer_id) {
        if aid != sid {
            if let Some(p) = game.pl.iter_mut().find(|p| p.id == aid) { p.assists += 1; }
        }
    }
}

fn set_ball_owner(game: &mut Game, player_idx: usize, x: f32, y: f32) {
    let px = clamp(x, PR, FW - PR);
    let py = clamp(y, PR, FH - PR);
    let p_id = game.pl[player_idx].id;
    let p_team = game.pl[player_idx].team;
    game.pl[player_idx].x = px;
    game.pl[player_idx].y = py;
    game.pl[player_idx].state = PlayerState::Active;
    game.pl[player_idx].knock_timer = 0;
    game.pl[player_idx].jump_timer = 0;
    game.ball.x = px;
    game.ball.y = py;
    game.ball.vx = 0.0;
    game.ball.vy = 0.0;
    game.ball.owner = Some(p_id);
    game.ball.mega = false;
    game.ball.cooldown = BALL_COOL;
    game.ball.last_touch_team = Some(p_team);
    game.phase = Phase::Playing;
    game.set_piece_timer = 90;
    game.set_piece_taker_id = None;
}

// Place ball at set-piece spot, mark a specific taker. Match continues.
fn award_set_piece(game: &mut Game, taker_id: usize, sx: f32, sy: f32) {
    game.ball.x = sx; game.ball.y = sy;
    game.ball.vx = 0.0; game.ball.vy = 0.0;
    game.ball.owner = None; game.ball.mega = false;
    game.ball.cooldown = 0;
    game.set_piece_taker_id = Some(taker_id);
    game.set_piece_x = sx;
    game.set_piece_y = sy;
    game.set_piece_timer = 120;
    game.phase = Phase::Playing;
}

pub fn start_free_kick(game: &mut Game, fouled_id: usize, fx: f32, fy: f32) {
    award_set_piece(game, fouled_id, fx, fy);
    game.free_kick_shooter_id = Some(fouled_id);
    game.free_kick_active = true;
    game.stats.free_kicks += 1;
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
    let (tx, ty) = (clamp(bx, PR, FW - PR), if by < H2 { PR } else { FH - PR });
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
        // Clear non-shooter/non-opposing-GK from penalty area
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
    game.phase = Phase::Penalty;
    game.penalty_team = Some(team);
    game.penalty_taken = false;
    game.stats.penalties += 1;
}

fn handle_ball_out(game: &mut Game) {
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

pub fn update_ball(game: &mut Game) {
    if game.ball.cooldown > 0 { game.ball.cooldown -= 1; }

    if let Some(owner_id) = game.ball.owner {
        let owner_idx = game.pl.iter().position(|p| p.id == owner_id);
        match owner_idx {
            Some(idx) if game.pl[idx].state == PlayerState::Active => {
                let (px, py, pt) = (game.pl[idx].x, game.pl[idx].y, game.pl[idx].team);
                game.ball.x = px;
                game.ball.y = py;
                game.ball.last_touch_team = Some(pt);
            }
            _ => { game.ball.owner = None; }
        }
        return;
    }

    game.ball.x += game.ball.vx;
    game.ball.y += game.ball.vy;
    game.ball.vx *= BALL_FRIC;
    game.ball.vy *= BALL_FRIC;

    let in_goal_y = (game.ball.y - H2).abs() < GH / 2.0;

    if game.ball.x - BR <= FIELD_LINE {
        if in_goal_y {
            game.score[1] += 1;
            game.phase = Phase::Goal;
            game.goal_anim = 160;
            game.goal_team = Some(1);
            game.stats.goals += 1;
            attribute_goal(game);
            game.gk_has_ball[0] = false;
        } else {
            handle_ball_out(game);
        }
        return;
    }
    if game.ball.x + BR >= FW - FIELD_LINE {
        if in_goal_y {
            game.score[0] += 1;
            game.phase = Phase::Goal;
            game.goal_anim = 160;
            game.goal_team = Some(0);
            game.stats.goals += 1;
            attribute_goal(game);
            game.gk_has_ball[1] = false;
        } else {
            handle_ball_out(game);
        }
        return;
    }
    if game.ball.y - BR <= 0.0 || game.ball.y + BR >= FH {
        handle_ball_out(game);
        return;
    }

    if game.ball.mega {
        let spd2 = game.ball.vx * game.ball.vx + game.ball.vy * game.ball.vy;
        if spd2 < 20.0 {
            game.ball.mega = false;
        } else {
            let bx = game.ball.x;
            let by = game.ball.y;
            for i in 0..game.pl.len() {
                if game.pl[i].state == PlayerState::Active
                    && (game.pl[i].x - bx).hypot(game.pl[i].y - by) < MEGA_KR {
                    knock_player(game, i, KNOCK_DUR);
                }
            }
        }
    }

    if game.ball.cooldown <= 0 {
        let bx = game.ball.x;
        let by = game.ball.y;
        let mut near_id: Option<usize> = None;
        let mut near_d2 = CAPTURE_DIST2;
        for p in &game.pl {
            if p.state != PlayerState::Active { continue; }
            // During an active set piece, only the designated taker can pick up
            if let Some(tid) = game.set_piece_taker_id {
                if p.id != tid { continue; }
            }
            let dd = (p.x - bx) * (p.x - bx) + (p.y - by) * (p.y - by);
            if dd < near_d2 {
                near_d2 = dd;
                near_id = Some(p.id);
            }
        }
        if let Some(pid) = near_id {
            let pidx = game.pl.iter().position(|p| p.id == pid).unwrap();
            let p_team = game.pl[pidx].team;
            let p_role = game.pl[pidx].role;
            let p_x = game.pl[pidx].x;
            let p_y = game.pl[pidx].y;

            // GK using hands outside goal area = handball foul → free kick to opponents
            if p_role == Role::Gk {
                let in_goal_area = if p_team == 0 {
                    p_x <= FIELD_LINE + GOAL_AREA_W
                } else {
                    p_x >= FW - FIELD_LINE - GOAL_AREA_W
                };
                if !in_goal_area {
                    let opp_team = 1 - p_team;
                    let fk_taker = game.pl.iter()
                        .filter(|q| q.team == opp_team && q.state == PlayerState::Active && q.role != Role::Gk)
                        .min_by(|a,b| {
                            (a.x - p_x).hypot(a.y - p_y)
                                .partial_cmp(&(b.x - p_x).hypot(b.y - p_y)).unwrap()
                        })
                        .map(|q| q.id);
                    if let Some(tid) = fk_taker {
                        award_set_piece(game, tid, p_x, p_y);
                        game.free_kick_shooter_id = Some(tid);
                        game.free_kick_active = true;
                        game.stats.fouls += 1;
                        game.pl[pidx].fouls += 1;
                    }
                    return; // skip normal pickup
                }
            }

            let prev_team = game.ball.last_touch_team;
            game.ball.owner = Some(pid);
            if game.set_piece_taker_id == Some(pid) { game.set_piece_taker_id = None; }
            if prev_team == Some(p_team) {
                game.stats.pass_completed += 1;
            }
            game.ball.last_touch_team = Some(p_team);

            if game.free_kick_active && game.free_kick_shooter_id != Some(pid) {
                game.free_kick_active = false;
            }

            // GK in goal area gets hands privilege
            if p_role == Role::Gk {
                game.gk_has_ball[p_team] = true;
                game.pl[pidx].gk_hold_timer = GK_HOLD_DELAY;
                game.pl[pidx].gk_hold_extended = 0;
            }
        }
    }

    // Clear gkHasBall if GK no longer has ball
    for t in 0..2 {
        if game.gk_has_ball[t] {
            let gk_owns = game.pl.iter()
                .any(|p| p.role == Role::Gk && p.team == t && game.ball.owner == Some(p.id));
            if !gk_owns { game.gk_has_ball[t] = false; }
        }
    }
}

pub fn reset_kickoff(game: &mut Game) {
    let init_players = make_players();
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
    game.phase = Phase::Kickoff;
    game.set_piece_timer = 0;
    game.penalty_team = None;
    game.penalty_taken = false;
    game.free_kick_active = false;
    game.free_kick_shooter_id = None;
    game.gk_has_ball = [false; 2];
    game.set_piece_taker_id = None;
}

pub fn step_game(game: &mut Game, rng: &mut impl Rng) {
    if game.phase == Phase::Kickoff { game.phase = Phase::Playing; }

    if game.phase == Phase::Goal {
        game.goal_anim -= 1;
        if game.goal_anim <= 0 { reset_kickoff(game); }
        return;
    }

    if game.phase == Phase::Penalty {
        if let Some(owner_id) = game.ball.owner {
            let owner_idx = game.pl.iter().position(|p| p.id == owner_id);
            if let Some(idx) = owner_idx {
                if !game.penalty_taken {
                    game.set_piece_timer -= 1;
                    if game.set_piece_timer <= 35 {
                        let team = game.penalty_team.unwrap_or(1);
                        let tx = if team == 0 { FW - FIELD_LINE } else { FIELD_LINE };
                        let jitter = (rng.gen::<f32>() * 2.0 - 1.0) * 48.0;
                        game.pl[idx].penalties_taken += 1;
                        game.penalty_shot_pending = true;
                        do_shoot(game, idx, false, tx, H2 + jitter, Some(SHOOT_POW), false);
                        game.phase = Phase::Playing;
                        game.penalty_taken = true;
                    }
                }
            } else {
                game.phase = Phase::Playing;
            }
        } else {
            game.phase = Phase::Playing;
        }
        return;
    }

    if game.phase == Phase::Fulltime { return; }

    game.timer -= 1;
    if game.timer <= 0 {
        game.phase = Phase::Fulltime;
        return;
    }

    if game.set_piece_timer > 0 { game.set_piece_timer -= 1; }

    // Tick all CPU players
    for i in 0..game.pl.len() {
        if game.pl[i].tackle_cooldown > 0 { game.pl[i].tackle_cooldown -= 1; }
        if game.pl[i].jump_timer > 0 { game.pl[i].jump_timer -= 1; }
        if game.pl[i].slow_timer > 0 { game.pl[i].slow_timer -= 1; }
        if game.pl[i].gk_dive_timer < 0 {
            game.pl[i].gk_dive_timer += 1; // counting up from negative toward 0
            continue;
        }
        if game.pl[i].state != PlayerState::Active {
            game.pl[i].knock_timer -= 1;
            if game.pl[i].knock_timer <= 0 {
                game.pl[i].state = PlayerState::Active;
                game.pl[i].gk_dive_dir = None;
            }
            continue;
        }
        if game.human_player == Some(game.pl[i].id) { continue; }
        crate::brain::tick_player(game, i, rng);
    }

    update_ball(game);
}

