#!/usr/bin/env bash
# Compatibility entrypoint only. The Rust AST test is the sole gate authority.
set -euo pipefail
cd "$(dirname "$0")/.."
exec cargo test --locked --test dead_param_gate_test
