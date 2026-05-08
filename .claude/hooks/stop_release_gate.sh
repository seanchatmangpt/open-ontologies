#!/bin/bash
# Stop Release Gate - Check compliance before stopping

set -euo pipefail

# Check adversarial gate passes
if ! make adversarial > /dev/null 2>&1; then
  echo "ERROR: make adversarial failed - cannot stop" >&2
  exit 1
fi

# Output gate decision
echo '{"stop_allowed": true, "reason": "adversarial_gate_passed"}' >&2
exit 0
