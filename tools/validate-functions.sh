#!/usr/bin/env bash
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FAIL=0
for fn in route-gate autonomic-executor receipt-verify ocel-export; do
  if [ ! -f "${REPO_ROOT}/supabase/functions/${fn}/index.ts" ]; then
    echo "FAIL: missing supabase/functions/${fn}/index.ts" >&2
    FAIL=1
  fi
done
if [ "${FAIL}" -eq 1 ]; then
  exit 1
fi
echo "validate:functions PASS — all 4 edge functions present"
