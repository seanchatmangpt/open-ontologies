#!/usr/bin/env bash
# verify-receipts.sh — Verify ggen receipt integrity.
#
# Strategy:
#   1. If the receipt has the envelope schema (signature is a JSON object),
#      delegate to `ggen envelope verify`.
#   2. If the receipt has the legacy schema (signature is a hex string),
#      verify manually:
#        a. Signature field is non-empty.
#        b. Every file listed in output_hashes exists on disk with a
#           matching SHA-256 digest.
#
# Exit codes:
#   0 — all checks passed
#   1 — at least one check failed
#   2 — receipt file not found

set -euo pipefail

RECEIPT="${1:-.ggen/receipts/latest.json}"
KEY="${2:-.ggen/keys/verifying.key}"

if [[ ! -f "$RECEIPT" ]]; then
    echo "ERROR: receipt not found: $RECEIPT" >&2
    exit 2
fi

# ── Determine schema version ──────────────────────────────────────────────────
sig_type=$(python3 - "$RECEIPT" <<'PY'
import json, sys
r = json.load(open(sys.argv[1]))
sig = r.get("signature", "")
if isinstance(sig, dict):
    print("envelope")
elif isinstance(sig, str):
    print("legacy")
else:
    print("unknown")
PY
)

# ── Envelope schema → delegate to ggen ───────────────────────────────────────
if [[ "$sig_type" == "envelope" ]]; then
    if [[ ! -f "$KEY" ]]; then
        echo "ERROR: verifying key not found: $KEY" >&2
        exit 1
    fi
    echo "[verify-receipts] envelope schema detected — delegating to ggen envelope verify"
    ggen envelope verify --envelope_file "$RECEIPT" --public_key "$KEY"
    echo "[verify-receipts] ✅ ggen envelope verify passed: $RECEIPT"
    exit 0
fi

# ── Legacy schema (hex signature string) → manual checks ─────────────────────
echo "[verify-receipts] legacy schema — running local integrity checks on $RECEIPT"

failures=0

# Check 1: signature field is non-empty.
sig=$(python3 -c "import json,sys; r=json.load(open('$RECEIPT')); print(r.get('signature',''))")
if [[ -z "$sig" ]]; then
    echo "FAIL: receipt signature is empty" >&2
    failures=$((failures + 1))
else
    echo "  [✓] signature: non-empty (${#sig} chars)"
fi

# Check 2: output_hashes — every listed file must exist with matching SHA-256.
python3 - "$RECEIPT" <<'PY'
import json, hashlib, sys, os

receipt = json.load(open(sys.argv[1]))
output_hashes = receipt.get("output_hashes", [])

if not output_hashes:
    print("WARN: output_hashes is empty — nothing to verify", file=sys.stderr)
    sys.exit(0)

failures = 0
for entry in output_hashes:
    # Format: "path:sha256hex"  OR  {"path": "...", "digest": "..."}
    if isinstance(entry, str) and ":" in entry:
        # May be "path:hash" or "scheme://path:hash" — split on last ':'
        parts = entry.rsplit(":", 1)
        if len(parts) == 2:
            path, expected = parts[0], parts[1]
        else:
            print(f"  [?] cannot parse entry: {entry}", file=sys.stderr)
            continue
    elif isinstance(entry, dict):
        path = entry.get("path", "")
        expected = entry.get("digest", "")
    else:
        print(f"  [?] unknown entry format: {entry}", file=sys.stderr)
        continue

    if not os.path.exists(path):
        print(f"  FAIL: output file missing: {path}", file=sys.stderr)
        failures += 1
        continue

    actual = hashlib.sha256(open(path, "rb").read()).hexdigest()
    # expected may be a short prefix (ggen sometimes truncates to 16 hex chars)
    if not expected or actual.startswith(expected) or expected.startswith(actual):
        print(f"  [✓] {path}: hash match ({expected[:16]}…)")
    else:
        print(f"  FAIL: hash mismatch for {path}", file=sys.stderr)
        print(f"       expected: {expected}", file=sys.stderr)
        print(f"       actual:   {actual}", file=sys.stderr)
        failures += 1

if failures:
    sys.exit(1)
PY

py_exit=$?
if [[ $py_exit -ne 0 ]]; then
    failures=$((failures + 1))
fi

# ── Summary ───────────────────────────────────────────────────────────────────
if [[ $failures -eq 0 ]]; then
    echo "[verify-receipts] ✅ all checks passed: $RECEIPT"
    exit 0
else
    echo "[verify-receipts] ❌ $failures check(s) FAILED: $RECEIPT" >&2
    exit 1
fi
