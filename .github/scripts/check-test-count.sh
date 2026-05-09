#!/usr/bin/env bash
# Prevent silent regression in test count. A drop is permitted only with the
# `tests-removed` PR label set in $GITHUB_PR_LABELS.
set -euo pipefail

cd "$(dirname "$0")/../.."

BASELINE_FILE=".github/baselines/test-count.txt"
BASELINE="$(tr -d '[:space:]' < "$BASELINE_FILE")"
CURRENT=$(grep -rE '#\[(tokio::)?test\]' tests/ src/ 2>/dev/null | wc -l | tr -d ' ')

if [ "$CURRENT" -lt "$BASELINE" ]; then
  if echo "${GITHUB_PR_LABELS:-}" | grep -q 'tests-removed'; then
    echo "⚠ check-test-count: $CURRENT < baseline $BASELINE — allowed by 'tests-removed' label"
    exit 0
  fi
  echo "❌ check-test-count: $CURRENT < baseline $BASELINE — tests deleted without 'tests-removed' label"
  exit 1
fi

echo "✓ check-test-count: $CURRENT >= baseline $BASELINE"
