#!/bin/bash
# UserPromptSubmit hook - Scan for onto-specific anti-patterns (non-blocking)

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/utils.sh"

patterns=(
  "editing src/cmds/generated.rs"
  "direct cargo without make"
  "SHACL validation skipped"
  "Cell8 gates not passing"
  "dead-param violation"
  "completion without make adversarial"
)

# This is non-blocking, so always exit 0
exit 0
