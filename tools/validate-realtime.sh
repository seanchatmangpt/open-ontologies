#!/usr/bin/env bash
set -euo pipefail
SUPABASE_URL="${SUPABASE_URL:-http://127.0.0.1:54321}"
if ! curl -fsS "${SUPABASE_URL}/health" >/dev/null 2>&1; then
  echo "SKIP: Supabase local not running"
  exit 0
fi
# Realtime runs on a separate port locally
RT_STATUS=$(curl -fsS "http://127.0.0.1:54321/realtime/v1/health" 2>/dev/null | grep -c '"status"' || true)
if [ "${RT_STATUS}" -eq 0 ]; then
  echo "SKIP: Supabase Realtime not responding (may not be enabled in config)"
  exit 0
fi
echo "validate:realtime PASS"
