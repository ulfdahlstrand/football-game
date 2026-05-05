#!/usr/bin/env bash
set -euo pipefail

REPO="/Users/ulfdahlstrand/Projects/Code/Private/Spel/football-game"
BIN="$REPO/training-engine/target/release/training-engine"
LOG="/tmp/overnight-train.log"
POP_PID=40478
GLACIER_PID=40645
AURORA_LOOP_UNTIL="06:00"

log() { echo "[$(date '+%H:%M:%S')] $*" | tee -a "$LOG"; }

log "=== OVERNIGHT ORCHESTRATOR START ==="
log "Waiting for PID $POP_PID (aurora/granite/phoenix/tempest)..."
log "Waiting for PID $GLACIER_PID (glacier-fc)..."

# Wait for both already-running processes
wait $POP_PID 2>/dev/null && log "PID $POP_PID done (population 0-3)" || log "PID $POP_PID already gone or failed (continuing)"
wait $GLACIER_PID 2>/dev/null && log "PID $GLACIER_PID done (glacier-fc)" || log "PID $GLACIER_PID already gone or failed (continuing)"

# STEP 3 — nebula-rangers: fresh population train (index 9)
log "=== STEP 3a: nebula-rangers full train (population index 9) ==="
"$BIN" --v6-population 1 --full --skip 9 2>&1 | tee -a "$LOG"
log "Step 3a done."

log "=== STEP 3b: nebula-rangers extra full pass ==="
"$BIN" --v6-team-train nebula-rangers --full 2>&1 | tee -a "$LOG"
log "Step 3b done."

# STEP 4 — aurora-fc loop until AURORA_LOOP_UNTIL
log "=== STEP 4: aurora-fc loop until $AURORA_LOOP_UNTIL ==="
while [[ "$(date +%H:%M)" < "$AURORA_LOOP_UNTIL" ]]; do
    log "--- aurora-fc pass start ---"
    "$BIN" --v6-team-train aurora-fc --full 2>&1 | tee -a "$LOG"
    log "--- aurora-fc pass done ---"
done
log "Aurora loop finished (time >= $AURORA_LOOP_UNTIL)."

# STEP 5 — Morning wrap-up
log "=== STEP 5a: Regenerate all SVGs ==="
"$BIN" --v6-team-svgs 2>&1 | tee -a "$LOG"

log "=== STEP 5b: Tournament 1000 games ==="
"$BIN" --v6-tournament 1000 2>&1 | tee -a "$LOG"

log "=== ALL DONE. Check $LOG for full output. ==="
