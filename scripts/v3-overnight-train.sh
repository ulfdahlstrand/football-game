#!/bin/bash
# Runs v3 training sessions back-to-back until 05:00 (next morning if it's
# currently between 05:00 and 23:59, or today if already past midnight).
# Each session: 100 epochs × 100 000 games (~30 min).
# Picks the next free session-N number automatically.

set -u

DIR=/Users/ulfdahlstrand/Projects/Code/Private/Spel/football-game
ENGINE=$DIR/training-engine/target/release/training-engine
SESSIONS_DIR=$DIR/data/policies/v3/sessions
LOG_DIR=/tmp

# Wait for any running training-engine process (e.g. v3 session-2 still going)
while pgrep -x training-engine >/dev/null 2>&1; do
  echo "$(date '+%H:%M:%S'): waiting for current training to finish..."
  sleep 60
done

# Pick next free session number
LAST=$(ls -d "$SESSIONS_DIR"/session-* 2>/dev/null \
  | sed 's/.*session-//' | sort -n | tail -1)
N=$(( ${LAST:-0} + 1 ))

# Compute stop timestamp = next occurrence of 05:00
HOUR=$(date +%H)
if [ "$HOUR" -ge 5 ]; then
  STOP_DATE=$(date -v+1d +%Y-%m-%d)   # past 05:00 → tomorrow's 05:00
else
  STOP_DATE=$(date +%Y-%m-%d)         # before 05:00 → today's 05:00
fi
STOP_TS=$(date -j -f "%Y-%m-%d %H:%M" "$STOP_DATE 05:00" +%s)

echo "$(date '+%Y-%m-%d %H:%M:%S'): auto-training until $STOP_DATE 05:00, starting from session-$N"

while [ "$(date +%s)" -lt "$STOP_TS" ]; do
  REMAINING=$(( STOP_TS - $(date +%s) ))
  # Need at least ~30 min headroom for a full session
  if [ "$REMAINING" -lt 1800 ]; then
    echo "$(date '+%H:%M:%S'): less than 30 min remaining, stopping"
    break
  fi

  echo "$(date '+%H:%M:%S'): starting session-$N (remaining ~$((REMAINING / 60)) min)"
  "$ENGINE" --v3 100 100000 "session-$N" > "$LOG_DIR/v3-session-$N.log" 2>&1
  echo "$(date '+%H:%M:%S'): session-$N done"
  N=$(( N + 1 ))
done

echo "$(date '+%Y-%m-%d %H:%M:%S'): auto-training stopped after session-$((N-1))"
