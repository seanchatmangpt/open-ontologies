#!/usr/bin/env bash
# untrack-secret.sh — fix-forward removal of an accidentally-committed
# secret file. Removes the file from the git index (keeps it on disk),
# appends the path to .gitignore if not already present, and stages
# both changes for a single fix-forward commit.
#
# Usage:
#   tools/untrack-secret.sh <path> [<path>...]
#
# Example:
#   tools/untrack-secret.sh .env config/local.env
#
# Notes:
# - The file content remains in git HISTORY at the prior commit. The
#   only complete remediation is to ROTATE the secret value at the
#   issuing service. This script only stops the bleeding going forward.
# - Per the project fix-forward rule (CLAUDE.md): never `git reset
#   --hard`, never amend a published commit, never force-push.

set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <path> [<path>...]" >&2
  exit 2
fi

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

ignore=".gitignore"
touch "$ignore"

for path in "$@"; do
  if [[ ! -e "$path" && ! "$(git ls-files -- "$path")" ]]; then
    echo "warning: $path neither on disk nor tracked — nothing to do" >&2
    continue
  fi

  # 1. Remove from index (recursive, in case caller passes a directory).
  if git ls-files --error-unmatch -- "$path" >/dev/null 2>&1; then
    git rm --cached -r --quiet -- "$path"
    echo "untracked: $path"
  else
    echo "already untracked: $path"
  fi

  # 2. Append to .gitignore if not already present (exact-line match).
  if ! grep -Fxq "$path" "$ignore"; then
    {
      echo ""
      echo "# Untracked by tools/untrack-secret.sh on $(date -u +%Y-%m-%dT%H:%M:%SZ)"
      echo "$path"
    } >> "$ignore"
    echo "appended to .gitignore: $path"
  else
    echo "already in .gitignore: $path"
  fi
done

git add "$ignore"

cat <<EOF

Next steps:
  1. Review staged changes:   git diff --cached
  2. Commit:                  git commit -m "fix(security): untrack <path>; tighten .gitignore"
  3. ROTATE the secret value at the issuing service. The old value
     remains in git history at the prior commit and is permanently
     compromised.
EOF
