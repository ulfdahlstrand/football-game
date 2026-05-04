use crate::constants::*;
use crate::brain::PlayerBrain;
use crate::policy::{PolicyParams, TeamPolicy, TeamPolicyV3, TeamPolicyV4};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Phase {
    Kickoff,
    Playing,
    Goal,
    Penalty,
    Fulltime,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Role {
    Fwd,
    Mid,
    Def,
    Gk,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PlayerState {
    Active,
    Knocked,
}

#[derive(Clone, Debug)]
pub struct Player {
    pub id: usize,
    pub team: usize,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub role: Role,
    pub state: PlayerState,
    pub knock_timer: i32,
    pub home_x: f32,
    pub home_y: f32,
    pub tackle_cooldown: i32,
    pub jump_timer: i32,
    pub ai_jitter_x: f32,
    pub ai_jitter_y: f32,
    pub ai_jitter_timer: i32,
    pub slow_timer: i32,
    pub gk_dive_dir: Option<bool>, // Some(true) = up (y<H2), Some(false) = down
    pub gk_dive_timer: i32,        // positive = diving, negative = on ground
    pub gk_hold_timer: i32,
    /// What algorithm + parameters this player uses to make decisions.
    /// Set at game setup; defaults to V1 with classic params.
    pub brain: PlayerBrain,
}

impl Player {
    pub fn new(id: usize, team: usize, x: f32, y: f32, role: Role) -> Self {
        Self {
            id,
            team,
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            role,
            state: PlayerState::Active,
            knock_timer: 0,
            home_x: x,
            home_y: y,
            tackle_cooldown: 0,
            jump_timer: 0,
            ai_jitter_x: 0.0,
            ai_jitter_y: 0.0,
            ai_jitter_timer: 0,
            slow_timer: 0,
            gk_dive_dir: None,
            gk_dive_timer: 0,
            gk_hold_timer: 0,
            brain: PlayerBrain::default(),
        }
    }
}

/// Returns the underlying classic PolicyParams for a player. Read from their
/// brain (V1/V2 directly, V3 returns its `.base`). Used by helper code that
/// only cares about classic params (e.g. `cpu_find_pass`).
pub fn effective_policy(game: &Game, player_idx: usize) -> PolicyParams {
    game.pl[player_idx].brain.base_params()
}

#[derive(Clone, Debug)]
pub struct Ball {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub owner: Option<usize>,
    pub mega: bool,
    pub cooldown: i32,
    pub last_touch_team: Option<usize>,
}

impl Ball {
    pub fn new() -> Self {
        Self {
            x: FW / 2.0,
            y: H2,
            vx: 0.0,
            vy: 0.0,
            owner: None,
            mega: false,
            cooldown: 0,
            last_touch_team: None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub passes: u32,
    pub pass_completed: u32,
    pub shots: u32,
    pub goals: u32,
    pub tackles: u32,
    pub tackle_success: u32,
    pub turnovers: u32,
    pub out_of_bounds: u32,
    pub fouls: u32,
    pub free_kicks: u32,
    pub corners: u32,
    pub penalties: u32,
}

#[derive(Clone, Debug)]
pub struct Game {
    pub pl: Vec<Player>,
    pub ball: Ball,
    pub score: [u32; 2],
    pub timer: i32,
    pub phase: Phase,
    pub goal_anim: i32,
    pub goal_team: Option<usize>,
    pub set_piece_timer: i32,
    pub penalty_team: Option<usize>,
    pub penalty_taken: bool,
    pub policies: [PolicyParams; 2],
    pub stats: Stats,
    // Free kick state (indirect rule)
    pub free_kick_active: bool,
    pub free_kick_shooter_id: Option<usize>,
    // GK hands state
    pub gk_has_ball: [bool; 2],
    // Set piece taker — only this player can pick up the ball during a set piece
    pub set_piece_taker_id: Option<usize>,
    pub set_piece_x: f32,
    pub set_piece_y: f32,
}

impl Game {
    pub fn new(policy0: PolicyParams, policy1: PolicyParams) -> Self {
        Self {
            pl: make_players(),
            ball: Ball::new(),
            score: [0, 0],
            timer: GAME_SECS * 60,
            phase: Phase::Kickoff,
            goal_anim: 0,
            goal_team: None,
            set_piece_timer: 0,
            penalty_team: None,
            penalty_taken: false,
            policies: [policy0, policy1],
            stats: Stats::default(),
            free_kick_active: false,
            free_kick_shooter_id: None,
            gk_has_ball: [false; 2],
            set_piece_taker_id: None,
            set_piece_x: 0.0,
            set_piece_y: 0.0,
        }
    }

    /// Sets up a v2 team-vs-team match: per-position classic params, V2 brain.
    pub fn for_team_battle(team0: &TeamPolicy, team1: &TeamPolicy) -> Self {
        let mut game = Self::new(team0[0], team1[0]);
        for player in &mut game.pl {
            let slot = player.id % 5;
            let p = if player.team == 0 { team0[slot] } else { team1[slot] };
            player.brain = PlayerBrain::V2(p);
        }
        game
    }

    /// Sets up a v1 team-vs-team match: shared team-level params, V1 brain.
    pub fn for_team_battle_v1(team0: PolicyParams, team1: PolicyParams) -> Self {
        let mut game = Self::new(team0, team1);
        for player in &mut game.pl {
            let p = if player.team == 0 { team0 } else { team1 };
            player.brain = PlayerBrain::V1(p);
        }
        game
    }

    /// Sets up a v3 team-vs-team match: per-position V3Params, V3 brain.
    pub fn for_team_battle_v3(team0: &TeamPolicyV3, team1: &TeamPolicyV3) -> Self {
        let mut game = Self::new(team0[0].base, team1[0].base);
        for player in &mut game.pl {
            let slot = player.id % 5;
            let p = if player.team == 0 { team0[slot] } else { team1[slot] };
            player.brain = PlayerBrain::V3(p);
        }
        game
    }

    /// Sets up a v4 team-vs-team match: per-position V4Params, V4 brain.
    pub fn for_team_battle_v4(team0: &TeamPolicyV4, team1: &TeamPolicyV4) -> Self {
        let mut game = Self::new(team0[0].v3.base, team1[0].v3.base);
        for player in &mut game.pl {
            let slot = player.id % 5;
            let p = if player.team == 0 { team0[slot] } else { team1[slot] };
            player.brain = PlayerBrain::V4(p);
        }
        game
    }

    /// Mixed-version match: caller provides 10 brains directly (one per
    /// player id). Lets you stage v1-vs-v2, mid-tier v3 vs full-v2, etc.
    pub fn for_mixed_battle(brains: [PlayerBrain; 10]) -> Self {
        let p0 = brains[0].base_params();
        let p1 = brains[5].base_params();
        let mut game = Self::new(p0, p1);
        for (player, brain) in game.pl.iter_mut().zip(brains.iter()) {
            player.brain = *brain;
        }
        game
    }
}

pub fn make_players() -> Vec<Player> {
    vec![
        Player::new(0, 0, FW * 0.44, H2, Role::Fwd),
        Player::new(1, 0, FW * 0.32, H2 - 85.0, Role::Mid),
        Player::new(2, 0, FW * 0.32, H2 + 85.0, Role::Mid),
        Player::new(3, 0, FW * 0.17, H2, Role::Def),
        Player::new(4, 0, FIELD_LINE + PR * 2.0, H2, Role::Gk),
        Player::new(5, 1, FW * 0.56, H2, Role::Fwd),
        Player::new(6, 1, FW * 0.68, H2 - 85.0, Role::Mid),
        Player::new(7, 1, FW * 0.68, H2 + 85.0, Role::Mid),
        Player::new(8, 1, FW * 0.83, H2, Role::Def),
        Player::new(9, 1, FW - FIELD_LINE - PR * 2.0, H2, Role::Gk),
    ]
}
