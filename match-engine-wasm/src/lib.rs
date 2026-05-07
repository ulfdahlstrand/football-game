use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

use training_engine::constants::*;
use training_engine::game::{Game, Phase, Role, PlayerState};
use training_engine::physics::{step_game as rust_step_game, do_shoot, tackle_player};
use training_engine::ai::cpu_find_pass;
use training_engine::policy::TeamPolicyV6;

// ── Human input from JS ───────────────────────────────────────────────────────

#[derive(Deserialize, Default)]
struct HumanInput {
    #[serde(default)]
    dx: f32,
    #[serde(default)]
    dy: f32,
    #[serde(default)]
    shoot: bool,
    #[serde(default)]
    mega_shoot: bool,
    #[serde(default)]
    pass_action: bool,
    #[serde(default)]
    pass_dir: Option<[f32; 2]>,
    #[serde(default)]
    tackle: bool,
    #[serde(default)]
    jump: bool,
    #[serde(default)]
    celebrate: bool,
}

// ── Rendering state per player (not in core Game) ────────────────────────────

#[derive(Clone)]
struct PlayerRender {
    facing: &'static str,
    step_counter: i32,
    celebrate_timer: i32,
}

const HAIR_COLORS: [&str; 10] = [
    "#5a3a1a", "#1a1a1a", "#f4d090", "#8b0000", "#ffd700",
    "#5a3a1a", "#1a1a1a", "#f4d090", "#5a3a1a", "#c87850",
];

impl PlayerRender {
    fn new() -> Self {
        Self { facing: "down", step_counter: 0, celebrate_timer: 0 }
    }
}

// ── Full game session ─────────────────────────────────────────────────────────

struct GameSession {
    game: Game,
    render: Vec<PlayerRender>,
    rng: rand::rngs::SmallRng,
    set_piece_text: Option<&'static str>,
    celebration: bool,
    celebrate_frame: i32,
    done: bool,
    prev_pos: Vec<(f32, f32)>,
    // Event flags reset each frame, read by JS for SFX
    ev_goal_scored: bool,
    ev_shot_taken: bool,
    ev_pass_done: bool,
    ev_tackle_done: bool,
}

impl GameSession {
    fn new(team0: &TeamPolicyV6, team1: &TeamPolicyV6, seed: u32) -> Self {
        use rand::SeedableRng;
        let mut game = Game::for_team_battle_v6(team0, team1);
        game.human_player = Some(0);
        let n = game.pl.len();
        let prev_pos = game.pl.iter().map(|p| (p.x, p.y)).collect();
        Self {
            game,
            render: (0..n).map(|_| PlayerRender::new()).collect(),
            rng: rand::rngs::SmallRng::seed_from_u64(seed as u64),
            set_piece_text: Some("AVSPARK"),
            celebration: false,
            celebrate_frame: 0,
            done: false,
            prev_pos,
            ev_goal_scored: false,
            ev_shot_taken: false,
            ev_pass_done: false,
            ev_tackle_done: false,
        }
    }
}

thread_local! {
    static SESSIONS: RefCell<Vec<Option<GameSession>>> = RefCell::new(Vec::new());
}

// ── JS-facing output structs ──────────────────────────────────────────────────

#[derive(Serialize)]
struct JsBall {
    x: f32, y: f32, vx: f32, vy: f32,
    owner: Option<usize>,
    mega: bool,
    cooldown: i32,
    #[serde(rename = "lastTouchTeam")]
    last_touch_team: Option<usize>,
}

#[derive(Serialize)]
struct JsPlayer {
    id: usize,
    team: usize,
    x: f32,
    y: f32,
    role: &'static str,
    state: &'static str,
    #[serde(rename = "knockTimer")]
    knock_timer: i32,
    #[serde(rename = "jumpTimer")]
    jump_timer: i32,
    #[serde(rename = "tackleCooldown")]
    tackle_cooldown: i32,
    #[serde(rename = "slowTimer")]
    slow_timer: i32,
    #[serde(rename = "gkDiveTimer")]
    gk_dive_timer: i32,
    #[serde(rename = "gkDiveDir")]
    gk_dive_dir: Option<&'static str>, // "up" | "down" | null
    facing: &'static str,
    #[serde(rename = "stepCounter")]
    step_counter: i32,
    #[serde(rename = "celebrateTimer")]
    celebrate_timer: i32,
    #[serde(rename = "hairColor")]
    hair_color: &'static str,
    human: bool,
    #[serde(rename = "homeX")]
    home_x: f32,
    #[serde(rename = "homeY")]
    home_y: f32,
    goals: u32,
    shots: u32,
    assists: u32,
}

#[derive(Serialize)]
struct JsEvents {
    #[serde(rename = "goalScored")]
    goal_scored: bool,
    #[serde(rename = "shotTaken")]
    shot_taken: bool,
    #[serde(rename = "passDone")]
    pass_done: bool,
    #[serde(rename = "tackleDone")]
    tackle_done: bool,
}

#[derive(Serialize)]
struct JsGameState {
    pl: Vec<JsPlayer>,
    ball: JsBall,
    score: [u32; 2],
    timer: i32,
    phase: &'static str,
    #[serde(rename = "goalAnim")]
    goal_anim: i32,
    #[serde(rename = "goalTeam")]
    goal_team: Option<usize>,
    #[serde(rename = "setPieceText")]
    set_piece_text: Option<&'static str>,
    #[serde(rename = "setPieceTimer")]
    set_piece_timer: i32,
    #[serde(rename = "penaltyTeam")]
    penalty_team: Option<usize>,
    #[serde(rename = "penaltyTaken")]
    penalty_taken: bool,
    #[serde(rename = "penaltyShotPending")]
    penalty_shot_pending: bool,
    #[serde(rename = "freekickActive")]
    freekick_active: bool,
    #[serde(rename = "freekickShooterId")]
    freekick_shooter_id: Option<usize>,
    #[serde(rename = "setPieceX")]
    set_piece_x: f32,
    #[serde(rename = "setPieceY")]
    set_piece_y: f32,
    #[serde(rename = "gkHasBall")]
    gk_has_ball: [bool; 2],
    celebration: bool,
    #[serde(rename = "celebrateFrame")]
    celebrate_frame: i32,
    #[serde(rename = "_done")]
    done: bool,
    events: JsEvents,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn phase_str(phase: &Phase) -> &'static str {
    match phase {
        Phase::Kickoff  => "kickoff",
        Phase::Playing  => "playing",
        Phase::Goal     => "goal",
        Phase::Penalty  => "penalty",
        Phase::Fulltime => "fulltime",
    }
}

fn role_str(role: Role) -> &'static str {
    match role {
        Role::Fwd => "fwd",
        Role::Mid => "mid",
        Role::Def => "def",
        Role::Gk  => "gk",
    }
}

fn state_str(state: PlayerState) -> &'static str {
    match state {
        PlayerState::Active  => "active",
        PlayerState::Knocked => "knocked",
    }
}

fn facing_from_delta(dx: f32, dy: f32) -> &'static str {
    if dx.abs() > dy.abs() {
        if dx > 0.0 { "right" } else { "left" }
    } else {
        if dy > 0.0 { "down" } else { "up" }
    }
}

fn build_state_json(session: &GameSession) -> String {
    let g = &session.game;

    let pl = g.pl.iter().enumerate().map(|(i, p)| {
        let r = &session.render[i];
        JsPlayer {
            id: p.id,
            team: p.team,
            x: p.x,
            y: p.y,
            role: role_str(p.role),
            state: state_str(p.state),
            knock_timer: p.knock_timer,
            jump_timer: p.jump_timer,
            tackle_cooldown: p.tackle_cooldown,
            slow_timer: p.slow_timer,
            gk_dive_timer: p.gk_dive_timer,
            gk_dive_dir: match p.gk_dive_dir { Some(true) => Some("up"), Some(false) => Some("down"), None => None },
            facing: r.facing,
            step_counter: r.step_counter,
            celebrate_timer: r.celebrate_timer,
            hair_color: HAIR_COLORS[i % 10],
            human: i == 0,
            home_x: p.home_x,
            home_y: p.home_y,
            goals: p.goals,
            shots: p.shots,
            assists: p.assists,
        }
    }).collect();

    let state = JsGameState {
        pl,
        ball: JsBall {
            x: g.ball.x,
            y: g.ball.y,
            vx: g.ball.vx,
            vy: g.ball.vy,
            owner: g.ball.owner,
            mega: g.ball.mega,
            cooldown: g.ball.cooldown,
            last_touch_team: g.ball.last_touch_team,
        },
        score: g.score,
        timer: g.timer,
        phase: phase_str(&g.phase),
        goal_anim: g.goal_anim,
        goal_team: g.goal_team,
        set_piece_text: session.set_piece_text,
        set_piece_timer: g.set_piece_timer,
        penalty_team: g.penalty_team,
        penalty_taken: g.penalty_taken,
        penalty_shot_pending: g.penalty_shot_pending,
        freekick_active: g.free_kick_active,
        freekick_shooter_id: g.free_kick_shooter_id,
        set_piece_x: g.set_piece_x,
        set_piece_y: g.set_piece_y,
        gk_has_ball: g.gk_has_ball,
        celebration: session.celebration,
        celebrate_frame: session.celebrate_frame,
        done: session.done,
        events: JsEvents {
            goal_scored: session.ev_goal_scored,
            shot_taken: session.ev_shot_taken,
            pass_done: session.ev_pass_done,
            tackle_done: session.ev_tackle_done,
        },
    };

    serde_json::to_string(&state).unwrap_or_else(|_| "{}".to_string())
}

// ── WASM API ──────────────────────────────────────────────────────────────────

/// Creates a new game session. Returns opaque handle.
/// team0_json / team1_json: contents of baseline.json (TeamBaselineFileV6 format).
/// Passing empty string uses default V6 parameters.
#[wasm_bindgen]
pub fn create_game(team0_json: &str, team1_json: &str, seed: u32) -> usize {
    let team0 = parse_team_v6(team0_json);
    let team1 = parse_team_v6(team1_json);
    let session = GameSession::new(&team0, &team1, seed);
    SESSIONS.with(|s| {
        let mut sessions = s.borrow_mut();
        for (i, slot) in sessions.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(session);
                return i;
            }
        }
        sessions.push(Some(session));
        sessions.len() - 1
    })
}

#[derive(serde::Deserialize)]
struct TeamBaselineFileV6 {
    #[serde(rename = "playerParams")]
    player_params: TeamPolicyV6,
}

fn parse_team_v6(json: &str) -> TeamPolicyV6 {
    if json.is_empty() {
        return core::array::from_fn(|i| training_engine::policy::v6_default_for_slot(i));
    }
    serde_json::from_str::<TeamBaselineFileV6>(json)
        .map(|w| w.player_params)
        .unwrap_or_else(|_| core::array::from_fn(|i| training_engine::policy::v6_default_for_slot(i)))
}

/// Advances the game by one frame.
/// input_json: JSON with optional human input fields (dx, dy, shoot, pass_action, etc.).
/// Returns serialized JsGameState.
#[wasm_bindgen]
pub fn step_game(handle: usize, input_json: &str) -> String {
    SESSIONS.with(|s| {
        let mut sessions = s.borrow_mut();
        let session = match sessions.get_mut(handle).and_then(|s| s.as_mut()) {
            Some(s) => s,
            None => return "{}".to_string(),
        };

        let input: HumanInput = if input_json.is_empty() || input_json == "null" {
            HumanInput::default()
        } else {
            serde_json::from_str(input_json).unwrap_or_default()
        };

        apply_step(session, &input);
        build_state_json(session)
    })
}

fn apply_step(session: &mut GameSession, input: &HumanInput) {
    // Reset per-frame events
    session.ev_goal_scored = false;
    session.ev_shot_taken = false;
    session.ev_pass_done = false;
    session.ev_tackle_done = false;

    let g = &mut session.game;

    match g.phase {
        Phase::Kickoff => {
            let has_input = input.dx.abs() > 0.01 || input.dy.abs() > 0.01
                || input.shoot || input.tackle || input.jump;
            if has_input {
                g.phase = Phase::Playing;
            }
            return;
        }
        Phase::Goal => {
            g.goal_anim -= 1;
            if session.celebration {
                session.celebrate_frame += 1;
                if session.render[0].celebrate_timer > 0 {
                    session.render[0].celebrate_timer -= 1;
                }
            }
            if input.celebrate && session.celebration {
                session.render[0].celebrate_timer = 55;
            }
            if g.goal_anim <= 0 {
                training_engine::physics::reset_kickoff(g);
                session.set_piece_text = Some("AVSPARK");
                session.celebration = false;
                session.celebrate_frame = 0;
            }
            return;
        }
        Phase::Penalty => {
            if g.penalty_team == Some(0) && !g.penalty_taken {
                if let Some(owner_id) = g.ball.owner {
                    if owner_id == 0 && (input.shoot || input.mega_shoot) {
                        let [dx, dy] = input.pass_dir.unwrap_or([1.0, 0.0]);
                        let tx = g.pl[0].x + dx * 220.0;
                        let ty = g.pl[0].y + dy * 220.0;
                        do_shoot(g, 0, false, tx, ty, Some(SHOOT_POW), false);
                        g.phase = Phase::Playing;
                        g.penalty_taken = true;
                        session.set_piece_text = None;
                        session.ev_shot_taken = true;
                        return;
                    }
                    let (sx, sy) = (g.pl[0].x, g.pl[0].y);
                    g.ball.x = sx; g.ball.y = sy;
                }
            }
            rust_step_game(g, &mut session.rng);
            update_set_piece_text(session);
            return;
        }
        Phase::Fulltime => {
            if !session.done {
                session.done = true;
            }
            return;
        }
        Phase::Playing => {}
    }

    // ── Apply human movement ──────────────────────────────────────────────────
    let human_idx = 0usize;
    let prev_x = g.pl[human_idx].x;
    let prev_y = g.pl[human_idx].y;

    if g.pl[human_idx].state == PlayerState::Active {
        // Decrement human timers (Rust step_game does this for AI players but skips human)
        if g.pl[human_idx].tackle_cooldown > 0 { g.pl[human_idx].tackle_cooldown -= 1; }
        if g.pl[human_idx].jump_timer > 0 { g.pl[human_idx].jump_timer -= 1; }
        if g.pl[human_idx].slow_timer > 0 { g.pl[human_idx].slow_timer -= 1; }

        let slow_mult = if g.pl[human_idx].slow_timer > 0 { 0.5 } else { 1.0 };
        let mut dx = input.dx;
        let mut dy = input.dy;
        if dx != 0.0 && dy != 0.0 {
            dx *= 0.707;
            dy *= 0.707;
        }
        let nx = (g.pl[human_idx].x + dx * PSPEED * slow_mult).clamp(PR, FW - PR);
        let ny = (g.pl[human_idx].y + dy * PSPEED * slow_mult).clamp(PR, FH - PR);
        g.pl[human_idx].x = nx;
        g.pl[human_idx].y = ny;

        if g.ball.owner == Some(0) {
            if input.shoot || input.mega_shoot {
                do_shoot(g, human_idx, input.mega_shoot, FW + GD, H2, None, false);
                session.ev_shot_taken = true;
            } else if input.pass_action {
                if let Some([pdx, pdy]) = input.pass_dir {
                    let tx = g.pl[human_idx].x + pdx * 180.0;
                    let ty = g.pl[human_idx].y + pdy * 180.0;
                    do_shoot(g, human_idx, false, tx, ty, Some(PASS_POW), true);
                    session.ev_pass_done = true;
                } else if let Some(pass_result) = cpu_find_pass(g, human_idx) {
                    let tidx = g.pl.iter().position(|p| p.id == pass_result.target_id).unwrap_or(1);
                    let tx = g.pl[tidx].x;
                    let ty = g.pl[tidx].y;
                    do_shoot(g, human_idx, false, tx, ty, Some(PASS_POW), true);
                    session.ev_pass_done = true;
                }
            }
        }

        if input.tackle && g.pl[human_idx].tackle_cooldown <= 0 {
            let carrier_idx = g.pl.iter().position(|p| g.ball.owner == Some(p.id) && p.team != 0);
            if let Some(cidx) = carrier_idx {
                let dist = (g.pl[human_idx].x - g.pl[cidx].x).hypot(g.pl[human_idx].y - g.pl[cidx].y);
                if dist < TACKLE_DIST {
                    tackle_player(g, human_idx, cidx);
                    session.ev_tackle_done = true;
                }
            }
        }

        if input.jump && g.pl[human_idx].jump_timer <= 0 {
            g.pl[human_idx].jump_timer = JUMP_DUR;
        }
    }

    // ── Update facing for human player ────────────────────────────────────────
    let ddx = g.pl[human_idx].x - prev_x;
    let ddy = g.pl[human_idx].y - prev_y;
    if ddx.hypot(ddy) > 0.1 {
        session.render[human_idx].facing = facing_from_delta(ddx, ddy);
        session.render[human_idx].step_counter += 1;
    }

    // ── Run Rust simulation step (skips human player AI via human_player flag) ─
    let prev_positions: Vec<(f32, f32)> = g.pl.iter().map(|p| (p.x, p.y)).collect();
    let prev_phase = g.phase;
    rust_step_game(g, &mut session.rng);

    // ── Detect goal for events/celebration ────────────────────────────────────
    if g.phase == Phase::Goal && prev_phase == Phase::Playing {
        session.ev_goal_scored = true;
        if g.goal_team == Some(0) {
            session.celebration = true;
            session.celebrate_frame = 0;
        }
    }

    // ── Update facing for AI players ─────────────────────────────────────────
    for i in 1..g.pl.len() {
        let (px, py) = prev_positions[i];
        let ddx = g.pl[i].x - px;
        let ddy = g.pl[i].y - py;
        if ddx.hypot(ddy) > 0.1 {
            session.render[i].facing = facing_from_delta(ddx, ddy);
            session.render[i].step_counter += 1;
        }
    }

    update_set_piece_text(session);
}

fn update_set_piece_text(session: &mut GameSession) {
    let g = &session.game;
    if g.set_piece_timer == 0 {
        session.set_piece_text = None;
    }
}

/// Toggles whether player 0 is human-controlled or AI-controlled.
/// Call with active=false to let Rust AI take over player 0.
#[wasm_bindgen]
pub fn set_human_player(handle: usize, active: bool) {
    SESSIONS.with(|s| {
        let mut sessions = s.borrow_mut();
        if let Some(Some(session)) = sessions.get_mut(handle) {
            session.game.human_player = if active { Some(0) } else { None };
        }
    });
}

/// Frees a game session.
#[wasm_bindgen]
pub fn destroy_game(handle: usize) {
    SESSIONS.with(|s| {
        let mut sessions = s.borrow_mut();
        if handle < sessions.len() {
            sessions[handle] = None;
        }
    });
}

/// Runs a full simulation (no rendering). Returns JSON: {score0, score1}.
#[wasm_bindgen]
pub fn run_simulation(team0_json: &str, team1_json: &str, seed: u32) -> String {
    use rand::SeedableRng;
    let team0 = parse_team_v6(team0_json);
    let team1 = parse_team_v6(team1_json);
    let mut game = Game::for_team_battle_v6(&team0, &team1);
    let mut rng = rand::rngs::SmallRng::seed_from_u64(seed as u64);
    while game.phase != Phase::Fulltime {
        rust_step_game(&mut game, &mut rng);
    }
    serde_json::json!({
        "score0": game.score[0],
        "score1": game.score[1],
    }).to_string()
}
