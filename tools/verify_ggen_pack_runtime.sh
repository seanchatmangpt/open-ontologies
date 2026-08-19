#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PACK="$ROOT/packs/open-ontologies-pack"
GGEN_SHA="7fc324df397973004059c37b752a365315d7bfb8"
GGEN_TOOLCHAIN="nightly-2026-06-22"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cd "$ROOT"
python3 tools/ggen_pack.py --check

rustup toolchain install "$GGEN_TOOLCHAIN" --profile minimal --component rustfmt

git clone --filter=blob:none --no-checkout https://github.com/seanchatmangpt/ggen.git "$TMP/ggen"
git -C "$TMP/ggen" fetch --depth=1 origin "$GGEN_SHA"
git -C "$TMP/ggen" checkout --detach "$GGEN_SHA"
git -C "$TMP/ggen" rev-parse HEAD | grep -Fx "$GGEN_SHA"

(
  cd "$TMP/ggen"
  cargo +"$GGEN_TOOLCHAIN" build --locked -p ggen-cli-lib --bin ggen
)

GGEN_BIN="$TMP/ggen/target/debug/ggen"
"$GGEN_BIN" --version

cd "$PACK"
rm -rf generated .ggen-v2

"$GGEN_BIN" sync run --dry-run
test ! -e generated/cmds.rs

"$GGEN_BIN" sync run
test -s generated/cmds.rs
test -s .ggen-v2/receipt.json
first_sha="$(sha256sum generated/cmds.rs | awk '{print $1}')"
"$GGEN_BIN" receipt verify

"$GGEN_BIN" sync run
second_sha="$(sha256sum generated/cmds.rs | awk '{print $1}')"
"$GGEN_BIN" receipt verify

if [[ "$first_sha" != "$second_sha" ]]; then
  echo "REFUSED:GGEN_PACK_NONDETERMINISTIC first=$first_sha second=$second_sha" >&2
  exit 3
fi

python3 - "$GGEN_SHA" "$GGEN_TOOLCHAIN" "$first_sha" <<'PY'
import json
import sys
print(json.dumps({
    "schema": "chatmangpt.ggen-pack-runtime/v1",
    "standing": "ALIVE",
    "ggen_sha": sys.argv[1],
    "ggen_toolchain": sys.argv[2],
    "generated_cmds_sha256": sys.argv[3],
    "dry_run": "passed",
    "sync_run_1": "passed",
    "receipt_verify_1": "passed",
    "sync_run_2": "passed",
    "receipt_verify_2": "passed",
    "deterministic_replay": True,
}, sort_keys=True))
PY
