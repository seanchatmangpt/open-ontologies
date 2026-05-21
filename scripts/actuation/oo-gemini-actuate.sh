#!/usr/bin/env bash
set -eo pipefail

PLAN_PATH="$1"
if [[ -z "$PLAN_PATH" || ! -f "$PLAN_PATH" ]]; then
  echo "Usage: $0 <path_to_actuation_plan.json>"
  exit 1
fi
PLAN_PATH="$(realpath "$PLAN_PATH")"

TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
RECEIPTS_DIR="$(pwd)/artifacts/actuation/receipts"
REFUSALS_DIR="$(pwd)/artifacts/actuation/refusals"
mkdir -p "$RECEIPTS_DIR" "$REFUSALS_DIR"

emit_refusal() {
  local code="$1"
  local reason="$2"
  local action_id
  action_id=$(jq -r '.action_id // "unknown"' "$PLAN_PATH")
  local out_path="$REFUSALS_DIR/refusal_$(date +%s)_$RANDOM.json"
  
  # Create refusal receipt without hash first
  jq -n \
    --arg rt "GeminiCliRefusalReceipt" \
    --arg aid "$action_id" \
    --arg req "open-ontologies" \
    --arg act "gemini-cli" \
    --argjson all false \
    --arg rcode "$code" \
    --arg rsn "$reason" \
    '{
      receipt_type: $rt,
      action_id: $aid,
      requested_by: $req,
      actuator: $act,
      allowed: $all,
      refusal_code: $rcode,
      reason: $rsn
    }' > "$out_path.tmp"
    
  local hash
  hash=$(shasum -a 256 "$out_path.tmp" | awk '{print $1}')
  
  jq --arg h "$hash" '. + {receipt_hash: $h}' "$out_path.tmp" > "$out_path"
  rm "$out_path.tmp"
  
  echo "Refused: $code - $reason (receipt: $out_path)"
  exit 0
}

# Verify plan authority
EMITTER=$(jq -r '.emitted_by // empty' "$PLAN_PATH")
if [[ "$EMITTER" != "open-ontologies" ]]; then
  emit_refusal "POLICY_NOT_FOUND" "Plan not emitted by open-ontologies"
fi

POLICY_ID=$(jq -r '.policy_id // empty' "$PLAN_PATH")
if [[ -z "$POLICY_ID" ]]; then
  emit_refusal "POLICY_NOT_FOUND" "policy_id missing in ActuationPlan"
fi

ALLOWED=$(jq -r '.allowed // empty' "$PLAN_PATH")
if [[ "$ALLOWED" != "true" ]]; then
  emit_refusal "ACTION_NOT_ALLOWED" "action not allowed by policy"
fi

WORKDIR=$(jq -r '.working_directory // empty' "$PLAN_PATH")
if [[ "$WORKDIR" != "/Users/sac/open-ontologies" && "$WORKDIR" != "/Users/sac/wasm4pm" ]]; then
  emit_refusal "FORBIDDEN_WRITE_ROOT" "working_directory is not allowed: $WORKDIR"
fi

PROMPT=$(jq -r '.prompt // empty' "$PLAN_PATH")
if [[ -z "$PROMPT" ]]; then
  PROMPT=$(jq -r '.command // empty' "$PLAN_PATH")
  if [[ -z "$PROMPT" ]]; then
    emit_refusal "FORBIDDEN_SHELL_COMMAND" "No prompt or command specified"
  fi
fi

ACTION_ID=$(jq -r '.action_id // empty' "$PLAN_PATH")

# Capture git state
GIT_BEFORE=$(git rev-parse HEAD 2>/dev/null || echo "unknown")
GIT_DIRTY_BEFORE=$(git status --short 2>/dev/null || true)

# Execution params
TIMEOUT_SECONDS=600
PRIMARY_MODEL="gemini-3.1-flash-lite-preview"
FALLBACK_MODEL="gemini-3.1-pro-preview"
RUNNER="npx -y @google/gemini-cli"

INPUTS_HASH=$(echo -n "$PROMPT" | shasum -a 256 | awk '{print $1}')

cd "$WORKDIR" || emit_refusal "FORBIDDEN_WRITE_ROOT" "Cannot cd to $WORKDIR"

STDOUT_FILE=$(mktemp)
STDERR_FILE=$(mktemp)

gemini_call() {
  local model="$1"
  timeout "$TIMEOUT_SECONDS" npx -y @google/gemini-cli -p "$PROMPT" --model "$model" --approval-mode yolo < /dev/null > "$STDOUT_FILE" 2> "$STDERR_FILE"
  return $?
}

# Try primary
set +e
gemini_call "$PRIMARY_MODEL"
EXIT_CODE=$?
set -e

# Fallback detection
if [[ $EXIT_CODE -ne 0 || ! -s "$STDOUT_FILE" ]]; then
  set +e
  gemini_call "$FALLBACK_MODEL"
  EXIT_CODE=$?
  set -e
fi

GIT_AFTER=$(git rev-parse HEAD 2>/dev/null || echo "unknown")
GIT_DIRTY_AFTER=$(git status --short 2>/dev/null || true)

# Simple files_changed array by checking git status diff
FILES_CHANGED=$(git diff --name-only | jq -R -s -c 'split("\n")[:-1]')

STDOUT_HASH=$(shasum -a 256 "$STDOUT_FILE" | awk '{print $1}')
STDERR_HASH=$(shasum -a 256 "$STDERR_FILE" | awk '{print $1}')

# Emit receipt
OUT_RECEIPT="$RECEIPTS_DIR/receipt_$(date +%s)_$RANDOM.json"

ACTOR_BASIS8=$(jq -r '.actor_basis8 // "unknown"' "$PLAN_PATH")

jq -n \
  --arg rt "GeminiCliActuationReceipt" \
  --arg aid "$ACTION_ID" \
  --arg req "open-ontologies" \
  --arg act "gemini-cli" \
  --arg wd "$WORKDIR" \
  --arg cmd "$PROMPT" \
  --arg ih "$INPUTS_HASH" \
  --arg soh "$STDOUT_HASH" \
  --arg seh "$STDERR_HASH" \
  --argjson ec "$EXIT_CODE" \
  --argjson fc "$FILES_CHANGED" \
  --arg gb "$GIT_BEFORE" \
  --arg ga "$GIT_AFTER" \
  --arg pid "$POLICY_ID" \
  --arg ab8 "$ACTOR_BASIS8" \
  --arg bt "headless_membrane" \
  --argjson all true \
  --arg ts "$TIMESTAMP" \
  '{
    receipt_type: $rt,
    action_id: $aid,
    requested_by: $req,
    actuator: $act,
    working_directory: $wd,
    command: $cmd,
    inputs_hash: $ih,
    stdout_hash: $soh,
    stderr_hash: $seh,
    exit_code: $ec,
    files_changed: $fc,
    git_before: $gb,
    git_after: $ga,
    policy_id: $pid,
    actor_basis8: $ab8,
    boundary_type: $bt,
    allowed: $all,
    refusal_code: null,
    created_at: $ts
  }' > "$OUT_RECEIPT.tmp"

RECEIPT_HASH=$(shasum -a 256 "$OUT_RECEIPT.tmp" | awk '{print $1}')
jq --arg h "$RECEIPT_HASH" '. + {receipt_hash: $h}' "$OUT_RECEIPT.tmp" > "$OUT_RECEIPT"
rm "$OUT_RECEIPT.tmp"

rm -f "$STDOUT_FILE" "$STDERR_FILE"
echo "Actuation complete. Receipt emitted: $OUT_RECEIPT"
exit 0
