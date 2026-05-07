use crate::constants::*;
use crate::game::{Game, Player, PlayerState};

pub use crate::math::norm;

pub fn is_in_penalty_area_for_team(team: usize, x: f32, y: f32) -> bool {
    if (y - H2).abs() > GH / 2.0 + 38.0 { return false; }
    if team == 0 { x <= PENALTY_AREA_W } else { x >= FW - PENALTY_AREA_W }
}

pub fn is_in_own_penalty_area(p: &Player) -> bool {
    is_in_penalty_area_for_team(p.team, p.x, p.y)
}

pub fn is_jumping(p: &Player) -> bool {
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
