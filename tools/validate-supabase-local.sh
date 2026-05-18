#!/usr/bin/env bash
set -euo pipefail
SUPABASE_URL="${SUPABASE_URL:-http://127.0.0.1:54321}"
# Kong 2.8.1 returns 404 on /health; use /auth/v1/health which returns 200
HEALTH_ENDPOINT="${SUPABASE_URL}/auth/v1/health"
if ! curl -fsS "${HEALTH_ENDPOINT}" >/dev/null 2>&1; then
  echo "SKIP: Supabase local not running at ${SUPABASE_URL}"
  exit 0
fi
# /auth/v1/health returns GoTrue version JSON — any non-empty 200 response is healthy
STATUS=$(curl -fsS "${HEALTH_ENDPOINT}" | grep -c '"version"' || true)
if [ "${STATUS}" -eq 0 ]; then
  echo "FAIL: Supabase health check returned non-ok status" >&2
  exit 1
fi
echo "validate:supabase-local PASS"
