#!/usr/bin/env bash
# Adversarial gate: detect dead params in CLI verb implementations
# Any "let _ = param;" in src/cmds/ is a soft stub = theater
# Armstrong principle: Parameters accepted but silently ignored are theater.
# Verifies that every CLI parameter either:
#   1. Is wired to real behavior, OR
#   2. Is explicitly documented as unsupported (returns clear error)
set -euo pipefail

cd "$(dirname "$0")/.."

echo "Scanning src/cmds/ for dead parameters..."

# Grep for "let _ = <identifier>;" patterns
# Exclude known safe patterns (std bindings, match expressions, error results)
VIOLATIONS=$(grep -rn 'let _ = [a-z_][a-z_0-9]*;' src/cmds/ \
  | grep -v 'let _ = std::' \
  | grep -v 'let _ = match ' \
  | grep -v '_guard;' \
  | grep -v 'let _ = result;' \
  | grep -v 'let _ = Err' \
  | grep -v 'let _ = Ok' \
  || true)

if [ -n "$VIOLATIONS" ]; then
    echo "❌ ADVERSARIAL GATE FAIL: Soft stubs (dead params) detected:"
    echo ""
    echo "$VIOLATIONS"
    echo ""
    echo "Parameters accepted in CLI but silently ignored are theater."
    echo "These indicate that a --flag is exposed in the CLI but does nothing."
    echo ""
    echo "Fix one of:"
    echo "  1. Wire the param to real behavior (forward it to the underlying lib call)"
    echo "  2. Remove it from the CLI signature (delete it from the #[verb] function)"
    echo "  3. Return a clear error message (\"feature not yet supported\")"
    echo ""
    exit 1
fi

echo "✓ No dead params detected in src/cmds/"
exit 0
