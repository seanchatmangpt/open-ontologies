#!/usr/bin/env bash
# Single source of truth for the default-feature repository verification matrix.
# Runs every admitted gate in declared order; first failure halts the run.
# The reserved `mcpp` feature is UNSUPPORTED until mcpp-core is published or
# vendored, so `--all-features` would overclaim the clean-checkout boundary.
set -euo pipefail

cd "$(dirname "$0")/../.."

step() {
  echo ""
  echo "── $1 ─────────────────────────────────────────────"
}

step "1/9: ggen standards admission"
python3 tools/verify_ggen_standards.py

step "2/9: cargo metadata --locked"
cargo metadata --locked --format-version 1 --no-deps >/dev/null

step "3/9: cargo build --locked --lib"
cargo build --locked --lib

step "4/9: cargo test --locked --lib"
cargo test --locked --lib

step "5/9: cargo test --locked --tests --no-fail-fast"
cargo test --locked --tests --no-fail-fast

step "6/9: Rust dead-parameter gate"
cargo test --locked --test dead_param_gate_test -- --test-threads=1

step "7/9: cargo clippy --locked --all-targets -- -D warnings"
cargo clippy --locked --all-targets -- -D warnings

step "8/9: baseline checks (ignore + test-count + ratchet-floor)"
bash .github/scripts/check-ignore-baseline.sh
bash .github/scripts/check-test-count.sh
bash .github/scripts/check-ratchet-floor.sh

step "9/9: verification matrix complete"
echo "✓ locked default-feature regression gates passed"
