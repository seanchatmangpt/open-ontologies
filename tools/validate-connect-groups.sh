#!/usr/bin/env bash
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FAIL=0

# Check TTL source
if [ ! -f "${REPO_ROOT}/ontology/zoela/connect-group-routes.ttl" ]; then
  echo "FAIL: ontology/zoela/connect-group-routes.ttl missing" >&2; FAIL=1
fi
if [ ! -f "${REPO_ROOT}/ontology/zoela/connect-group-schedules.ttl" ]; then
  echo "FAIL: ontology/zoela/connect-group-schedules.ttl missing" >&2; FAIL=1
fi
if [ ! -f "${REPO_ROOT}/ontology/zoela/connect-group-capacity.ttl" ]; then
  echo "FAIL: ontology/zoela/connect-group-capacity.ttl missing" >&2; FAIL=1
fi

# Check generated artifacts
for f in \
  "packages/routes/connectGroupStages.ts" \
  "packages/routes/connectGroupWorkOrders.ts" \
  "packages/routes/connectGroupAdmin.ts" \
  "packages/forms/connectGroupInterestForm.tsx" \
  "supabase/seeds/connect_groups.sql"; do
  if [ ! -f "${REPO_ROOT}/${f}" ]; then
    echo "FAIL: missing ${f}" >&2; FAIL=1
  fi
done

# Check seed data has both persons
if [ -f "${REPO_ROOT}/supabase/seeds/connect_groups.sql" ]; then
  if ! grep -q "WithConsent" "${REPO_ROOT}/supabase/seeds/connect_groups.sql"; then
    echo "FAIL: seed missing person_with_consent" >&2; FAIL=1
  fi
  if ! grep -q "NoConsent" "${REPO_ROOT}/supabase/seeds/connect_groups.sql"; then
    echo "FAIL: seed missing person_without_consent" >&2; FAIL=1
  fi
fi

[ "${FAIL}" -eq 0 ] && echo "validate:connect-groups PASS" || exit 1
