#!/usr/bin/env python3
"""Collect training statistics across all policy versions and write data/training-history.json."""

import json
import os
from datetime import datetime, timezone

POLICIES_DIR = os.path.join(os.path.dirname(__file__), "..", "data", "policies")
OUTPUT_FILE = os.path.join(os.path.dirname(__file__), "..", "data", "training-history.json")
VERSIONS = ["v1", "v2", "v3", "v4", "v6"]


def collect_version_stats(version: str) -> dict:
    sessions_dir = os.path.join(POLICIES_DIR, version, "sessions")
    if not os.path.isdir(sessions_dir):
        return {"sessions": 0, "totalMatches": 0, "totalEpochs": 0, "totalTrainingMs": 0}

    session_count = 0
    total_matches = 0
    total_epochs = 0
    total_training_ms = 0

    for session_name in sorted(os.listdir(sessions_dir)):
        summary_path = os.path.join(sessions_dir, session_name, "summary.json")
        if not os.path.isfile(summary_path):
            continue

        with open(summary_path, "r") as f:
            try:
                summary = json.load(f)
            except json.JSONDecodeError:
                print(f"  WARNING: could not parse {summary_path}")
                continue

        session_count += 1
        history = summary.get("history", [])
        total_epochs += len(history)
        total_matches += sum(epoch.get("gamesRun", 0) for epoch in history)
        total_training_ms += summary.get("totalTrainingElapsedMs", 0)

    return {
        "sessions": session_count,
        "totalMatches": total_matches,
        "totalEpochs": total_epochs,
        "totalTrainingMs": total_training_ms,
    }


def main():
    result = {
        "generatedAt": datetime.now(timezone.utc).isoformat(),
        "note": "Archived before v1-v4 data removal. totalMatches = actual games run (respects early stopping).",
        "versions": {},
    }

    for version in VERSIONS:
        stats = collect_version_stats(version)
        result["versions"][version] = stats
        print(
            f"  {version}: {stats['sessions']} sessions, "
            f"{stats['totalMatches']:,} matches, "
            f"{stats['totalEpochs']} epochs, "
            f"{stats['totalTrainingMs'] / 1000:.0f}s training time"
        )

    with open(OUTPUT_FILE, "w") as f:
        json.dump(result, f, indent=2)
    print(f"\nWrote {OUTPUT_FILE}")


if __name__ == "__main__":
    main()
