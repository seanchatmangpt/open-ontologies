#!/usr/bin/env bash
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RECEIPTS="${REPO_ROOT}/packages/evidence/receipts.ts"
if [ ! -f "${RECEIPTS}" ]; then
  echo "FAIL: packages/evidence/receipts.ts missing — run ggen sync" >&2
  exit 1
fi
LINE_COUNT=$(wc -l < "${RECEIPTS}" | tr -d ' ')
if [ "${LINE_COUNT}" -lt 10 ]; then
  echo "FAIL: packages/evidence/receipts.ts is too short (${LINE_COUNT} lines) — ggen may have emitted empty output" >&2
  exit 1
fi
echo "validate:receipts PASS (${LINE_COUNT} lines)"
