#!/usr/bin/env bash
# Single source of truth for the regression-gates verification matrix.
# Runs every gate in declared order; first failure halts the run.
set -euo pipefail

cd "$(dirname "$0")/../.."

step() {
  echo ""
  echo "── $1 ─────────────────────────────────────────────"
}

step "1/7: cargo build --lib"
cargo build --locked --lib --all-features

step "2/7: cargo test --lib"
cargo test --locked --lib --all-features

step "3/7: cargo test --tests --no-fail-fast"
cargo test --locked --tests --no-fail-fast

step "4/7: dead-parameter gate"
cargo test --locked --test dead_param_gate_test

step "5/7: cargo clippy --all-targets -- -D warnings"
cargo clippy --locked --all-targets --all-features -- -D warnings

step "6/7: baseline checks (ignore + test-count + ratchet-floor)"
bash .github/scripts/check-ignore-baseline.sh
bash .github/scripts/check-test-count.sh
bash .github/scripts/check-ratchet-floor.sh

step "7/7: verification matrix complete"
echo "✓ all regression gates passed"
