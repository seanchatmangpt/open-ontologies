#!/usr/bin/env bash
set -euo pipefail
SUPABASE_URL="${SUPABASE_URL:-http://127.0.0.1:54321}"
if ! curl -fsS "${SUPABASE_URL}/auth/v1/health" >/dev/null 2>&1; then
  echo "SKIP: Supabase local not running"
  exit 0
fi
HTTP_CODE=$(curl -o /dev/null -w "%{http_code}" -sS "${SUPABASE_URL}/storage/v1/status" 2>/dev/null || true)
if [ "${HTTP_CODE}" = "503" ] || [ "${HTTP_CODE}" = "000" ]; then
  echo "SKIP: Supabase Storage not enabled (storage-api excluded from start)"
  exit 0
fi
STATUS=$(curl -fsS "${SUPABASE_URL}/storage/v1/status" 2>/dev/null | grep -c '"version"' || true)
if [ "${STATUS}" -eq 0 ]; then
  echo "FAIL: Supabase Storage not responding" >&2
  exit 1
fi
echo "validate:storage PASS"
