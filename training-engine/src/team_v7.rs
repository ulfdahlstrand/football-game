use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::detector::{detect_global, GlobalBehavior};
use crate::game::Game;
use crate::policy::{V6Params, V6Spatial, DistancePref, TeamPolicyV6};
use crate::team::Team;

// ── Coach directive ──────────────────────────────────────────────────────────

/// Aktiv taktisk instruktion från coachen, uppdateras ~var 300:e tick.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CoachDirective {
    /// 0=djup/defensivt press, 1=högt press
    pub press_intensity: f32,
    /// 0=djup backlinje, 1=hög linje
    pub line_height: f32,
    /// 0=brett, 1=kompakt
    pub compactness: f32,
    /// 0=håll boll/lugn, 1=direktspel/högt tempo
    pub tempo: f32,
}

impl Default for CoachDirective {
    fn default() -> Self {
        Self { press_intensity: 0.5, line_height: 0.5, compactness: 0.4, tempo: 0.5 }
    }
}

// ── Coach style (träningsbara params) ────────────────────────────────────────

/// Coachens reaktionsstil — styr hur direktiven uppdateras utifrån GlobalBehavior.
/// Dessa tränas tillsammans med V7-spelarna.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CoachStyle {
    /// Hur aggressivt coachen svarar på motståndartryck (0=passiv, 1=press tillbaka)
    pub press_response: f32,
    /// Hur kraftigt coachen justerar linjedjup utifrån utrymme bakom (0=stabil, 1=reaktiv)
    pub depth_response: f32,
    /// Basnivå kompakthet oberoende av motståndare [0,1]
    pub compactness_base: f32,
    /// Bastempo oberoende av matchläge [0,1]
    pub tempo_base: f32,
}

impl Default for CoachStyle {
    fn default() -> Self {
        Self { press_response: 0.5, depth_response: 0.5, compactness_base: 0.4, tempo_base: 0.5 }
    }
}

// ── Coach ────────────────────────────────────────────────────────────────────

pub struct Coach {
    pub directive: CoachDirective,
    pub style: CoachStyle,
}

impl Coach {
    pub fn new(style: CoachStyle) -> Self {
        Self { directive: CoachDirective::default(), style }
    }

    pub fn update(&mut self, game: &Game, team_id: usize) {
        let b = detect_global(game, team_id);
        self.directive = compute_directive(&b, &self.style);
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

fn compute_directive(b: &GlobalBehavior, style: &CoachStyle) -> CoachDirective {
    // Press intensity: hög när motståndet pressar OCH press_response är hög;
    // låg (sitter djupt) när motståndet pressar och press_response är låg (absorb-taktik).
    let press_intensity = if b.opp_press_rate > 0.5 {
        lerp(0.2, 0.9, style.press_response)
    } else {
        lerp(0.3, 0.7, style.press_response * 0.6)
    };

    // Linjedjup: exploatera utrymmet bakom motståndet om depth_response är hög
    let line_height = lerp(0.3, 0.85, b.space_behind * style.depth_response);

    // Kompakthet: baseras på style + anpassas när motståndaren är högt
    let opp_is_high = b.opp_avg_x > 0.6; // deras spelare långt framåt = vi är pressade
    let compactness = if opp_is_high {
        (style.compactness_base + 0.25).min(1.0)
    } else {
        style.compactness_base
    };

    // Tempo: öka när vi har utrymme och motståndaren inte pressar
    let tempo = if b.space_behind > 0.5 && b.opp_press_rate < 0.4 {
        (style.tempo_base + 0.2).min(1.0)
    } else if b.opp_press_rate > 0.6 {
        (style.tempo_base - 0.2).max(0.0)
    } else {
        style.tempo_base
    };

    CoachDirective { press_intensity, line_height, compactness, tempo }
}

// ── V7Player ─────────────────────────────────────────────────────────────────

/// En spelare i V7-systemet. Äger sin instinkt (V6Params) + coachability.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct V7Player {
    /// 0=ren instinkt (V6), 1=följer coachen fullt ut
    pub coachability: f32,
    /// V6-parametrar som bas/instinkt
    pub instinct: V6Params,
}

impl V7Player {
    pub fn from_v6(instinct: V6Params) -> Self {
        Self { coachability: 0.5, instinct }
    }
}

/// Justerar V6Params med ett additivt delta baserat på CoachDirective.
/// Neutral direktiv (alla=0.5) → noll förändring → idempotent mot instinkt.
/// coachability=0 → ingen effekt; coachability=1 → full direktiv-delta.
pub fn apply_directive(
    instinct: &V6Params,
    dir: &CoachDirective,
    coachability: f32,
) -> V6Params {
    let t = coachability.clamp(0.0, 1.0);
    if t < 1e-3 { return *instinct; }

    // Signerade deltas: dir=0.5 → delta=0, dir=1.0 → delta=+1, dir=0.0 → delta=-1
    let press_delta = (dir.press_intensity - 0.5) * 2.0; // [-1, +1]
    let tempo_delta  = (dir.tempo - 0.5) * 2.0;
    let line_delta   = (dir.line_height - 0.5) * 2.0;
    let compact_delta = (dir.compactness - 0.5) * 2.0;

    let mut coached = *instinct;
    let d = &instinct.decisions;

    // Press → aggression och tackle (additivt)
    coached.decisions.aggression    = (d.aggression   + t * press_delta * 0.30).max(0.0);
    coached.decisions.tackle_chance = (d.tackle_chance + t * press_delta * 0.03).clamp(0.01, 0.30);

    // Tempo → pass-chanser (additivt; positiv tempo = passa mer)
    coached.decisions.pass_chance_pressured = (d.pass_chance_pressured + t * tempo_delta * 0.04).clamp(0.01, 0.40);
    coached.decisions.pass_chance_default   = (d.pass_chance_default   + t * tempo_delta * 0.015).clamp(0.005, 0.20);
    coached.decisions.pass_chance_forward   = (d.pass_chance_forward   + t * tempo_delta * 0.012).clamp(0.005, 0.18);
    coached.decisions.pass_chance_wing      = (d.pass_chance_wing      + t * tempo_delta * 0.015).clamp(0.005, 0.20);

    // Riskaptit: högt press → mer offensiv riskvilja
    coached.decisions.risk_appetite = (d.risk_appetite + t * press_delta * 0.12).clamp(0.0, 1.0);

    // Passriktning: högt tempo → mer offensivpass
    coached.decisions.pass_dir_offensive = (d.pass_dir_offensive + t * tempo_delta * 0.15).max(0.1);
    coached.decisions.pass_dir_defensive = (d.pass_dir_defensive - t * tempo_delta * 0.10).max(0.1);

    // Rumslig preferens (GK orörd)
    if instinct.gk.is_none() {
        let s = &instinct.spatial;
        // Linjedjup: positiv → spela högre upp (own_goal.preferred ökar)
        coached.spatial.own_goal = DistancePref {
            min:       s.own_goal.min,
            max:       s.own_goal.max,
            preferred: (s.own_goal.preferred + t * line_delta * 60.0)
                .clamp(s.own_goal.min, s.own_goal.max),
        };
        // Kompakthet: positiv → täta ihop (teammate.preferred minskar)
        coached.spatial = V6Spatial {
            own_goal: coached.spatial.own_goal,
            side:     s.side,
            ball:     s.ball,
            teammate: DistancePref {
                min:       s.teammate.min,
                max:       s.teammate.max,
                preferred: (s.teammate.preferred - t * compact_delta * 25.0)
                    .clamp(s.teammate.min, s.teammate.max),
            },
            opponent: s.opponent,
        };
    }

    coached
}

// ── V7Team ───────────────────────────────────────────────────────────────────

pub struct V7Team {
    id: usize,
    pub coach: Coach,
    pub players: [V7Player; 5],
}

impl V7Team {
    pub fn new(id: usize, policy: TeamPolicyV6, style: CoachStyle, coachability: [f32; 5]) -> Self {
        let players = std::array::from_fn(|slot| {
            V7Player {
                coachability: coachability[slot].clamp(0.0, 1.0),
                instinct: policy[slot],
            }
        });
        Self { id, coach: Coach::new(style), players }
    }

    /// Skapar ett V7Team med standardvärden (coachability=0.5, neutral CoachStyle).
    pub fn from_v6(id: usize, policy: TeamPolicyV6) -> Self {
        Self::new(id, policy, CoachStyle::default(), [0.5; 5])
    }
}

impl Team for V7Team {
    fn pre_tick(&mut self, game: &Game) {
        // Uppdatera direktiv var 300:e tick (≈5 sekunder i speltid)
        if game.timer % 300 == 0 {
            self.coach.update(game, self.id);
        }
    }

    fn tick_player(&mut self, game: &mut Game, player_idx: usize, rng: &mut dyn RngCore) {
        let slot = player_idx % 5;
        let vp = &self.players[slot];
        let coached = apply_directive(&vp.instinct, &self.coach.directive, vp.coachability);
        crate::ai::v6_tick(game, player_idx, &coached, rng);
    }

    fn team_id(&self) -> usize {
        self.id
    }
}
