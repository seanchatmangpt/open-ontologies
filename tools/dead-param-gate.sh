#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

if command -v rg >/dev/null 2>&1; then
  if rg -n 'let\s+_\s*=\s*(?:&\s*)?input\.' src/server.rs src/cmds 2>/dev/null; then
    echo 'dead-param-gate: MCP input parameter is explicitly discarded' >&2
    exit 1
  fi
fi

cargo test --locked --test adversarial_jtbd_test
echo 'dead-param-gate: no explicit input discard; adversarial parameter tests passed'
