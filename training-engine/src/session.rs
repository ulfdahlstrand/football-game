use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::policy::PolicyParams;
use crate::trainer::{EvalResult, EarlyStop};

#[derive(Deserialize)]
pub struct BaselineFile {
    pub name: Option<String>,
    pub version: Option<u32>,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub policy_type: Option<String>,
    pub parameters: PolicyParams,
}

pub fn read_baseline(path: &Path) -> anyhow::Result<BaselineFile> {
    let text = fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Cannot read {}: {}", path.display(), e))?;
    let baseline: BaselineFile = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("Cannot parse {}: {}", path.display(), e))?;
    Ok(baseline)
}

pub fn write_json(path: &Path, value: &Value) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(value)?;
    fs::write(path, format!("{}\n", text))?;
    Ok(())
}

fn eval_result_to_json(r: &EvalResult) -> Value {
    serde_json::json!({
        "games": r.games,
        "maxGames": r.max_games,
        "elapsedMs": r.elapsed_ms,
        "gamesPerSecond": if r.elapsed_ms > 0 { (r.games as f64 / (r.elapsed_ms as f64 / 1000.0) * 100.0).round() / 100.0 } else { 0.0 },
        "baselineAvgGoals": r.baseline_avg_goals,
        "candidateAvgGoals": r.candidate_avg_goals,
        "goalDiff": r.goal_diff,
        "candidateWon": r.candidate_won,
        "earlyStop": r.early_stop.map(|s| match s { EarlyStop::Worse => "worse", EarlyStop::Better => "better" }),
        "zScore": r.z_score,
        "avgPasses": r.avg_passes,
        "passCompletionRate": r.pass_completion_rate,
        "avgShots": r.avg_shots,
        "avgGoals": r.avg_goals,
        "avgTackles": r.avg_tackles,
        "tackleSuccessRate": r.tackle_success_rate,
        "avgOutOfBounds": r.avg_out_of_bounds,
    })
}

pub struct SessionWriter {
    session_dir: PathBuf,
    session_name: String,
}

impl SessionWriter {
    pub fn new(policies_dir: &Path, session_name: &str) -> anyhow::Result<Self> {
        let session_dir = policies_dir.join("sessions").join(session_name);
        fs::create_dir_all(&session_dir)?;
        Ok(Self { session_dir, session_name: session_name.to_string() })
    }

    pub fn write_initial_baseline(&self, params: &PolicyParams, created_at: &str) -> anyhow::Result<()> {
        let v = serde_json::json!({
            "name": "epoch-000-baseline",
            "session": self.session_name,
            "epoch": 0,
            "createdAt": created_at,
            "role": "initial-baseline",
            "parameters": params,
        });
        write_json(&self.session_dir.join("epoch-000-baseline.json"), &v)
    }

    pub fn write_epoch(
        &self,
        epoch: usize,
        opponent_epoch: usize,
        opponent_params: &PolicyParams,
        candidate_params: &PolicyParams,
        accepted: bool,
        champion_epoch: usize,
        champion_params: &PolicyParams,
        eval: &EvalResult,
        created_at: &str,
        games_per_epoch: usize,
    ) -> anyhow::Result<()> {
        let name = format!("epoch-{:03}", epoch);
        let v = serde_json::json!({
            "name": name,
            "session": self.session_name,
            "epoch": epoch,
            "createdAt": created_at,
            "gamesPerEpoch": games_per_epoch,
            "opponent": {
                "sourceEpoch": opponent_epoch,
                "parameters": opponent_params,
            },
            "candidate": {
                "parameters": candidate_params,
            },
            "accepted": accepted,
            "championEpoch": champion_epoch,
            "championParameters": champion_params,
            "evaluation": eval_result_to_json(eval),
        });
        write_json(&self.session_dir.join(format!("{}.json", name)), &v)
    }

    pub fn write_summary(
        &self,
        started_at: &str,
        finished_at: &str,
        epochs: usize,
        games_per_epoch: usize,
        final_champion_epoch: usize,
        final_champion_params: &PolicyParams,
        history: &[EpochSummary],
    ) -> anyhow::Result<()> {
        let accepted_epochs: Vec<usize> = history.iter().filter(|h| h.accepted).map(|h| h.epoch).collect();
        let best_goal_diff = history.iter().map(|h| h.goal_diff).fold(f64::NEG_INFINITY, f64::max);
        let total_ms: u128 = history.iter().map(|h| h.elapsed_ms).sum();
        let avg_ms = if !history.is_empty() { total_ms / history.len() as u128 } else { 0 };

        let history_json: Vec<Value> = history.iter().map(|h| serde_json::json!({
            "epoch": h.epoch,
            "accepted": h.accepted,
            "championEpoch": h.champion_epoch,
            "goalDiff": h.goal_diff,
            "baselineAvgGoals": h.baseline_avg_goals,
            "candidateAvgGoals": h.candidate_avg_goals,
            "elapsedMs": h.elapsed_ms,
            "earlyStop": h.early_stop,
            "zScore": h.z_score,
            "gamesRun": h.games_run,
        })).collect();

        let v = serde_json::json!({
            "name": self.session_name,
            "startedAt": started_at,
            "finishedAt": finished_at,
            "epochs": epochs,
            "gamesPerEpoch": games_per_epoch,
            "finalChampionEpoch": final_champion_epoch,
            "finalChampionParameters": final_champion_params,
            "acceptedEpochs": accepted_epochs,
            "acceptedCount": history.iter().filter(|h| h.accepted).count(),
            "rejectedCount": history.iter().filter(|h| !h.accepted).count(),
            "bestGoalDiff": best_goal_diff,
            "averageEpochElapsedMs": avg_ms,
            "totalTrainingElapsedMs": total_ms,
            "history": history_json,
        });
        write_json(&self.session_dir.join("summary.json"), &v)
    }

    pub fn write_best(
        &self,
        champion_epoch: usize,
        champion_params: &PolicyParams,
        session_name: &str,
    ) -> anyhow::Result<()> {
        let v = serde_json::json!({
            "name": format!("{}-best", session_name),
            "version": 1,
            "type": "rule-policy",
            "sourceSession": session_name,
            "sourceEpoch": champion_epoch,
            "parameters": champion_params,
        });
        write_json(&self.session_dir.join("best.json"), &v)
    }

    pub fn session_dir(&self) -> &Path {
        &self.session_dir
    }
}

pub fn update_baseline(
    baseline_path: &Path,
    original: &BaselineFile,
    champion_params: &PolicyParams,
    session_name: &str,
    champion_epoch: usize,
    goal_diff: f64,
    updated_at: &str,
) -> anyhow::Result<()> {
    // Read current file as raw JSON to extract version counter and history chain.
    let current_raw: Value = fs::read_to_string(baseline_path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or(serde_json::json!({}));

    let next_version = current_raw["version"].as_u64().unwrap_or(1) + 1;

    // Build the new history entry.
    let new_entry = serde_json::json!({
        "session": session_name,
        "epoch": champion_epoch,
        "goalDiff": goal_diff,
        "updatedAt": updated_at,
    });

    // Preserve existing history and append.
    let mut history = current_raw["history"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    history.push(new_entry);

    let v = serde_json::json!({
        "name": original.name.as_deref().unwrap_or("baseline"),
        "version": next_version,
        "description": original.description.as_deref().unwrap_or(""),
        "type": original.policy_type.as_deref().unwrap_or("rule-policy"),
        "parameters": champion_params,
        "updatedAt": updated_at,
        "updatedBySession": session_name,
        "updatedByEpoch": champion_epoch,
        "latestGoalDiff": goal_diff,
        "history": history,
    });
    write_json(baseline_path, &v)
}

/// Creates baseline-genesis.json next to baseline.json if it does not already exist.
/// Called once at session start so new installations get a genesis reference point.
pub fn ensure_genesis(baseline_path: &Path, original: &BaselineFile) {
    let genesis_path = baseline_path.parent().unwrap().join("baseline-genesis.json");
    if genesis_path.exists() { return; }
    let v = serde_json::json!({
        "name": "baseline-genesis",
        "version": 1,
        "description": "Snapshot of baseline at first training session. Never modified. Used as anchor to measure total improvement.",
        "type": original.policy_type.as_deref().unwrap_or("rule-policy"),
        "createdAt": "auto",
        "parameters": original.parameters,
    });
    let _ = write_json(&genesis_path, &v);
}

#[derive(Clone)]
pub struct EpochSummary {
    pub epoch: usize,
    pub accepted: bool,
    pub champion_epoch: usize,
    pub goal_diff: f64,
    pub baseline_avg_goals: f64,
    pub candidate_avg_goals: f64,
    pub elapsed_ms: u128,
    pub early_stop: Option<String>,
    pub z_score: f64,
    pub games_run: usize,
}
