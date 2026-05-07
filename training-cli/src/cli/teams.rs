use std::path::Path;

use training_engine::policy::TeamPolicyV6;

pub const TEAM_NAMES: &[&str] = &[
    "aurora-fc",
    "granite-athletic",
    "phoenix-rovers",
    "tempest-united",
    "mirage-sc",
    "eclipse-town",
    "catalyst-city",
    "vortex-galaxy",
    "glacier-fc",
    "nebula-rangers",
];

pub const TEAM_DESCRIPTIONS: &[&str] = &[
    "Aurora FC — graceful and fluid",
    "Granite Athletic — solid and robust",
    "Phoenix Rovers — energetic, comeback-prone",
    "Tempest United — stormy, chaotic press",
    "Mirage SC — deceptive, unpredictable",
    "Eclipse Town — dark horse, counter-attack",
    "Catalyst City — fast transitions",
    "Vortex Galaxy — possession circulation",
    "Glacier FC — slow but inevitable",
    "Nebula Rangers — diffuse, exploratory",
];

/// Slå upp tränarbeskrivning efter lagnamn. Faller tillbaka till lagnamn självt.
pub fn lookup_team_desc(team_name: &str) -> &str {
    TEAM_NAMES.iter()
        .position(|n| *n == team_name)
        .and_then(|i| TEAM_DESCRIPTIONS.get(i))
        .copied()
        .unwrap_or(team_name)
}

pub fn write_team_info_md(team_dir: &Path, team_name: &str, description: &str, params: &TeamPolicyV6) {
    let mut md = String::new();
    md.push_str(&format!("# {}\n\n", team_name));
    md.push_str(&format!("> {}\n\n", description));
    md.push_str("Trained from clustered start (all field players at centre of own half) via\n");
    md.push_str("adaptive multi-stage anneal. Spatial preferences emerged organically from\n");
    md.push_str("mutation + selection — no positional logic was hand-coded.\n\n");
    md.push_str("## Spatial preferences (min / preferred / max)\n\n");
    md.push_str("| Slot | own_goal | side | ball | teammate | opponent |\n");
    md.push_str("|------|----------|------|------|----------|----------|\n");
    let slot_names = ["FWD", "MID-T", "MID-B", "DEF", "GK"];
    for i in 0..5 {
        let s = &params[i].spatial;
        md.push_str(&format!(
            "| {} | {:.0}/{:.0}/{:.0} | {:.0}/{:.0}/{:.0} | {:.0}/{:.0}/{:.0} | {:.0}/{:.0}/{:.0} | {:.0}/{:.0}/{:.0} |\n",
            slot_names[i],
            s.own_goal.min, s.own_goal.preferred, s.own_goal.max,
            s.side.min, s.side.preferred, s.side.max,
            s.ball.min, s.ball.preferred, s.ball.max,
            s.teammate.min, s.teammate.preferred, s.teammate.max,
            s.opponent.min, s.opponent.preferred, s.opponent.max,
        ));
    }
    md.push_str("\n## Decision parameters\n\n");
    md.push_str("| Slot | tackle | shoot_thr | aggr | risk | passDirOff | passDirDef |\n");
    md.push_str("|------|--------|-----------|------|------|------------|------------|\n");
    for i in 0..5 {
        let d = &params[i].decisions;
        md.push_str(&format!("| {} | {:.3} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} |\n",
            slot_names[i], d.tackle_chance, d.shoot_progress_threshold,
            d.aggression, d.risk_appetite, d.pass_dir_offensive, d.pass_dir_defensive));
    }
    md.push_str("\n## Inferred strategy\n\n");
    md.push_str("_(filled in after tournament analysis — see matrix in `data/matrices/`)_\n\n");
    md.push_str("## Tournament\n\n_(filled in after `--v6-tournament` run)_\n");
    let _ = std::fs::write(team_dir.join("info.md"), md);
}
