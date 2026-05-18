#!/usr/bin/env bash
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STAGES="${REPO_ROOT}/packages/routes/connectGroupStages.ts"
WORK_ORDERS="${REPO_ROOT}/packages/routes/connectGroupWorkOrders.ts"
if [ ! -f "${STAGES}" ]; then
  echo "FAIL: packages/routes/connectGroupStages.ts missing" >&2; exit 1
fi
if [ ! -f "${WORK_ORDERS}" ]; then
  echo "FAIL: packages/routes/connectGroupWorkOrders.ts missing" >&2; exit 1
fi
# Verify 8 stages are present (each stage has a "code:" field)
STAGE_COUNT=$(grep -c '^\s*code:' "${STAGES}" || true)
if [ "${STAGE_COUNT}" -lt 8 ]; then
  echo "FAIL: expected 8 CG stages, found ${STAGE_COUNT}" >&2; exit 1
fi
# Verify 7 work orders are present (each work order has a "code:" field)
WO_COUNT=$(grep -c '^\s*code:' "${WORK_ORDERS}" || true)
if [ "${WO_COUNT}" -lt 7 ]; then
  echo "FAIL: expected 7 work orders, found ${WO_COUNT}" >&2; exit 1
fi
echo "validate:autonomics PASS (${STAGE_COUNT} stages, ${WO_COUNT} work orders)"
