#!/usr/bin/env bash
# ==============================================================================
# tests/test_multi_vendor_cli.sh
# End-to-end multi-vendor CLI validation using custom sysfs-root mock trees.
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
BIN="${REPO_DIR}/target/debug/awccctl"

if [[ ! -x "$BIN" ]]; then
    echo "[*] Building awccctl binary..."
    cargo build --bin awccctl --manifest-path "${REPO_DIR}/Cargo.toml"
fi

TMP_ROOT="$(mktemp -d)"
trap 'rm -rf "${TMP_ROOT}"' EXIT

PASSED=0
FAILED=0

assert_test() {
    local desc="$1"
    local status="$2"
    if [[ "$status" -eq 0 ]]; then
        echo -e "\e[32m[PASS]\e[0m ${desc}"
        PASSED=$((PASSED + 1))
    else
        echo -e "\e[31m[FAIL]\e[0m ${desc}"
        FAILED=$((FAILED + 1))
    fi
}

echo "=== Running Multi-Vendor CLI Test Suite ==="

# ------------------------------------------------------------------------------
# Test 1: ASUS ROG Zephyrus G14 Mock Sysfs
# ------------------------------------------------------------------------------
ASUS_DIR="${TMP_ROOT}/asus"
mkdir -p "${ASUS_DIR}/sys/devices/virtual/dmi/id"
echo "ASUSTeK COMPUTER INC." > "${ASUS_DIR}/sys/devices/virtual/dmi/id/sys_vendor"
echo "ROG Zephyrus G14" > "${ASUS_DIR}/sys/devices/virtual/dmi/id/product_name"
echo "10" > "${ASUS_DIR}/sys/devices/virtual/dmi/id/chassis_type"
mkdir -p "${ASUS_DIR}/sys/devices/platform/asus-nb-wmi"
echo "0" > "${ASUS_DIR}/sys/devices/platform/asus-nb-wmi/throttle_thermal_policy"
mkdir -p "${ASUS_DIR}/sys/class/platform-profile"
mkdir -p "${ASUS_DIR}/sys/class/platform-profile/platform-profile-0"
echo "asus-wmi" > "${ASUS_DIR}/sys/class/platform-profile/platform-profile-0/name"
echo "balanced" > "${ASUS_DIR}/sys/class/platform-profile/platform-profile-0/profile"
echo "quiet balanced performance" > "${ASUS_DIR}/sys/class/platform-profile/platform-profile-0/choices"

OUTPUT=$("$BIN" --sysfs-root "${ASUS_DIR}" detect)
if echo "$OUTPUT" | grep -q "ASUSTeK" && echo "$OUTPUT" | grep -q "asus_wmi"; then
    assert_test "ASUS Mock: detect reports Asus vendor and asus_wmi provider" 0
else
    assert_test "ASUS Mock: detect reports Asus vendor and asus_wmi provider" 1
fi

CAPS_JSON=$("$BIN" --sysfs-root "${ASUS_DIR}" capabilities --json)
if echo "$CAPS_JSON" | grep -q '"vendor": "Asus"' && echo "$CAPS_JSON" | grep -q '"active_provider_name": "asus_wmi"'; then
    assert_test "ASUS Mock: capabilities JSON has valid vendor and provider" 0
else
    assert_test "ASUS Mock: capabilities JSON has valid vendor and provider" 1
fi

# ------------------------------------------------------------------------------
# Test 2: Lenovo ThinkPad X1 Carbon Mock Sysfs
# ------------------------------------------------------------------------------
LENOVO_DIR="${TMP_ROOT}/lenovo"
mkdir -p "${LENOVO_DIR}/sys/devices/virtual/dmi/id"
echo "LENOVO" > "${LENOVO_DIR}/sys/devices/virtual/dmi/id/sys_vendor"
echo "ThinkPad X1 Carbon Gen 11" > "${LENOVO_DIR}/sys/devices/virtual/dmi/id/product_name"
echo "10" > "${LENOVO_DIR}/sys/devices/virtual/dmi/id/chassis_type"
mkdir -p "${LENOVO_DIR}/proc/acpi/ibm"
echo -e "status: enabled\nspeed: 2400\nlevel: auto" > "${LENOVO_DIR}/proc/acpi/ibm/fan"

OUTPUT=$("$BIN" --sysfs-root "${LENOVO_DIR}" detect)
if echo "$OUTPUT" | grep -q "LENOVO" && echo "$OUTPUT" | grep -q "lenovo_thinkpad"; then
    assert_test "Lenovo Mock: detect reports Lenovo vendor and lenovo_thinkpad provider" 0
else
    assert_test "Lenovo Mock: detect reports Lenovo vendor and lenovo_thinkpad provider" 1
fi

"$BIN" --sysfs-root "${LENOVO_DIR}" fan max >/dev/null 2>&1 || true
if grep -q "disengaged" "${LENOVO_DIR}/proc/acpi/ibm/fan"; then
    assert_test "Lenovo Mock: fan max disengages fans via ibm/fan" 0
else
    assert_test "Lenovo Mock: fan max disengages fans via ibm/fan" 1
fi

"$BIN" --sysfs-root "${LENOVO_DIR}" fan release >/dev/null 2>&1 || true
if grep -q "auto" "${LENOVO_DIR}/proc/acpi/ibm/fan"; then
    assert_test "Lenovo Mock: fan release restores auto level via ibm/fan" 0
else
    assert_test "Lenovo Mock: fan release restores auto level via ibm/fan" 1
fi

# ------------------------------------------------------------------------------
# Test 3: Generic Desktop PC Mock Sysfs
# ------------------------------------------------------------------------------
GEN_DIR="${TMP_ROOT}/generic"
mkdir -p "${GEN_DIR}/sys/devices/virtual/dmi/id"
echo "Micro-Star International Co., Ltd." > "${GEN_DIR}/sys/devices/virtual/dmi/id/sys_vendor"
echo "MS-7D91" > "${GEN_DIR}/sys/devices/virtual/dmi/id/product_name"
echo "3" > "${GEN_DIR}/sys/devices/virtual/dmi/id/chassis_type"
mkdir -p "${GEN_DIR}/sys/class/hwmon/hwmon0"
echo "nct6775" > "${GEN_DIR}/sys/class/hwmon/hwmon0/name"
echo "1200" > "${GEN_DIR}/sys/class/hwmon/hwmon0/fan1_input"
echo "150" > "${GEN_DIR}/sys/class/hwmon/hwmon0/pwm1"
echo "1" > "${GEN_DIR}/sys/class/hwmon/hwmon0/pwm1_enable"

STATUS_JSON=$("$BIN" --sysfs-root "${GEN_DIR}" status --json)
if echo "$STATUS_JSON" | grep -q '"active_platform_profile"'; then
    assert_test "Generic Desktop Mock: status JSON produces valid schema" 0
else
    assert_test "Generic Desktop Mock: status JSON produces valid schema" 1
fi

# ------------------------------------------------------------------------------
# Test 4: Unsupported PC Graceful Degradation
# ------------------------------------------------------------------------------
UNSUP_DIR="${TMP_ROOT}/unsupported"
mkdir -p "${UNSUP_DIR}/sys/devices/virtual/dmi/id"
echo "Unknown OEM" > "${UNSUP_DIR}/sys/devices/virtual/dmi/id/sys_vendor"
echo "Barebone Box" > "${UNSUP_DIR}/sys/devices/virtual/dmi/id/product_name"

set +e
ERR_OUT=$("$BIN" --sysfs-root "${UNSUP_DIR}" fan set --fan 1 --boost 100 2>&1)
ERR_CODE=$?
set -e

if [[ "$ERR_CODE" -ne 0 ]] && echo "$ERR_OUT" | grep -qi "Manual fan control is not supported"; then
    assert_test "Unsupported Mock: fan set fails safely with descriptive error" 0
else
    assert_test "Unsupported Mock: fan set fails safely with descriptive error" 1
fi

set +e
ERR_MAX_OUT=$("$BIN" --sysfs-root "${UNSUP_DIR}" fan max 2>&1)
ERR_MAX_CODE=$?
set -e

if [[ "$ERR_MAX_CODE" -ne 0 ]] && echo "$ERR_MAX_OUT" | grep -qi "Maximum fan boost is not supported"; then
    assert_test "Unsupported Mock: fan max fails safely with descriptive error" 0
else
    assert_test "Unsupported Mock: fan max fails safely with descriptive error" 1
fi

echo "=== Summary: ${PASSED} Passed, ${FAILED} Failed ==="
if [[ "$FAILED" -gt 0 ]]; then
    exit 1
fi
