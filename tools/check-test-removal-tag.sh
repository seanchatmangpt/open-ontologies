#!/usr/bin/env bash
# R5 WA — §22 ratchet hole #10: dual-count manipulation guard.
#
# A coding agent could regress a load-bearing test while satisfying the
# README test-count check by ADDING two new tests in the same commit.
# The net delta is +1 test, the count check passes, but a regression
# slipped through.
#
# This gate counts every removed `#[test]` / `#[tokio::test]` line
# between merge-base and HEAD (or BASE..HEAD when BASE is set
# explicitly). Each removed test must be paired with a `test-removal:
# <reason>` tag in the commit messages on the branch.
#
# Material-record doctrine (TPS §20): the act of removing a test is a
# load-bearing claim. A removal without a justification tag is a
# theatrical-truth defect — a regression hidden by accounting tricks.
#
# Local usage: BASE=origin/main bash tools/check-test-removal-tag.sh
# CI usage:    BASE="$GITHUB_BASE_REF" bash tools/check-test-removal-tag.sh
#
# Skip behaviour: if no merge-base / no remote / no commits, exit 0
# (treat as no-op rather than block CI when the repo is in a transient
# state — e.g. fresh clone in a sandbox without `origin/main`).
set -euo pipefail

cd "$(dirname "$0")/.."

# Resolve BASE: explicit env var, then merge-base with origin/main, then
# bail gracefully.
BASE="${BASE:-}"
if [ -z "$BASE" ]; then
  for ref in origin/main origin/master main master; do
    if git rev-parse --verify --quiet "$ref" >/dev/null 2>&1; then
      BASE="$(git merge-base HEAD "$ref" 2>/dev/null || true)"
      if [ -n "$BASE" ]; then
        break
      fi
    fi
  done
fi

if [ -z "$BASE" ]; then
  # No baseline available — skip rather than block. This is the
  # fail-open path; it's annotated, deliberate, and tolerable because
  # the caller can always pass BASE explicitly when stricter behaviour
  # is needed.
  echo "check-test-removal-tag: no BASE/origin merge-base resolvable; skipping"
  exit 0
fi

# Count removed `#[test]` / `#[tokio::test]` lines in the diff. We use
# unified=0 so context lines don't contaminate the count, and we filter
# for added-by-removal markers (`^-` prefix without `^---`).
REGEX='^-[[:space:]]*#\[(tokio::)?test'
REMOVED=$(git diff --unified=0 "$BASE..HEAD" -- 'tests/' \
  | grep -cE "$REGEX" \
  || true)
REMOVED=$(echo "$REMOVED" | tr -d ' \n')

if [ "$REMOVED" = "0" ]; then
  echo "check-test-removal-tag: no test removals between $BASE and HEAD"
  exit 0
fi

# Count `test-removal:` justification tags in commit messages on the
# branch. One tag per removal is required.
TAGS=$(git log --format=%B "$BASE..HEAD" \
  | grep -cE '^test-removal:[[:space:]]+\S' \
  || true)

if [ "$TAGS" -lt "$REMOVED" ]; then
  echo "FAIL: $REMOVED test(s) removed but only $TAGS 'test-removal:' tag(s) found in commit messages" >&2
  echo "" >&2
  echo "Each removed #[test] / #[tokio::test] must be justified with a commit-message line:" >&2
  echo "  test-removal: <reason>" >&2
  echo "" >&2
  echo "Example:" >&2
  echo "  test-removal: replaced by stronger AST-based test in tests/round5_ast_red_team.rs" >&2
  exit 1
fi

echo "check-test-removal-tag: OK ($REMOVED removed, $TAGS tag(s) present)"
exit 0
