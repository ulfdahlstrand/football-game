#!/usr/bin/env python3
"""
Coach Nudge Tool — kontrollerad manuell parameterändring

Regler:
- En nudge tillåts per 100 000 träningsevalueringar sedan förra nudgen
- Nudge-beloppet får aldrig överstiga max_delta (typisk epochmutation ≈ 0.05)
- Parametervärdet klampas till [param_min, param_max]
- All aktivitet loggas i <team>/nudge_log.json

Användning:
  python3 coach_nudge.py status <team>
  python3 coach_nudge.py nudge <team> <slot> <param> <delta>
  python3 coach_nudge.py record-training <team> <evaluations>
"""

import json
import sys
import os
from datetime import datetime
from pathlib import Path

TEAMS_DIR = Path(__file__).parent / "data" / "teams"
EVALS_PER_NUDGE = 100_000   # träningsevalueringar per tillåten nudge
MAX_DELTA = 0.05             # max beloppet per nudge (typisk epoch-mutation)

# Parameterens giltiga spann
PARAM_BOUNDS = {
    "aggression":              (0.0, 2.0),
    "forwardPassMinGain":      (0.0, 30.0),
    "markDistance":            (0.0, 200.0),
    "passChanceDefault":       (0.0, 1.0),
    "passChanceForward":       (0.0, 1.0),
    "passChancePressured":     (0.0, 1.0),
    "passChanceWing":          (0.0, 1.0),
    "passDirDefensive":        (0.0, 2.0),
    "passDirNeutral":          (0.0, 2.0),
    "passDirOffensive":        (0.0, 2.0),
    "riskAppetite":            (0.0, 1.0),
    "shootProgressThreshold":  (0.0, 1.0),
    "tackleChance":            (0.0, 1.0),
    "gkDistributionZone":      (0.0, 1.0),
    "gkDiveChance":            (0.0, 1.0),
    "gkDiveCommitDist":        (0.0, 400.0),
    "gkPassTargetDist":        (0.0, 200.0),
    "gkRiskClearance":         (0.0, 1.0),
}

SLOT_NAMES = ["fwd (Orion Vex)", "mid (Lyra Cass)", "mid (Quasar Dyne)",
              "def (Nova Stern)", "gk (Cosmo Rael)"]


def load_nudge_log(team_dir: Path) -> dict:
    path = team_dir / "nudge_log.json"
    if path.exists():
        with open(path) as f:
            return json.load(f)
    return {"total_evals_recorded": 0, "evals_since_last_nudge": 0, "nudges": []}


def save_nudge_log(team_dir: Path, log: dict):
    path = team_dir / "nudge_log.json"
    with open(path, "w") as f:
        json.dump(log, f, indent=2)


def cmd_status(team: str):
    team_dir = TEAMS_DIR / team
    if not team_dir.exists():
        print(f"Fel: lag '{team}' hittades inte.")
        sys.exit(1)
    log = load_nudge_log(team_dir)
    evals = log["evals_since_last_nudge"]
    available = evals // EVALS_PER_NUDGE
    print(f"=== Nudge-status: {team} ===")
    print(f"Evalueringar sedan sista nudge: {evals:,}")
    print(f"Tillgängliga nudges:             {available}")
    print(f"Nästa nudge om:                 {max(0, EVALS_PER_NUDGE - evals % EVALS_PER_NUDGE):,} eval")
    print(f"Totalt inspelade evalueringar:  {log['total_evals_recorded']:,}")
    print(f"Totalt nudges gjorda:           {len(log['nudges'])}")
    if log["nudges"]:
        last = log["nudges"][-1]
        print(f"Senaste nudge: {last['date']} slot{last['slot']} {last['param']} "
              f"{last['old_value']:.4f} → {last['new_value']:.4f} (Δ{last['delta']:+.4f})")


def cmd_record_training(team: str, evaluations: int):
    team_dir = TEAMS_DIR / team
    if not team_dir.exists():
        print(f"Fel: lag '{team}' hittades inte.")
        sys.exit(1)
    log = load_nudge_log(team_dir)
    log["total_evals_recorded"] += evaluations
    log["evals_since_last_nudge"] += evaluations
    save_nudge_log(team_dir, log)
    available = log["evals_since_last_nudge"] // EVALS_PER_NUDGE
    print(f"Registrerade {evaluations:,} evalueringar för {team}.")
    print(f"Tillgängliga nudges: {available}")


def cmd_nudge(team: str, slot: int, param: str, delta: float):
    team_dir = TEAMS_DIR / team
    if not team_dir.exists():
        print(f"Fel: lag '{team}' hittades inte.")
        sys.exit(1)

    log = load_nudge_log(team_dir)
    available = log["evals_since_last_nudge"] // EVALS_PER_NUDGE

    if available < 1:
        needed = EVALS_PER_NUDGE - log["evals_since_last_nudge"] % EVALS_PER_NUDGE
        print(f"❌ Ingen nudge tillgänglig. Kör {needed:,} fler evalueringar först.")
        print(f"   (Hittills sedan sista: {log['evals_since_last_nudge']:,} / {EVALS_PER_NUDGE:,})")
        sys.exit(1)

    if abs(delta) > MAX_DELTA:
        print(f"❌ Delta {delta:+.4f} överstiger max tillåtet (±{MAX_DELTA}).")
        sys.exit(1)

    if param not in PARAM_BOUNDS:
        print(f"❌ Okänd parameter '{param}'. Giltiga: {', '.join(PARAM_BOUNDS)}")
        sys.exit(1)

    if slot < 0 or slot > 4:
        print(f"❌ Slot måste vara 0–4.")
        sys.exit(1)

    # Ladda baseline
    baseline_path = team_dir / "baseline.json"
    with open(baseline_path) as f:
        baseline = json.load(f)

    player = baseline["playerParams"][slot]

    # Kolla om parametern är i decisions eller gk
    if param in player["decisions"]:
        old_value = player["decisions"][param]
        section = "decisions"
    elif player.get("gk") and param in player["gk"]:
        old_value = player["gk"][param]
        section = "gk"
    else:
        print(f"❌ Parameter '{param}' finns inte i slot {slot} ({SLOT_NAMES[slot]}).")
        sys.exit(1)

    lo, hi = PARAM_BOUNDS[param]
    new_value = max(lo, min(hi, old_value + delta))
    actual_delta = new_value - old_value

    if actual_delta == 0:
        print(f"⚠️  Värdet klampades — ingen förändring möjlig (redan vid gräns).")
        sys.exit(1)

    print(f"=== Nudge: {team} slot{slot} ({SLOT_NAMES[slot]}) ===")
    print(f"Parameter: {param}")
    print(f"Gammalt värde: {old_value:.6f}")
    print(f"Nytt värde:    {new_value:.6f}  (Δ{actual_delta:+.6f})")
    print(f"Bounds: [{lo}, {hi}]")

    confirm = input("Bekräfta nudge? (ja/nej): ").strip().lower()
    if confirm != "ja":
        print("Avbrutet.")
        sys.exit(0)

    # Applicera
    if section == "decisions":
        baseline["playerParams"][slot]["decisions"][param] = new_value
    else:
        baseline["playerParams"][slot]["gk"][param] = new_value

    baseline["description"] = f"{baseline.get('description', team)}, nudged slot{slot} {param}"

    with open(baseline_path, "w") as f:
        json.dump(baseline, f, indent=2)

    # Uppdatera log — förbruka en nudge
    log["evals_since_last_nudge"] -= EVALS_PER_NUDGE  # förbruka precis en nudge
    log["nudges"].append({
        "date": datetime.now().isoformat(),
        "slot": slot,
        "slot_name": SLOT_NAMES[slot],
        "param": param,
        "old_value": old_value,
        "new_value": new_value,
        "delta": actual_delta,
        "evals_before": log["evals_since_last_nudge"] + EVALS_PER_NUDGE,
    })
    save_nudge_log(team_dir, log)

    print(f"✅ Nudge applicerad. Evalueringar kvar tills nästa: {EVALS_PER_NUDGE:,}")


if __name__ == "__main__":
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(1)

    cmd = sys.argv[1]
    team = sys.argv[2]

    if cmd == "status":
        cmd_status(team)
    elif cmd == "record-training":
        if len(sys.argv) < 4:
            print("Fel: ange antal evalueringar. Ex: record-training nebula-rangers 200000")
            sys.exit(1)
        cmd_record_training(team, int(sys.argv[3]))
    elif cmd == "nudge":
        if len(sys.argv) < 6:
            print("Fel: nudge <team> <slot> <param> <delta>")
            sys.exit(1)
        cmd_nudge(team, int(sys.argv[3]), sys.argv[4], float(sys.argv[5]))
    else:
        print(f"Okänt kommando: {cmd}")
        print(__doc__)
        sys.exit(1)
