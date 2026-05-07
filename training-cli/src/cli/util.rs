use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde_json::Value;

use training_engine::policy::TeamPolicyV6;

use training_render::write_team_layout_svg;
use super::teams::{lookup_team_desc, write_team_info_md};

// ── Tid ──────────────────────────────────────────────────────────────────────

pub fn iso_now() -> String {
    let d = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let (y, mo, day, h, mi, s) = unix_to_datetime(d.as_secs());
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, mo, day, h, mi, s)
}

pub fn unix_to_datetime(mut ts: u64) -> (u64, u64, u64, u64, u64, u64) {
    let s = ts % 60; ts /= 60;
    let mi = ts % 60; ts /= 60;
    let h = ts % 24; ts /= 24;
    let mut days = ts;
    let mut year = 1970u64;
    loop {
        let dy = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) { 366 } else { 365 };
        if days < dy { break; }
        days -= dy;
        year += 1;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let months = [31u64, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u64;
    for &m in &months {
        if days < m { break; }
        days -= m;
        month += 1;
    }
    (year, month, days + 1, h, mi, s)
}

// ── Sessions ─────────────────────────────────────────────────────────────────

pub fn numeric_session_sort(dirs: &mut Vec<String>) {
    dirs.sort_by(|a, b| {
        let key = |s: &str| -> (Option<u64>, String) {
            let trailing: String = s.chars().rev()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>().chars().rev().collect();
            let n = trailing.parse::<u64>().ok();
            let prefix = s[..s.len() - trailing.len()].to_string();
            (n, prefix)
        };
        let (na, pa) = key(a);
        let (nb, pb) = key(b);
        pa.cmp(&pb).then_with(|| match (na, nb) {
            (Some(x), Some(y)) => x.cmp(&y),
            _ => a.cmp(b),
        })
    });
}

#[allow(dead_code)]
pub fn list_sessions(sessions_dir: &Path) -> Vec<String> {
    let mut dirs: Vec<String> = std::fs::read_dir(sessions_dir)
        .ok()
        .into_iter()
        .flat_map(|rd| rd.filter_map(|entry| {
            let entry = entry.ok()?;
            if !entry.file_type().ok()?.is_dir() { return None; }
            entry.file_name().into_string().ok()
        }))
        .collect();
    numeric_session_sort(&mut dirs);
    dirs
}

#[allow(dead_code)]
pub fn compute_total_stats(sessions_dir: &Path) -> (u64, f64) {
    let mut total_matches: u64 = 0;
    let mut total_ms: u128 = 0;
    if let Ok(rd) = std::fs::read_dir(sessions_dir) {
        for entry in rd.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) { continue; }
            let summary_path = entry.path().join("summary.json");
            if !summary_path.exists() { continue; }
            if let Ok(text) = std::fs::read_to_string(&summary_path) {
                if let Ok(val) = serde_json::from_str::<Value>(&text) {
                    let games_per_epoch = val["gamesPerEpoch"].as_u64().unwrap_or(0);
                    if let Some(history) = val["history"].as_array() {
                        for h in history {
                            let games = h["gamesRun"].as_u64().unwrap_or(games_per_epoch);
                            total_matches += games;
                        }
                    }
                    if let Some(ms) = val["totalTrainingElapsedMs"].as_u64() {
                        total_ms += ms as u128;
                    }
                }
            }
        }
    }
    (total_matches, total_ms as f64 / 60_000.0)
}

// ── JSON ─────────────────────────────────────────────────────────────────────

/// Skriver pretty-printad JSON följt av newline. Felet ignoreras (eldfast).
pub fn write_json_pretty(path: &Path, value: &Value) {
    let _ = std::fs::write(path, format!("{}\n", serde_json::to_string_pretty(value).unwrap()));
}

// ── Lag ──────────────────────────────────────────────────────────────────────

/// Lista alla lag-mappar under `teams_dir` som har `baseline.json`. Sorterad efter namn.
pub fn list_team_dirs(teams_dir: &Path) -> Vec<(String, PathBuf)> {
    let mut out = vec![];
    if let Ok(entries) = std::fs::read_dir(teams_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() { continue; }
            if !path.join("baseline.json").exists() { continue; }
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            out.push((name, path));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Trio: baseline.json + info.md + layout.svg. Ersätter ~6 rader upprepad kod.
pub fn save_team_artifacts(
    team_dir: &Path,
    team_name: &str,
    params: &TeamPolicyV6,
    description: &str,
    training_method: &str,
    version: u64,
) {
    let doc = serde_json::json!({
        "name": team_name, "version": version,
        "type": "team-policy-v6",
        "description": description,
        "playerParams": params,
        "trainedAt": iso_now(),
        "trainingMethod": training_method,
    });
    write_json_pretty(&team_dir.join("baseline.json"), &doc);
    let team_desc = lookup_team_desc(team_name);
    write_team_info_md(team_dir, team_name, team_desc, params);
    write_team_layout_svg(&team_dir.join("layout.svg"), team_name, team_desc, params);
}

pub fn v6_diff_slots(a: &TeamPolicyV6, b: &TeamPolicyV6) -> Vec<usize> {
    (0..5).filter(|&i| {
        let pa = serde_json::to_value(a[i]).unwrap();
        let pb = serde_json::to_value(b[i]).unwrap();
        pa != pb
    }).collect()
}
