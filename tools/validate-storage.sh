#!/usr/bin/env bash
set -euo pipefail
SUPABASE_URL="${SUPABASE_URL:-http://127.0.0.1:54321}"
if ! curl -fsS "${SUPABASE_URL}/health" >/dev/null 2>&1; then
  echo "SKIP: Supabase local not running"
  exit 0
fi
STATUS=$(curl -fsS "${SUPABASE_URL}/storage/v1/status" 2>/dev/null | grep -c '"version"' || true)
if [ "${STATUS}" -eq 0 ]; then
  echo "FAIL: Supabase Storage not responding" >&2
  exit 1
fi
echo "validate:storage PASS"
