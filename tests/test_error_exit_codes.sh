#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== AWCC Linux: Error Propagation and Exit Code Verification ==="

AWCCCTL="${REPO_DIR}/target/release/awccctl"
if [[ ! -x "${AWCCCTL}" ]]; then
    AWCCCTL="${REPO_DIR}/target/debug/awccctl"
fi

if [[ ! -x "${AWCCCTL}" ]]; then
    echo "[*] Building awccctl..."
    cargo build --bin awccctl --manifest-path "${REPO_DIR}/Cargo.toml"
    AWCCCTL="${REPO_DIR}/target/debug/awccctl"
fi

TESTS_PASSED=0

# 1. Invalid fan index out of range (> 6)
echo "[Test 1] Testing invalid fan index (99)..."
if "${AWCCCTL}" fan set --fan 99 --boost 100 2>/dev/null; then
    echo "FAIL: awccctl exited with 0 on invalid fan index 99!" >&2
    exit 1
fi
echo "  PASS: Non-zero exit code returned for fan index 99."
TESTS_PASSED=$(( TESTS_PASSED + 1 ))

# 2. Invalid fan index out of range (0)
echo "[Test 2] Testing invalid fan index (0)..."
if "${AWCCCTL}" fan set --fan 0 --boost 100 2>/dev/null; then
    echo "FAIL: awccctl exited with 0 on invalid fan index 0!" >&2
    exit 1
fi
echo "  PASS: Non-zero exit code returned for fan index 0."
TESTS_PASSED=$(( TESTS_PASSED + 1 ))

# 3. Invalid platform profile
echo "[Test 3] Testing invalid platform profile name..."
if "${AWCCCTL}" profile "nonexistent_mode_xyz" 2>/dev/null; then
    echo "FAIL: awccctl exited with 0 on invalid profile!" >&2
    exit 1
fi
echo "  PASS: Non-zero exit code returned for invalid profile."
TESTS_PASSED=$(( TESTS_PASSED + 1 ))

# 4. Unknown subcommand
echo "[Test 4] Testing unknown CLI subcommand..."
if "${AWCCCTL}" invalid_subcommand 2>/dev/null; then
    echo "FAIL: awccctl exited with 0 on unknown subcommand!" >&2
    exit 1
fi
echo "  PASS: Non-zero exit code returned for unknown subcommand."
TESTS_PASSED=$(( TESTS_PASSED + 1 ))

# 5. Invalid boost input in set-fan-speed.sh (> 255)
echo "[Test 5] Testing set-fan-speed.sh with boost 999..."
if "${REPO_DIR}/bin/set-fan-speed.sh" 999 2>/dev/null; then
    echo "FAIL: set-fan-speed.sh exited with 0 on out-of-range boost!" >&2
    exit 1
fi
echo "  PASS: Non-zero exit code returned for out-of-range boost."
TESTS_PASSED=$(( TESTS_PASSED + 1 ))

# 6. Invalid mode in awcc-mode
echo "[Test 6] Testing awcc-mode with unknown mode..."
if "${REPO_DIR}/bin/awcc-mode" "invalid_nonexistent_mode" 2>/dev/null; then
    echo "FAIL: awcc-mode exited with 0 on unknown mode!" >&2
    exit 1
fi
echo "  PASS: Non-zero exit code returned for unknown mode."
TESTS_PASSED=$(( TESTS_PASSED + 1 ))

echo "================================================================"
echo "PASS: All ${TESTS_PASSED} error exit code assertions succeeded!"
