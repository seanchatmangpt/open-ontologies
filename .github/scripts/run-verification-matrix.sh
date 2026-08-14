#!/usr/bin/env bash
# Single source of truth for the regression-gates verification matrix.
# Runs every gate in declared order; first failure halts the run.
set -euo pipefail

cd "$(dirname "$0")/../.."

step() {
  echo ""
  echo "── $1 ─────────────────────────────────────────────"
}

step "1/8: cargo build --lib"
cargo build --lib --all-features

step "2/8: cargo test --lib"
cargo test --lib --all-features

step "3/8: cargo test --tests --no-fail-fast"
cargo test --tests --no-fail-fast

step "4/8: dead-param-gate"
bash tools/dead-param-gate.sh

step "5/8: cargo clippy --all-targets -- -D warnings"
cargo clippy --all-targets --all-features -- -D warnings

step "6/8: baseline checks (ignore + test-count + ratchet-floor)"
bash .github/scripts/check-ignore-baseline.sh
bash .github/scripts/check-test-count.sh
bash .github/scripts/check-ratchet-floor.sh

step "7/8: DFLSS/DMEDI definition of done"
bash .github/scripts/dflss-dod.sh

step "8/8: verification matrix complete"
echo "✓ all regression gates passed"
