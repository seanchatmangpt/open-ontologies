#!/usr/bin/env bash
# Single source of truth for the ggen verification matrix.
# Runs every gate in declared order; first failure halts the run.
set -euo pipefail

cd "$(dirname "$0")/../.."
export CARGO_INCREMENTAL=0
export RUST_BACKTRACE=1

started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

step() {
  echo ""
  echo "── $1 ─────────────────────────────────────────────"
}

step "1/10: ggen source-law replay and constitutional admission"
python3 tools/verify_ggen_ci.py --output target/verifier/ggen-ci-report.json
python3 tools/verify_ggen_standards.py
python3 tools/activate_zoela_rules.py --check

step "2/10: cargo build --lib --all-features"
cargo build --locked --lib --all-features

step "3/10: cargo test --lib --all-features"
cargo test --locked --lib --all-features

step "4/10: cargo test --tests --no-fail-fast"
cargo test --locked --tests --no-fail-fast

step "5/10: dead-parameter AST gate"
cargo test --locked --test dead_param_gate_test

step "6/10: cargo clippy --all-targets --all-features -- -D warnings"
cargo clippy --locked --all-targets --all-features -- -D warnings

step "7/10: baseline checks (ignore + test-count + ratchet-floor)"
bash .github/scripts/check-ignore-baseline.sh
bash .github/scripts/check-test-count.sh
bash .github/scripts/check-ratchet-floor.sh

step "8/10: adversarial refusal suite"
make adversarial

step "9/10: Cell 8 certification"
make cell8-certify

step "10/10: machine-readable verifier receipt"
mkdir -p target/verifier
cat > target/verifier/verification-matrix-report.json <<EOF
{
  "schema": "chatmangpt.verifier-report/v1",
  "started_at": "$started_at",
  "finished_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "standing": "ALIVE",
  "gates": [
    "ggen_ci_replay",
    "ggen_standards",
    "zoela_source_law",
    "compile_all_features",
    "unit_all_features",
    "integration_e2e",
    "dead_parameter_ast",
    "clippy_deny_warnings",
    "baseline_ratchets",
    "adversarial_refusals",
    "cell8_certification"
  ]
}
EOF

echo "✓ ggen verification matrix ALIVE"