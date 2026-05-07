use rand::Rng;

use crate::constants::*;
use crate::game::{Game, Phase, PlayerState};
use crate::team::Team;
use crate::physics::helpers::{is_in_own_penalty_area, is_jumping, knock_player, norm, slow_player};
use crate::physics::setpieces::{reset_kickoff, start_free_kick, start_penalty};
use crate::physics::ball::{do_shoot, update_ball};

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
        let tackler_id = game.pl[tackler_idx].id;
        let target_id = game.pl[target_idx].id;
        game.events.push(crate::game::MatchEvent::Tackle {
            tackler_id,
            tackler_team,
            target_id,
            target_team,
            x: gx,
            y: gy,
        });
    } else {
        if target_team != tackler_team && tackler_in_own_area {
            game.player_stats[tackler_idx].fouls += 1;
            game.player_stats[tackler_idx].penalties_caused += 1;
            let tackler_id = game.pl[tackler_idx].id;
            let target_id  = game.pl[target_idx].id;
            let (tx, ty)   = (game.pl[tackler_idx].x, game.pl[tackler_idx].y);
            game.events.push(crate::game::MatchEvent::Foul {
                tackler_id, tackler_team, target_id,
                x: tx, y: ty, is_penalty: true,
            });
            start_penalty(game, target_team);
            return true;
        }
        let fx = game.pl[target_idx].x;
        let fy = game.pl[target_idx].y;
        let fouled_id = game.pl[target_idx].id;
        let tackler_id = game.pl[tackler_idx].id;
        let target_id  = fouled_id;
        let (tx, ty)   = (game.pl[tackler_idx].x, game.pl[tackler_idx].y);
        slow_player(game, target_idx, SLOW_DUR * 4);
        start_free_kick(game, fouled_id, fx, fy);
        game.stats.tackle_success += 1;
        game.stats.fouls += 1;
        game.player_stats[tackler_idx].fouls += 1;
        game.events.push(crate::game::MatchEvent::Foul {
            tackler_id, tackler_team, target_id,
            x: tx, y: ty, is_penalty: false,
        });
    }
    true
}

pub fn step_game(game: &mut Game, teams: &mut [Box<dyn Team>; 2], rng: &mut impl Rng) {
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
                        game.player_stats[idx].penalties_taken += 1;
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

    for team in teams.iter_mut() { team.pre_tick(game); }

    for i in 0..game.pl.len() {
        if game.pl[i].tackle_cooldown > 0 { game.pl[i].tackle_cooldown -= 1; }
        if game.pl[i].jump_timer > 0 { game.pl[i].jump_timer -= 1; }
        if game.pl[i].slow_timer > 0 { game.pl[i].slow_timer -= 1; }
        if game.pl[i].gk_dive_timer < 0 {
            game.pl[i].gk_dive_timer += 1;
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
        let team_id = game.pl[i].team;
        teams[team_id].tick_player(game, i, rng as &mut dyn rand::RngCore);
    }

    update_ball(game);
}
