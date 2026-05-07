use crate::constants::*;
use crate::game::{Game, Phase, PlayerState, Role};
use crate::math::norm;
use crate::physics::helpers::knock_player;

pub fn set_ball_owner(game: &mut Game, player_idx: usize, x: f32, y: f32) {
    let px = x.clamp(PR, FW - PR);
    let py = y.clamp(PR, FH - PR);
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

pub fn do_shoot(game: &mut Game, shooter_idx: usize, mega: bool, tx: f32, ty: f32, pow: Option<f32>, is_pass: bool) {
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
    let shooter_team = game.pl[shooter_idx].team;
    if is_pass {
        game.last_passer = Some(shooter_id);
        game.stats.passes += 1;
        game.events.push(crate::game::MatchEvent::Pass {
            team: shooter_team,
            player_id: shooter_id,
        });
    } else {
        game.last_passer = None;
        game.last_shooter = Some(shooter_id);
        game.player_stats[shooter_idx].shots += 1;
        game.stats.shots += 1;
        game.events.push(crate::game::MatchEvent::Shot {
            team: shooter_team,
            player_id: shooter_id,
            mega,
        });
    }
}

fn attribute_goal(game: &mut Game) {
    let scorer_id = game.last_shooter;
    let assister_id = game.last_passer;
    let is_penalty = game.penalty_shot_pending;
    game.penalty_shot_pending = false;
    if let Some(sid) = scorer_id {
        if let Some(idx) = game.pl.iter().position(|p| p.id == sid) {
            game.player_stats[idx].goals += 1;
            if is_penalty { game.player_stats[idx].penalties_scored += 1; }
        }
    }
    let mut emit_assist: Option<usize> = None;
    if let (Some(aid), Some(sid)) = (assister_id, scorer_id) {
        if aid != sid {
            if let Some(idx) = game.pl.iter().position(|p| p.id == aid) {
                game.player_stats[idx].assists += 1;
                emit_assist = Some(aid);
            }
        }
    }
    if let Some(team) = game.goal_team {
        game.events.push(crate::game::MatchEvent::Goal {
            team,
            scorer_id,
            assister_id: emit_assist,
            is_penalty,
        });
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
            crate::physics::setpieces::handle_ball_out(game);
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
            crate::physics::setpieces::handle_ball_out(game);
        }
        return;
    }
    if game.ball.y - BR <= 0.0 || game.ball.y + BR >= FH {
        crate::physics::setpieces::handle_ball_out(game);
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
                        .min_by(|a, b| {
                            (a.x - p_x).hypot(a.y - p_y)
                                .partial_cmp(&(b.x - p_x).hypot(b.y - p_y)).unwrap()
                        })
                        .map(|q| q.id);
                    if let Some(tid) = fk_taker {
                        crate::physics::setpieces::award_set_piece(game, tid, p_x, p_y);
                        game.free_kick_shooter_id = Some(tid);
                        game.free_kick_active = true;
                        game.stats.fouls += 1;
                        game.player_stats[pidx].fouls += 1;
                    }
                    return;
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

            if p_role == Role::Gk {
                game.gk_has_ball[p_team] = true;
                game.pl[pidx].gk_hold_timer = GK_HOLD_DELAY;
                game.pl[pidx].gk_hold_extended = 0;
                if let Some(shooter_id) = game.last_shooter {
                    let shooter_team = game.pl.iter().find(|p| p.id == shooter_id).map(|p| p.team);
                    if shooter_team == Some(1 - p_team) {
                        game.events.push(crate::game::MatchEvent::Save {
                            gk_team: p_team,
                            gk_id: pid,
                            shooter_id: Some(shooter_id),
                        });
                        game.last_shooter = None;
                    }
                }
            }
        }
    }

    for t in 0..2 {
        if game.gk_has_ball[t] {
            let gk_owns = game.pl.iter()
                .any(|p| p.role == Role::Gk && p.team == t && game.ball.owner == Some(p.id));
            if !gk_owns { game.gk_has_ball[t] = false; }
        }
    }
}
