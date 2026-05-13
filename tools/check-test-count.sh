#!/usr/bin/env bash
# Adversarial gate: detect drift between the test-count claim in README.md and
# the actual number of `#[test]` / `#[tokio::test]` functions in tests/.
#
# Material-record doctrine (TPS §20): the README is a load-bearing claim
# surface. A claim like "Test totals: 456 #[test] functions" that drifts from
# reality by even one number is a theatrical-truth defect — it asserts a
# specific count without producing proof. This gate produces the proof on
# every `make adversarial` run and fails loudly when reality drifts from the
# claim.
#
# Regex policy: we count only top-of-line `#[test]` and `#[tokio::test]`
# attributes (with optional leading whitespace). The looser regex
# (`#[test]|#[tokio::test]` anywhere on the line) over-counts because it picks
# up strings inside macro_rules!, comments, and docstrings.
set -euo pipefail

cd "$(dirname "$0")/.."

REGEX='^\s*#\[(tokio::)?test'
ACTUAL=$(grep -rE "$REGEX" tests/ | wc -l | tr -d ' ')

# Pull the claimed count from the README. The line we are validating is:
#   **Test totals:** 470 `#[test]` functions across `tests/` ...
CLAIMED=$(grep -oE 'Test totals:\*\* [0-9]+' README.md | grep -oE '[0-9]+' || echo "0")

if [ -z "$CLAIMED" ] || [ "$CLAIMED" = "0" ]; then
  echo "FAIL: Could not find 'Test totals:** N' claim in README.md" >&2
  exit 1
fi

if [ "$ACTUAL" != "$CLAIMED" ]; then
  echo "FAIL: README test-count drift detected" >&2
  echo "  README.md claims:  $CLAIMED" >&2
  echo "  tests/ actually:   $ACTUAL"  >&2
  echo "  regex:             $REGEX"   >&2
  echo "" >&2
  echo "Fix: update the 'Test totals:** N' line in README.md to $ACTUAL." >&2
  exit 1
fi

echo "OK: README test count ($CLAIMED) matches tests/ ($ACTUAL #[test]/#[tokio::test] functions)"
