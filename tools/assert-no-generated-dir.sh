#!/usr/bin/env bash
# assert-no-generated-dir.sh — fails if any generated/ staging directory exists
# CodeManufactory rule: ggen emits DIRECTLY to canonical runtime paths.
# generated/, app/generated/, supabase/generated/, packages/generated/, .generated/ are forbidden.
set -euo pipefail

FORBIDDEN=(
  "generated"
  "app/generated"
  "supabase/generated"
  "packages/generated"
  ".generated"
)

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FAIL=0

for dir in "${FORBIDDEN[@]}"; do
  if [ -d "${REPO_ROOT}/${dir}" ]; then
    echo "FAIL: forbidden staging directory exists: ${dir}" >&2
    FAIL=1
  fi
done

if [ "${FAIL}" -eq 1 ]; then
  echo "" >&2
  echo "ggen must emit directly to canonical runtime paths." >&2
  echo "No generated/ staging directories are permitted." >&2
  exit 1
fi

echo "assert:no-generated-dir PASS — no forbidden directories found"
