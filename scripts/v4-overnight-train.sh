#!/bin/bash
# Runs v4 training sessions back-to-back until 06:00 (next morning if it's
# currently between 06:00 and 23:59, or today if already past midnight).
# Each session: 100 epochs × 100 000 games (~70 min observed for v4).
# Picks the next free session-N number automatically.

set -u

DIR=/Users/ulfdahlstrand/Projects/Code/Private/Spel/football-game
ENGINE=$DIR/training-engine/target/release/training-engine
SESSIONS_DIR=$DIR/data/policies/v4/sessions
LOG_DIR=/tmp

# Wait for any running training-engine process
while pgrep -x training-engine >/dev/null 2>&1; do
  echo "$(date '+%H:%M:%S'): waiting for current training to finish..."
  sleep 60
done

# Pick next free session number (only "session-N" pattern, not slot-* etc)
LAST=$(ls -d "$SESSIONS_DIR"/session-* 2>/dev/null \
  | grep -E 'session-[0-9]+$' \
  | sed 's/.*session-//' | sort -n | tail -1)
N=$(( ${LAST:-0} + 1 ))

# Compute stop timestamp = next occurrence of 06:00
HOUR=$(date +%H)
if [ "$HOUR" -ge 6 ]; then
  STOP_DATE=$(date -v+1d +%Y-%m-%d)
else
  STOP_DATE=$(date +%Y-%m-%d)
fi
STOP_TS=$(date -j -f "%Y-%m-%d %H:%M" "$STOP_DATE 06:00" +%s)

echo "$(date '+%Y-%m-%d %H:%M:%S'): v4 auto-training until $STOP_DATE 06:00, starting from session-$N"

while [ "$(date +%s)" -lt "$STOP_TS" ]; do
  REMAINING=$(( STOP_TS - $(date +%s) ))
  # Need at least ~70 min headroom for a v4 session
  if [ "$REMAINING" -lt 4200 ]; then
    echo "$(date '+%H:%M:%S'): less than 70 min remaining ($((REMAINING/60)) min), stopping"
    break
  fi

  echo "$(date '+%H:%M:%S'): starting session-$N (remaining ~$((REMAINING / 60)) min)"
  "$ENGINE" --v4 100 100000 "session-$N" > "$LOG_DIR/v4-session-$N.log" 2>&1
  echo "$(date '+%H:%M:%S'): session-$N done"
  N=$(( N + 1 ))
done

echo "$(date '+%Y-%m-%d %H:%M:%S'): v4 auto-training stopped after session-$((N-1))"
