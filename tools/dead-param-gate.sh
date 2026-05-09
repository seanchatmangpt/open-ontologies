#!/usr/bin/env bash
# Adversarial gate: detect dead params and silenced gate-fn results.
#
# Original rule (src/cmds/ scope):
#   `let _ = <ident>;` is a soft stub = theater (parameter accepted but ignored).
#
# Hardened rules (src/ + tests/ scope, ratchet_red_team.rs excluded as fixture):
#   `let _ = self.<gate_fn>(...)`  — discarding admission/receipt/event/sig results
#   `^\s*_ = self.<gate_fn>(...)`  — bare assignment form of the same
#   `let _ = self.<verb>(...)`     — generic Result-returning methods that
#                                    verify/persist/emit/admit/evaluate
#
# Armstrong principle: parameters and gate-fn return values that are silently
# ignored are theater. Either wire them in, or remove them.
set -euo pipefail

cd "$(dirname "$0")/.."

GATE_FNS='evaluate_admission|evaluate_admission_audit|persist_receipt|emit_event|verify_signature|admit'

# tests/ratchet_red_team.rs deliberately contains these patterns as fixtures —
# the test embeds offending code as content and asserts a separate gate flags
# them. Excluding it here is mandatory; it is not a real call site.
EXCLUDE='tests/ratchet_red_team.rs'

echo "Scanning src/cmds/ for dead parameters (legacy rule)..."

VIOLATIONS=$(grep -rn 'let _ = [a-z_][a-z_0-9]*;' src/cmds/ \
  | grep -v 'let _ = std::' \
  | grep -v 'let _ = match ' \
  | grep -v '_guard;' \
  | grep -v 'let _ = result;' \
  | grep -v 'let _ = Err' \
  | grep -v 'let _ = Ok' \
  || true)

if [ -n "$VIOLATIONS" ]; then
    echo "❌ ADVERSARIAL GATE FAIL: Soft stubs (dead params) detected in src/cmds/:"
    echo ""
    echo "$VIOLATIONS"
    echo ""
    echo "Parameters accepted in CLI but silently ignored are theater."
    echo "Fix one of:"
    echo "  1. Wire the param to real behavior"
    echo "  2. Remove it from the CLI signature"
    echo "  3. Return a clear error message"
    exit 1
fi

echo "✓ No dead params in src/cmds/"

echo "Scanning src/ + tests/ for silenced gate-fn results..."

# Rule 1: `let _ = self.<gate_fn>(`
GATE_LET=$(grep -rnE "let _ = self\.($GATE_FNS)\(" src/ tests/ 2>/dev/null \
  | grep -v "$EXCLUDE" || true)
# Rule 2: `_ = self.<gate_fn>(` at start of expression
GATE_BARE=$(grep -rnE "^\s*_ = self\.($GATE_FNS)\(" src/ tests/ 2>/dev/null \
  | grep -v "$EXCLUDE" || true)
# Rule 3: generic Result-returning verb method discard
GENERIC_DISCARD=$(grep -rnE "let _ = self\.[a-z_]*(verify|persist|emit|admit|evaluate)[a-z_]*\(" src/ tests/ 2>/dev/null \
  | grep -v "$EXCLUDE" || true)

FAIL=0
if [ -n "$GATE_LET" ]; then
  echo "❌ Gate-fn Result discarded via \`let _ = self.<gate>(...)\`:"
  echo "$GATE_LET"
  FAIL=1
fi
if [ -n "$GATE_BARE" ]; then
  echo "❌ Gate-fn Result discarded via bare \`_ = self.<gate>(...)\`:"
  echo "$GATE_BARE"
  FAIL=1
fi
if [ -n "$GENERIC_DISCARD" ]; then
  echo "❌ Verify/persist/emit/admit/evaluate Result discarded:"
  echo "$GENERIC_DISCARD"
  FAIL=1
fi

if [ "$FAIL" -ne 0 ]; then
  echo ""
  echo "Discarding gate-fn Results turns enforcement into theater."
  echo "Propagate with \`?\`, match on the Result, or assert success."
  exit 1
fi

echo "✓ No silenced gate-fn discards in src/ + tests/"
exit 0
