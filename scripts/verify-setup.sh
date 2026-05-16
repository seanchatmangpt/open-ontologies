#!/usr/bin/env bash
# verify-setup.sh — Quick health check for open-ontologies dev environment.
# Run after cloning or after pulling major changes.
# Exit 0: setup is healthy. Exit 1: one or more issues found.

set -euo pipefail

PASS=0
FAIL=0

ok()   { echo "  [OK]  $*"; PASS=$((PASS+1)); }
fail() { echo "  [!!]  $*"; FAIL=$((FAIL+1)); }
warn() { echo "  [??]  $*"; }

echo ""
echo "open-ontologies setup verification"
echo "─────────────────────────────────────────────────"

# ── 1. Rust toolchain ────────────────────────────────
echo "1. Rust toolchain"
if command -v cargo >/dev/null 2>&1; then
    ok "cargo $(cargo --version 2>&1 | head -1)"
else
    fail "cargo not found — install Rust via https://rustup.rs"
fi

# ── 2. Required sister repos ─────────────────────────
echo "2. Sister repositories"
for path in \
    /Users/sac/wasm4pm \
    /Users/sac/mcpp \
    /Users/sac/mcpp/crates/mcpp-core; do
    if [ -d "$path" ]; then
        ok "$path"
    else
        fail "Missing: $path (required by Cargo.toml path dep)"
    fi
done

# ── 3. Python scripts dependencies ───────────────────
echo "3. Python environment"
if command -v python3 >/dev/null 2>&1; then
    ok "python3 $(python3 --version 2>&1)"
    for pkg in groq pm4py; do
        if python3 -c "import $pkg" 2>/dev/null; then
            ok "  python: $pkg available"
        else
            warn "  python: $pkg not installed (needed for real Groq/pm4py tests)"
        fi
    done
else
    warn "python3 not found (needed for Groq integration scripts)"
fi

# ── 4. Binary compiles ───────────────────────────────
echo "4. Binary compilation"
if cargo build --release -q 2>/dev/null; then
    ok "cargo build --release succeeded"
    BIN=./target/release/open-ontologies
    if "$BIN" --help >/dev/null 2>&1; then
        ok "binary starts (--help exit 0)"
    else
        fail "binary exits non-zero on --help"
    fi
else
    fail "cargo build --release failed — run 'cargo check' for details"
fi

# ── 5. Config file ───────────────────────────────────
echo "5. Config"
CFG_PATHS=(
    ./config.toml
    ~/.config/open-ontologies/config.toml
    /etc/open-ontologies/config.toml
)
FOUND_CFG=0
for p in "${CFG_PATHS[@]}"; do
    if [ -f "$p" ]; then
        ok "config at $p"
        FOUND_CFG=1
        break
    fi
done
if [ "$FOUND_CFG" -eq 0 ]; then
    warn "No config.toml found — server will use built-in defaults"
    warn "  Copy config.example.toml → config.toml to customize"
fi

# ── 6. Key environment variables ─────────────────────
echo "6. Environment"
for var in GROQ_API_KEY MCPP_SIGNING_KEY_PATH OPEN_ONTOLOGIES_SIGNING_KEY_PATH; do
    if [ -n "${!var:-}" ]; then
        ok "$var is set"
    else
        warn "$var not set (optional — see README for when it's needed)"
    fi
done

# ── 7. make check ────────────────────────────────────
echo "7. make check"
if make check -s 2>/dev/null; then
    ok "make check passed"
else
    fail "make check failed — fix compilation errors before proceeding"
fi

# ── Summary ──────────────────────────────────────────
echo ""
echo "─────────────────────────────────────────────────"
echo "  Passed: $PASS   Failed: $FAIL"
if [ "$FAIL" -gt 0 ]; then
    echo "  Setup has issues — fix the [!!] items above."
    exit 1
else
    echo "  Setup looks good. Run 'make test' to verify."
    exit 0
fi
