#!/usr/bin/env bash
# validate-no-downstream-authority.sh
#
# P0-A refinement #6: codifies the anti-inversion doctrine.
#
# Surfaces every place where downstream-artifact authority language appears
# in ggen manifests, migration comments, or generated-file headers. These
# patterns indicate that a downstream artifact (SQL, edge function, screen)
# is being treated as source-of-truth, which is a source-of-truth inversion
# violation per .claude/projects/-Users-sac-open-ontologies/memory/
# feedback_manufacturing_doctrine.md.
#
# This validator is REPORT-ONLY for P0-A: it lists current violations but
# exits 0 unless the count INCREASES beyond the P0-A baseline. The baseline
# is the 4 protected_paths entries for hand-authored supabase/migrations/,
# which P0-B is responsible for eliminating.
#
# In a future release (after P0-B completes), this validator escalates to
# exit-non-zero on any match.

set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Patterns that indicate downstream-artifact authority
PATTERNS='protected_paths|DO NOT EDIT.*hand-authored|do not overwrite|manual authority|hand-written.*never overwrite'

SCOPE_PATHS=(
  "${REPO_ROOT}/ggen.toml"
  "${REPO_ROOT}/ggen-zoela-mobile.toml"
  "${REPO_ROOT}/supabase/migrations"
  "${REPO_ROOT}/supabase/functions"
  "${REPO_ROOT}/.specify/templates/zoela"
)

EXISTING_PATHS=()
for p in "${SCOPE_PATHS[@]}"; do
  [ -e "$p" ] && EXISTING_PATHS+=("$p")
done

# Baseline established at P0-A landing (2026-05-18):
# - 4 protected_paths entries in ggen.toml for supabase/migrations/*.sql
# - Comment lines mentioning "hand-authored" in those migration headers
# - Each migration file has a "DO NOT EDIT" / "Regenerate with:" header
# Total acceptable baseline matches: 15
# (If you change ggen.toml or migration headers, recount this number.)
BASELINE_COUNT=15

CURRENT_COUNT=$(grep -rEc "${PATTERNS}" \
  --include='*.toml' --include='*.sql' --include='*.ts' --include='*.tera' --include='*.tsx' \
  --exclude='*.backup' \
  "${EXISTING_PATHS[@]}" 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')

echo "validate:no-downstream-authority — informational report"
echo "  current downstream-authority pattern matches: ${CURRENT_COUNT}"
echo "  P0-A baseline (acceptable until P0-B):         ${BASELINE_COUNT}"

if [ "${CURRENT_COUNT}" -gt "${BASELINE_COUNT}" ]; then
  echo "FAIL: downstream-authority drift — new protected/manual artifacts introduced." >&2
  echo "Listing matches:" >&2
  grep -rnE "${PATTERNS}" \
    --include='*.toml' --include='*.sql' --include='*.ts' --include='*.tera' --include='*.tsx' \
    --exclude='*.backup' \
    "${EXISTING_PATHS[@]}" 2>/dev/null >&2 || true
  echo "" >&2
  echo "Doctrine: downstream artifacts must regenerate from ontology/SPARQL/Tera." >&2
  echo "Do NOT add new protected_paths or hand-authored artifact entries." >&2
  echo "Fix at source (TTL/SPARQL/Tera) and let ggen emit the artifact." >&2
  exit 1
fi

if [ "${CURRENT_COUNT}" -gt 0 ]; then
  echo "  (${CURRENT_COUNT} known matches remain — to be eliminated in P0-B)"
fi

echo "validate:no-downstream-authority PASS"
