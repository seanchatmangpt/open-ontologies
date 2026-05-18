#!/usr/bin/env bash
set -euo pipefail
SUPABASE_URL="${SUPABASE_URL:-http://127.0.0.1:54321}"
DB_URL="${DATABASE_URL:-postgresql://postgres:postgres@127.0.0.1:54322/postgres}"
if ! command -v psql >/dev/null 2>&1 && ! command -v supabase >/dev/null 2>&1; then
  echo "SKIP: psql/supabase CLI not available"
  exit 0
fi
# Kong 2.8.1 returns 404 on /health; use /auth/v1/health which returns 200
if ! curl -fsS "${SUPABASE_URL}/auth/v1/health" >/dev/null 2>&1; then
  echo "SKIP: Supabase local not running"
  exit 0
fi
# Verify at least one RLS policy exists (hand-authored in supabase/migrations/20260518000003_zoela_rls.sql)
# Use psql directly — supabase db execute is not available in CLI v2.54.11
RLS_COUNT=$(PGPASSWORD=postgres psql -h 127.0.0.1 -p 54322 -U postgres -d postgres -At \
  -c "SELECT count(*) FROM pg_policies WHERE schemaname = 'public'" 2>/dev/null || echo "0")
if [ "${RLS_COUNT:-0}" -eq 0 ]; then
  echo "FAIL: No RLS policies found — run supabase db reset to apply migrations" >&2
  exit 1
fi
echo "validate:rls PASS (${RLS_COUNT} policies active)"
