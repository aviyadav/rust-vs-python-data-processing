#!/usr/bin/env bash
# ============================================================================
#  run_all.sh  —  Build, start, test, and compare both FastAPI versions
# ============================================================================
#
#  This script:
#    1. Builds the Rust validator (cargo build --release)
#    2. Runs the BEFORE (vulnerable) tests
#    3. Starts the Rust validator
#    4. Runs the AFTER (secure) tests
#    5. Stops all services
#
#  Prerequisites: Rust (cargo), Python 3.10+, pip
# ============================================================================

set -euo pipefail

# ── Ensure we run from the script's own directory ───────────────────────
cd "$(dirname "$0")"

cleanup() {
    echo ""
    echo "🧹 Cleaning up..."
    if [[ -n "${RUST_PID:-}" ]]; then
        kill "$RUST_PID" 2>/dev/null || true
        wait "$RUST_PID" 2>/dev/null || true
    fi
    echo "   Done."
}

trap cleanup EXIT

echo ""
echo "============================================================"
echo "  Rust Security Layer for FastAPI  —  Full Test Suite"
echo "============================================================"
echo ""

# ── Step 1: Install Python dependencies ──────────────────────────────
echo "[1/5] Installing Python dependencies..."
pip install -r before/requirements.txt -q 2>/dev/null || {
    echo "ERROR: pip install failed for before/requirements.txt"
    exit 1
}
pip install -r after/fastapi_app/requirements.txt -q 2>/dev/null || {
    echo "ERROR: pip install failed for after/fastapi_app/requirements.txt"
    exit 1
}
echo "      Done."
echo ""

# ── Step 2: Build Rust validator ─────────────────────────────────────
echo "[2/5] Building Rust validator (release mode)..."
if [ ! -f "after/rust_validator/Cargo.toml" ]; then
    echo "ERROR: after/rust_validator/Cargo.toml not found!"
    exit 1
fi
pushd after/rust_validator > /dev/null
cargo build --release 2>&1 || {
    echo "ERROR: Rust build failed!"
    popd > /dev/null
    exit 1
}
popd > /dev/null
echo "      Rust validator built successfully."
echo ""

# ── Step 3: Test BEFORE (vulnerable) app ────────────────────────────
echo "[3/5] Testing BEFORE (vulnerable) FastAPI app..."
echo "      ----------------------------------------"
python before/test_before.py || {
    echo "WARNING: Before tests had failures (exit code $?)."
}
echo ""

# ── Step 4: Start Rust validator in background ──────────────────────
echo "[4/5] Starting Rust validator on port 3000..."

RUST_EXE="after/rust_validator/target/release/rust-validator"
if [ ! -f "$RUST_EXE" ]; then
    echo "ERROR: Rust binary not found at $RUST_EXE"
    echo "       Make sure 'cargo build --release' succeeded."
    exit 1
fi

"$RUST_EXE" &
RUST_PID=$!

# Wait up to 10 seconds for the health endpoint to respond.
for i in $(seq 1 10); do
    sleep 1
    if curl -s http://127.0.0.1:3000/health > /dev/null 2>&1; then
        echo "      Rust validator running (PID $RUST_PID)."
        echo ""
        break
    fi
    if [ "$i" -eq 10 ]; then
        echo "ERROR: Rust validator did not start within 10 seconds!"
        exit 1
    fi
done

# ── Step 5: Test AFTER (secure) app ─────────────────────────────────
echo "[5/5] Testing AFTER (secure) FastAPI app..."
echo "      ----------------------------------------"
python after/fastapi_app/test_after.py
TEST_RESULT=$?
echo ""

# ── Cleanup (trap handles it) ───────────────────────────────────────
if [[ $TEST_RESULT -eq 0 ]]; then
    echo "============================================================"
    echo "  ✅ ALL TESTS PASSED"
    echo "============================================================"
else
    echo "============================================================"
    echo "  ❌ SOME TESTS FAILED (exit code $TEST_RESULT)"
    echo "============================================================"
fi
echo ""
exit $TEST_RESULT
