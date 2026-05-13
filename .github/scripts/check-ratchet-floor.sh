#!/usr/bin/env bash
# Floor on ratchet_red_team test count. Tests in this file may only grow.
set -euo pipefail

cd "$(dirname "$0")/../.."

BASELINE_FILE=".github/baselines/ratchet-red-team-count.txt"
BASELINE="$(tr -d '[:space:]' < "$BASELINE_FILE")"
CURRENT=$(grep -cE '^#\[test\]' tests/ratchet_red_team.rs || echo 0)

if [ "$CURRENT" -lt "$BASELINE" ]; then
  echo "❌ check-ratchet-floor: ratchet_red_team has $CURRENT tests, baseline $BASELINE — ratchet may only grow"
  exit 1
fi

echo "✓ check-ratchet-floor: $CURRENT >= baseline $BASELINE"
