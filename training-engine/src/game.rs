use crate::constants::*;
use crate::policy::PolicyParams;

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
        }
    }
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
        }
    }
}

pub fn make_players() -> Vec<Player> {
    vec![
        Player::new(0, 0, FW * 0.44, H2, Role::Fwd),
        Player::new(1, 0, FW * 0.32, H2 - 85.0, Role::Mid),
        Player::new(2, 0, FW * 0.32, H2 + 85.0, Role::Mid),
        Player::new(3, 0, FW * 0.17, H2, Role::Def),
        Player::new(4, 0, FW * 0.04, H2, Role::Gk),
        Player::new(5, 1, FW * 0.56, H2, Role::Fwd),
        Player::new(6, 1, FW * 0.68, H2 - 85.0, Role::Mid),
        Player::new(7, 1, FW * 0.68, H2 + 85.0, Role::Mid),
        Player::new(8, 1, FW * 0.83, H2, Role::Def),
        Player::new(9, 1, FW * 0.96, H2, Role::Gk),
    ]
}
