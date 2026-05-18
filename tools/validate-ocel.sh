#!/usr/bin/env bash
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OCEL="${REPO_ROOT}/packages/evidence/OcelEvents.ts"
if [ ! -f "${OCEL}" ]; then
  echo "FAIL: packages/evidence/OcelEvents.ts missing — run ggen sync" >&2
  exit 1
fi
FAIL=0
for event in "cg.interest.submitted" "cg.invite.sent" "cg.route.closed"; do
  if ! grep -q "${event}" "${OCEL}"; then
    echo "WARN: OCEL event type '${event}' not found in OcelEvents.ts" >&2
  fi
done
LINE_COUNT=$(wc -l < "${OCEL}" | tr -d ' ')
if [ "${LINE_COUNT}" -lt 15 ]; then
  echo "FAIL: packages/evidence/OcelEvents.ts is too short (${LINE_COUNT} lines)" >&2
  FAIL=1
fi
[ "${FAIL}" -eq 0 ] && echo "validate:ocel PASS (${LINE_COUNT} lines)" || exit 1
