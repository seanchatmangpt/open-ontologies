#!/usr/bin/env bash
# Prevent silent inflation of #[ignore] markers in tests/ + src/.
# Bare `#[ignore]` (no justification) is a hard fail.
# `#[ignore = "reason"]` is allowed but counted; total count must not exceed baseline.
set -euo pipefail

cd "$(dirname "$0")/../.."

BASELINE_FILE=".github/baselines/ignore-count.txt"
BASELINE="$(tr -d '[:space:]' < "$BASELINE_FILE")"

# All ignores (justified + bare)
TOTAL=$(grep -rE '^\s*#\[ignore' tests/ src/ 2>/dev/null | wc -l | tr -d ' ')
# Bare = #[ignore] without `= "..."`
BARE=$(grep -rEn '^\s*#\[ignore\]\s*$' tests/ src/ 2>/dev/null || true)

if [ -n "$BARE" ]; then
  echo "❌ check-ignore-baseline: bare \`#[ignore]\` (no justification) is forbidden:"
  echo "$BARE"
  echo "Use \`#[ignore = \"reason explaining why\"]\` instead."
  exit 1
fi

if [ "$TOTAL" -gt "$BASELINE" ]; then
  echo "❌ check-ignore-baseline: #[ignore] count $TOTAL exceeds baseline $BASELINE"
  echo "Either fix the disabled tests, or update $BASELINE_FILE with a PR justification."
  exit 1
fi

echo "✓ check-ignore-baseline: $TOTAL <= baseline $BASELINE (no bare ignores)"
