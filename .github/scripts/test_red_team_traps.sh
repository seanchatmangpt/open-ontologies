#!/usr/bin/env bash
# Adversarial proof harness for the regression-gates verification matrix.
#
# This script does NOT mutate the working tree at CI time. It is a manual
# verification harness: each `attack_N` function describes one of the five
# bypass classes that Round-2's audit found, and shows the typed error each
# attack must trigger when the matrix runs. To verify, copy the function body
# into a temporary git worktree, apply the mutation, and run
# `bash .github/scripts/run-verification-matrix.sh` — the script must exit
# non-zero with the documented signature.
#
# Attack table:
#   1. let-method-discard       → dead-param-gate (rule: GATE_LET / GENERIC_DISCARD)
#   2. bare-method-discard      → dead-param-gate (rule: GATE_BARE)
#   3. ignore-inflation         → check-ignore-baseline.sh (TOTAL > BASELINE)
#   4. bare-ignore-no-justify   → check-ignore-baseline.sh (BARE non-empty)
#   5. test-count-regression    → check-test-count.sh (CURRENT < BASELINE)
set -euo pipefail

cat <<'EOF'
============================================================
  red-team trap harness — 5 attacks × matrix catch points
============================================================

Each attack is documented but NOT applied. To run live:

  git worktree add /tmp/rt-attack HEAD
  cd /tmp/rt-attack
  # apply mutation from attack_N below
  bash .github/scripts/run-verification-matrix.sh
  # expect: non-zero exit + the documented error string

EOF

attack_1_let_method_discard() {
  cat <<'ATTACK'
[ATTACK 1] let-method-discard
  Mutation:
    Add to any src/*.rs (not src/cmds/) inside a method body:
        let _ = self.evaluate_admission(op, scope, kind, bytes);
  Expected catch:
    tools/dead-param-gate.sh exits 1 with
    "❌ Gate-fn Result discarded via `let _ = self.<gate>(...)`:"
ATTACK
}

attack_2_bare_method_discard() {
  cat <<'ATTACK'
[ATTACK 2] bare-method-discard
  Mutation:
    Insert at the start of a line in src/*.rs:
        _ = self.persist_receipt(receipt);
  Expected catch:
    tools/dead-param-gate.sh exits 1 with
    "❌ Gate-fn Result discarded via bare `_ = self.<gate>(...)`:"
ATTACK
}

attack_3_ignore_inflation() {
  cat <<'ATTACK'
[ATTACK 3] ignore-inflation
  Mutation:
    Add `#[ignore = "flaky"]` above two new tests in tests/*.rs without
    bumping .github/baselines/ignore-count.txt.
  Expected catch:
    .github/scripts/check-ignore-baseline.sh exits 1 with
    "❌ check-ignore-baseline: #[ignore] count N exceeds baseline 1"
ATTACK
}

attack_4_bare_ignore_no_justify() {
  cat <<'ATTACK'
[ATTACK 4] bare-ignore-no-justification
  Mutation:
    Replace any `#[ignore = "..."]` with a bare `#[ignore]`.
  Expected catch:
    .github/scripts/check-ignore-baseline.sh exits 1 with
    "❌ check-ignore-baseline: bare `#[ignore]` (no justification) is forbidden:"
ATTACK
}

attack_5_test_count_regression() {
  cat <<'ATTACK'
[ATTACK 5] test-count-regression
  Mutation:
    Delete a #[test] / #[tokio::test] function from tests/ without setting
    GITHUB_PR_LABELS=tests-removed.
  Expected catch:
    .github/scripts/check-test-count.sh exits 1 with
    "❌ check-test-count: N < baseline 572 — tests deleted without 'tests-removed' label"
ATTACK
}

attack_1_let_method_discard
echo
attack_2_bare_method_discard
echo
attack_3_ignore_inflation
echo
attack_4_bare_ignore_no_justify
echo
attack_5_test_count_regression
echo
echo "✓ harness documentation printed; tree was not modified"
