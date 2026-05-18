#!/usr/bin/env bash
# validate-namespace-singularity.sh
#
# P0-A hard gate: enforces that the ZOE LA ontology toolchain uses exactly
# one canonical namespace IRI: https://zoela.org/ontology/
#
# Fails if any of these banned forms appear in tracked source files:
#   * urn:zoela:               (legacy URN form, was in person/event/core)
#   * https://zoela.org/onto/  (legacy shortened HTTPS form, was in 7 modules)
#
# Scope expanded per P0-A refinements:
#   ontology/zoela/, .specify/queries/zoela/, .specify/templates/zoela/,
#   ggen.toml, ggen-zoela-mobile.toml, supabase/functions/, supabase/seeds/,
#   supabase/migrations/, tests/, .claude/rules/, docs/, specs/, package.json
#
# Explicitly ignored: .backup files (gitignored ggen cruft), .git, node_modules,
#   target, .turbo, .next, .ggen/receipts/ (signed historical artifacts).

set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

BANNED='urn:zoela:|https://zoela\.org/onto/'
CANONICAL='https://zoela.org/ontology/'

SCOPE_PATHS=(
  "${REPO_ROOT}/ontology/zoela"
  "${REPO_ROOT}/.specify/queries/zoela"
  "${REPO_ROOT}/.specify/templates/zoela"
  "${REPO_ROOT}/ggen.toml"
  "${REPO_ROOT}/ggen-zoela-mobile.toml"
  "${REPO_ROOT}/package.json"
  "${REPO_ROOT}/supabase/functions"
  "${REPO_ROOT}/supabase/seeds"
  "${REPO_ROOT}/supabase/migrations"
  "${REPO_ROOT}/tests"
  "${REPO_ROOT}/.claude/rules"
  "${REPO_ROOT}/docs"
)

# Include only source-shaped files; exclude gitignored backups and binaries
INCLUDE_GLOBS=(
  --include='*.ttl' --include='*.rq' --include='*.tera'
  --include='*.toml' --include='*.json' --include='*.sql'
  --include='*.ts' --include='*.tsx' --include='*.md'
  --include='*.rs' --include='*.sh' --include='*.yaml' --include='*.yml'
)

EXCLUDE_DIRS=(
  --exclude-dir='node_modules' --exclude-dir='target'
  --exclude-dir='.turbo' --exclude-dir='.next'
  --exclude-dir='.git' --exclude-dir='receipts'
)

# Only scan paths that actually exist
EXISTING_PATHS=()
for p in "${SCOPE_PATHS[@]}"; do
  [ -e "$p" ] && EXISTING_PATHS+=("$p")
done

VIOLATIONS=$(grep -rnE "${BANNED}" \
  "${INCLUDE_GLOBS[@]}" "${EXCLUDE_DIRS[@]}" \
  --exclude='*.backup' \
  "${EXISTING_PATHS[@]}" 2>/dev/null || true)

if [ -n "${VIOLATIONS}" ]; then
  echo "FAIL: namespace singularity violation — banned ZOE namespace forms found:" >&2
  echo "" >&2
  echo "${VIOLATIONS}" >&2
  echo "" >&2
  echo "Canonical ZOE namespace is: ${CANONICAL}" >&2
  echo "Banned forms: urn:zoela:  https://zoela.org/onto/" >&2
  echo "" >&2
  echo "Rewrite every occurrence to the canonical form, then re-run." >&2
  echo "(P0-A — established 2026-05-18)" >&2
  exit 1
fi

echo "validate:namespace-singularity PASS — only ${CANONICAL} in use"
