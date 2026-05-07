use crate::constants::{FW, FH};
use crate::game::{Game, PlayerState};

const PRESS_RADIUS: f32 = 120.0;
const SUPPORT_RADIUS: f32 = 150.0;
const SPACE_RADIUS: f32 = 80.0;

pub struct GlobalBehavior {
    pub opp_avg_x: f32,      // normaliserad 0–1, 0=nära vår mål, 1=nära deras
    pub opp_press_rate: f32, // andel av våra spelare som har en motståndare inom PRESS_RADIUS
    pub space_behind: f32,   // normaliserat utrymme bakom motståndarnas back-linje
}

pub struct PlayerContext {
    pub local_pressure: f32,   // antal motståndare inom PRESS_RADIUS (normaliserat 0–1 av max 5)
    pub space_ahead: f32,      // normaliserat avstånd till närmaste motståndare i anfallsriktning
    pub marking_load: f32,     // antal motståndare att bevaka (normaliserat)
    pub teammate_density: f32, // antal lagkamrater inom SUPPORT_RADIUS (normaliserat)
    pub ball_proximity: f32,   // normaliserat avstånd till boll (0=nära, 1=långt)
}

pub fn detect_global(game: &Game, my_team: usize) -> GlobalBehavior {
    let opp_team = 1 - my_team;

    let opp_players: Vec<_> = game.pl.iter()
        .filter(|p| p.team == opp_team && p.state == PlayerState::Active)
        .collect();

    if opp_players.is_empty() {
        return GlobalBehavior { opp_avg_x: 0.5, opp_press_rate: 0.0, space_behind: 0.5 };
    }

    // Motståndares genomsnittliga x, normaliserat från mitt-lagets perspektiv
    let raw_avg_x = opp_players.iter().map(|p| p.x).sum::<f32>() / opp_players.len() as f32;
    let opp_avg_x = if my_team == 0 { raw_avg_x / FW } else { 1.0 - raw_avg_x / FW };

    // Press rate: hur många av VÅRA spelare har en motståndare nära sig
    let my_players: Vec<_> = game.pl.iter()
        .filter(|p| p.team == my_team && p.state == PlayerState::Active)
        .collect();
    let pressed = my_players.iter().filter(|p| {
        opp_players.iter().any(|o| {
            let dx = p.x - o.x;
            let dy = p.y - o.y;
            (dx * dx + dy * dy).sqrt() < PRESS_RADIUS
        })
    }).count();
    let opp_press_rate = if my_players.is_empty() {
        0.0
    } else {
        pressed as f32 / my_players.len() as f32
    };

    // Utrymme bakom motståndarnas back-linje
    let opp_back_x = if my_team == 0 {
        opp_players.iter().map(|p| p.x).fold(f32::INFINITY, f32::min)
    } else {
        opp_players.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max)
    };
    let space_behind = if my_team == 0 {
        (opp_back_x / FW).clamp(0.0, 1.0)
    } else {
        (1.0 - opp_back_x / FW).clamp(0.0, 1.0)
    };

    GlobalBehavior { opp_avg_x, opp_press_rate, space_behind }
}

pub fn detect_local(game: &Game, player_idx: usize) -> PlayerContext {
    let p = &game.pl[player_idx];
    let my_team = p.team;
    let opp_team = 1 - my_team;

    // Anfallsriktning: team 0 angriper mot höger (positiv x), team 1 mot vänster
    let attack_dx: f32 = if my_team == 0 { 1.0 } else { -1.0 };

    let opp_players: Vec<_> = game.pl.iter()
        .filter(|o| o.team == opp_team && o.state == PlayerState::Active)
        .collect();

    // Lokalt tryck: motståndare inom PRESS_RADIUS
    let nearby_opps = opp_players.iter().filter(|o| {
        let dx = p.x - o.x;
        let dy = p.y - o.y;
        (dx * dx + dy * dy).sqrt() < PRESS_RADIUS
    }).count();
    let local_pressure = (nearby_opps as f32 / 5.0_f32).min(1.0);

    // Utrymme framåt: avstånd till närmaste motståndare i anfallsriktning
    let min_ahead_dist = opp_players.iter()
        .filter(|o| (o.x - p.x) * attack_dx > 0.0) // framför spelaren
        .map(|o| ((o.x - p.x).powi(2) + (o.y - p.y).powi(2)).sqrt())
        .fold(f32::INFINITY, f32::min);
    let space_ahead = if min_ahead_dist == f32::INFINITY {
        1.0
    } else {
        (min_ahead_dist / FW).min(1.0)
    };

    // Marking load: motståndare att bevaka (nära i y-led oavsett x)
    let marking_load = (opp_players.iter().filter(|o| {
        (o.y - p.y).abs() < SPACE_RADIUS
    }).count() as f32 / 5.0_f32).min(1.0);

    // Lagkamrater inom stöd-radie
    let teammate_density = (game.pl.iter()
        .filter(|t| t.team == my_team && t.id != p.id && t.state == PlayerState::Active)
        .filter(|t| {
            let dx = t.x - p.x;
            let dy = t.y - p.y;
            (dx * dx + dy * dy).sqrt() < SUPPORT_RADIUS
        })
        .count() as f32 / 4.0_f32).min(1.0);

    // Bollnärhet
    let ball_dx = game.ball.x - p.x;
    let ball_dy = game.ball.y - p.y;
    let ball_dist = (ball_dx * ball_dx + ball_dy * ball_dy).sqrt();
    let ball_proximity = 1.0 - (ball_dist / (FW * 0.5)).min(1.0);

    PlayerContext {
        local_pressure,
        space_ahead,
        marking_load,
        teammate_density,
        ball_proximity,
    }
}

/// Normaliserar ett FH-värde till [0,1]
#[allow(dead_code)]
pub fn norm_y(y: f32) -> f32 { (y / FH).clamp(0.0, 1.0) }
