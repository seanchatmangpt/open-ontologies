#!/bin/bash
# SessionStart hook - Inject workspace state

set -euo pipefail

WORKSPACE_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "")"
if [[ -z "$WORKSPACE_ROOT" ]]; then
  echo "ERROR: Not inside a git repository" >&2
  exit 1
fi

cd "$WORKSPACE_ROOT"

# 1. Check binary exists
BINARY="target/release/open-ontologies"
if [ ! -f "$BINARY" ]; then
  echo "⚠️  open-ontologies binary missing. Build with: cargo build --release" >&2
fi

# 2. Check crosswalks.parquet exists (clinical feature)
if [ ! -f "data/crosswalks.parquet" ]; then
  echo "⚠️  crosswalks.parquet missing (clinical feature disabled)" >&2
fi

# 3. Current branch
BRANCH="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")"

# 4. Uncommitted changes
UNCOMMITTED="$(git status --porcelain 2>/dev/null | wc -l | tr -d ' ')"
UNCOMMITTED="${UNCOMMITTED:-0}"

# 5. Compile state
COMPILE_OUTPUT="$(make check --dry-run 2>&1 | tail -3 || true)"
if echo "$COMPILE_OUTPUT" | grep -qE "error"; then
  COMPILE_STATE="ERRORS"
elif echo "$COMPILE_OUTPUT" | grep -qE "warning"; then
  COMPILE_STATE="WARNINGS"
else
  COMPILE_STATE="CLEAN"
fi

# Output to stderr
echo "Branch: ${BRANCH} | Changes: ${UNCOMMITTED} uncommitted | Compile: ${COMPILE_STATE}" >&2
echo "Evidence tier required: PROVEN (make adversarial + SHACL validation)" >&2

exit 0
