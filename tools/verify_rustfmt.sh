#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

OUT_DIR="${1:-target/verifier}"
STATUS_FILE="$OUT_DIR/rustfmt-status.txt"
PATCH_FILE="$OUT_DIR/rustfmt.patch"

mkdir -p "$OUT_DIR"
rm -f "$PATCH_FILE"

if cargo fmt --all -- --check; then
  printf '%s\n' 'RUSTFMT_ALIVE' > "$STATUS_FILE"
  exit 0
fi

cargo fmt --all
git diff --binary -- '*.rs' > "$PATCH_FILE"
printf '%s\n' 'REFUSED:RUSTFMT_DRIFT' > "$STATUS_FILE"
exit 86
