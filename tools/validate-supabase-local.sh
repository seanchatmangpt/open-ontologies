#!/usr/bin/env bash
set -euo pipefail
SUPABASE_URL="${SUPABASE_URL:-http://127.0.0.1:54321}"
if ! curl -fsS "${SUPABASE_URL}/health" >/dev/null 2>&1; then
  echo "SKIP: Supabase local not running at ${SUPABASE_URL}"
  exit 0
fi
STATUS=$(curl -fsS "${SUPABASE_URL}/health" | grep -c '"status":"ok"' || true)
if [ "${STATUS}" -eq 0 ]; then
  echo "FAIL: Supabase health check returned non-ok status" >&2
  exit 1
fi
echo "validate:supabase-local PASS"
