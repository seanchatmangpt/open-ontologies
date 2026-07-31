#!/usr/bin/env bash
# verify-setup.sh — Quick health check for a clean open-ontologies checkout.
# Exit 0: bounded default-feature setup is healthy. Exit 1: issues found.

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
    observed="$(rustc --version 2>&1 | awk '{print $2}')"
    if [ "$observed" = "1.97.1" ]; then
        ok "rustc $observed matches rust-toolchain.toml"
    else
        fail "rustc $observed does not match admitted 1.97.1 toolchain"
    fi
else
    fail "cargo not found — install Rust via https://rustup.rs"
fi

# ── 2. Constitutional and dependency admission ─────
echo "2. Constitutional and dependency admission"
if python3 tools/verify_ggen_standards.py >/dev/null; then
    ok "ggen v26.7.31 repository contract admitted"
else
    fail "ggen standards verifier refused the repository"
fi
if cargo metadata --locked --format-version 1 --no-deps >/dev/null 2>&1; then
    ok "committed Cargo.lock resolves without workstation paths"
else
    fail "locked dependency metadata does not resolve"
fi
if grep -Eq 'path\s*=\s*"(/|[A-Za-z]:\\)' Cargo.toml; then
    fail "Cargo.toml contains an absolute workstation dependency"
else
    ok "Cargo.toml has no absolute workstation dependency"
fi
if grep -Eq '^mcpp\s*=\s*\[\s*\]\s*$' Cargo.toml; then
    warn "mcpp feature is reserved but UNSUPPORTED until mcpp-core is published or vendored"
fi

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
    fail "python3 not found (required by the standards verifier)"
fi

# ── 4. Binary compiles ───────────────────────────────
echo "4. Binary compilation"
if cargo build --locked --release -q 2>/dev/null; then
    ok "cargo build --locked --release succeeded"
    BIN=./target/release/open-ontologies
    if "$BIN" --help >/dev/null 2>&1; then
        ok "binary starts (--help exit 0)"
    else
        fail "binary exits non-zero on --help"
    fi
else
    fail "locked release build failed — run 'cargo check --locked' for details"
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
for var in GROQ_API_KEY OPEN_ONTOLOGIES_SIGNING_KEY_PATH; do
    if [ -n "${!var:-}" ]; then
        ok "$var is set"
    else
        warn "$var not set (optional — see README for when it is needed)"
    fi
done

# ── 7. make check ────────────────────────────────────
echo "7. make check"
if make check -s 2>/dev/null; then
    ok "make check passed"
else
    fail "make check failed — inspect the first refused gate"
fi

# ── Summary ──────────────────────────────────────────
echo ""
echo "─────────────────────────────────────────────────"
echo "  Passed: $PASS   Failed: $FAIL"
if [ "$FAIL" -gt 0 ]; then
    echo "  Setup has issues — fix the [!!] items above."
    exit 1
else
    echo "  Bounded default-feature setup looks good."
    exit 0
fi
