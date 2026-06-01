#!/usr/bin/env bash
#
# watch-ggen-onto.sh — File system monitor for ontology TTL changes
#
# Watches ontology/ directory for .ttl file changes and triggers the ggen pipeline:
#   1. Detect .ttl file modification (inotifywait on macOS/Linux)
#   2. Run: ggen sync --audit true
#   3. Run: onto validate (SHACL conformance)
#   4. Register artifacts in onto:artifact-registry
#   5. Record lineage event
#
# Usage:
#   bash tools/watch-ggen-onto.sh               # watch ontology/ directory
#   bash tools/watch-ggen-onto.sh --dry-run     # preview what would run without executing
#
# Exit codes:
#   0 = watch loop active
#   1 = inotifywait not found OR ggen sync failed
#   2 = onto validate failed (SHACL violations)
#
# Environment variables:
#   DEBUG=1        — Enable verbose output
#   QUIET=1        — Suppress status messages (errors still logged)
#   STOP_ON_ERROR  — Exit on first validation failure (default: continue watching)
#

set -euo pipefail

# ─── Configuration ───────────────────────────────────────────────────────────

ONTOLOGY_DIR="ontology"
WATCH_PATTERN="*.ttl"
RECEIPT_DIR=".ggen/receipts"
DRY_RUN="${1:---watch}"
DEBUG="${DEBUG:-0}"
QUIET="${QUIET:-0}"
STOP_ON_ERROR="${STOP_ON_ERROR:-0}"

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# ─── Helpers ─────────────────────────────────────────────────────────────────

log() {
    [ "$QUIET" = "1" ] && return 0
    echo -e "${BLUE}[watch-ggen-onto]${NC} $*" >&2
}

log_success() {
    [ "$QUIET" = "1" ] && return 0
    echo -e "${GREEN}✓${NC} $*" >&2
}

log_error() {
    echo -e "${RED}✗${NC} $*" >&2
}

log_debug() {
    [ "$DEBUG" != "1" ] && return 0
    echo -e "${YELLOW}[DEBUG]${NC} $*" >&2
}

# ─── Dependency checks ───────────────────────────────────────────────────────

check_dependencies() {
    local missing_tools=()

    # Check for inotifywait (Linux) or fswatch (macOS)
    if ! command -v inotifywait &>/dev/null && ! command -v fswatch &>/dev/null; then
        missing_tools+=("inotifywait (Linux) or fswatch (macOS)")
    fi

    # Check for cargo (required for ggen sync)
    if ! command -v cargo &>/dev/null; then
        missing_tools+=("cargo")
    fi

    if [ ${#missing_tools[@]} -gt 0 ]; then
        log_error "Missing dependencies:"
        for tool in "${missing_tools[@]}"; do
            log_error "  - $tool"
        done
        log_error ""
        log_error "Installation:"
        log_error "  macOS:  brew install fswatch"
        log_error "  Linux:  apt-get install inotify-tools"
        log_error "  Cargo:  https://rustup.rs/"
        exit 1
    fi
}

# ─── File change detection ───────────────────────────────────────────────────

watch_with_inotifywait() {
    inotifywait \
        --monitor \
        --recursive \
        --event close_write \
        --format '%w%f' \
        --exclude '(\..*|node_modules)' \
        "$ONTOLOGY_DIR"
}

watch_with_fswatch() {
    fswatch \
        --event Updated \
        --recursive \
        --exclude '\..*' \
        "$ONTOLOGY_DIR"
}

# ─── Pipeline execution ──────────────────────────────────────────────────────

run_ggen_sync() {
    local file=$1
    log "Detected change: $file"
    log "Running: cargo run --release -- ggen sync --audit true"

    if ! cargo run --release -- ggen sync --audit true 2>&1 | tee /tmp/ggen_sync.log; then
        log_error "ggen sync failed"
        return 1
    fi

    log_success "ggen sync completed"
    return 0
}

run_onto_validate() {
    log "Running: cargo run --release -- ontology validate"

    if ! cargo run --release -- ontology validate 2>&1 | tee /tmp/onto_validate.log; then
        log_error "onto validate failed (SHACL violations)"
        return 2
    fi

    log_success "onto validate passed"
    return 0
}

register_artifacts() {
    local latest_receipt="${RECEIPT_DIR}/latest.json"

    if [ ! -f "$latest_receipt" ]; then
        log_error "Receipt not found: $latest_receipt"
        return 1
    fi

    log "Registering artifacts from receipt: $latest_receipt"

    # Extract output hashes and paths from receipt
    local output_count
    output_count=$(jq '.output_hashes | length' "$latest_receipt" 2>/dev/null || echo "0")
    log_debug "Receipt contains $output_count output artifacts"

    # In a real system, this would POST to onto:artifact-registry or update a lineage table.
    # For now, we just verify the receipt exists and is valid.
    if ! jq -e '.signature | length > 0' "$latest_receipt" >/dev/null 2>&1; then
        log_error "Receipt signature is empty (invalid)"
        return 1
    fi

    log_success "Artifacts registered (receipt: $latest_receipt)"
    return 0
}

record_lineage_event() {
    local file=$1
    local timestamp
    timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

    log "Recording lineage event"
    log_debug "  file: $file"
    log_debug "  timestamp: $timestamp"
    log_debug "  event: User edited TTL → ggen regenerated → validation passed"

    # In a real system, this would append to onto:lineage-log (RDF or database).
    # For now, we log to a local lineage file.
    local lineage_log=".ggen/lineage.log"
    mkdir -p "$(dirname "$lineage_log")"
    {
        echo "timestamp: $timestamp"
        echo "event: ggen-sync"
        echo "file: $file"
        echo "status: success"
    } >> "$lineage_log"

    log_success "Lineage recorded"
    return 0
}

# ─── Main watch loop ────────────────────────────────────────────────────────

watch_loop() {
    local watch_cmd

    if command -v inotifywait &>/dev/null; then
        watch_cmd="watch_with_inotifywait"
    elif command -v fswatch &>/dev/null; then
        watch_cmd="watch_with_fswatch"
    else
        log_error "No file change detection tool available"
        exit 1
    fi

    log "Watching for changes in: $ONTOLOGY_DIR"
    log "Press Ctrl+C to stop"
    log ""

    $watch_cmd | while read -r changed_file; do
        log "────────────────────────────────────────────────────────────────"

        if ! run_ggen_sync "$changed_file"; then
            if [ "$STOP_ON_ERROR" = "1" ]; then
                log_error "Stopping due to ggen sync failure (STOP_ON_ERROR=1)"
                exit 1
            fi
            log "Continuing to watch (error recovery enabled)"
            continue
        fi

        if ! run_onto_validate; then
            if [ "$STOP_ON_ERROR" = "1" ]; then
                log_error "Stopping due to validation failure (STOP_ON_ERROR=1)"
                exit 2
            fi
            log "Continuing to watch (error recovery enabled)"
            continue
        fi

        if ! register_artifacts; then
            log_error "Failed to register artifacts (continuing anyway)"
        fi

        if ! record_lineage_event "$changed_file"; then
            log_error "Failed to record lineage (continuing anyway)"
        fi

        log_success "Cycle complete"
        log ""
    done
}

# ─── Dry-run mode ───────────────────────────────────────────────────────────

dry_run_demo() {
    log "Dry-run mode: showing what would execute on file change"
    log ""
    log "1. Detect change: ontology/cli-open-ontologies.ttl"
    log ""
    log "2. Run ggen sync:"
    log "   \$ cargo run --release -- ggen sync --audit true"
    log "   → Produces: src/cmds/generated.rs"
    log "   → Produces: .ggen/receipts/latest.json"
    log ""
    log "3. Run onto validate:"
    log "   \$ cargo run --release -- ontology validate"
    log "   → Checks SHACL shapes (A1-A3 gates)"
    log "   → Reports violations if any"
    log ""
    log "4. Register artifacts:"
    log "   → Extract output hashes from receipt"
    log "   → Verify signature is non-empty"
    log "   → Update onto:artifact-registry"
    log ""
    log "5. Record lineage event:"
    log "   → Append to .ggen/lineage.log"
    log "   → Event: User edited TTL → ggen regenerated → validation passed"
    log ""
    log "To start watching, run:"
    log "  make watch-ggen-onto"
    exit 0
}

# ─── Entry point ────────────────────────────────────────────────────────────

main() {
    case "${DRY_RUN}" in
        --dry-run | -d | demo)
            dry_run_demo
            ;;
        --watch | -w | "")
            check_dependencies
            watch_loop
            ;;
        *)
            log_error "Unknown option: $DRY_RUN"
            log_error ""
            log_error "Usage: bash tools/watch-ggen-onto.sh [--dry-run|--watch]"
            exit 1
            ;;
    esac
}

main "$@"
