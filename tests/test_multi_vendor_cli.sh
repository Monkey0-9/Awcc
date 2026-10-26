#!/usr/bin/env bash
# ==============================================================================
# tests/test_multi_vendor_cli.sh
# End-to-end multi-vendor CLI validation using custom sysfs-root mock trees.
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
BIN="${REPO_DIR}/target/debug/awccctl"

echo "[*] Ensuring awccctl debug binary is up-to-date..."
cargo build --bin awccctl --manifest-path "${REPO_DIR}/Cargo.toml" --quiet

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
# Test 3: Acer Predator Mock Sysfs
# ------------------------------------------------------------------------------
ACER_DIR="${TMP_ROOT}/acer"
mkdir -p "${ACER_DIR}/sys/devices/virtual/dmi/id"
echo "Acer" > "${ACER_DIR}/sys/devices/virtual/dmi/id/sys_vendor"
echo "Predator Helios 16" > "${ACER_DIR}/sys/devices/virtual/dmi/id/product_name"
echo "10" > "${ACER_DIR}/sys/devices/virtual/dmi/id/chassis_type"
mkdir -p "${ACER_DIR}/sys/devices/platform/acer-wmi"
echo "0" > "${ACER_DIR}/sys/devices/platform/acer-wmi/turbo_mode"

OUTPUT=$("$BIN" --sysfs-root "${ACER_DIR}" detect)
if echo "$OUTPUT" | grep -q "Acer" && echo "$OUTPUT" | grep -q "acer_wmi"; then
    assert_test "Acer Mock: detect reports Acer vendor and acer_wmi provider" 0
else
    assert_test "Acer Mock: detect reports Acer vendor and acer_wmi provider" 1
fi

"$BIN" --sysfs-root "${ACER_DIR}" fan max >/dev/null 2>&1 || true
if grep -q "1" "${ACER_DIR}/sys/devices/platform/acer-wmi/turbo_mode"; then
    assert_test "Acer Mock: fan max sets turbo_mode = 1" 0
else
    assert_test "Acer Mock: fan max sets turbo_mode = 1" 1
fi

"$BIN" --sysfs-root "${ACER_DIR}" fan release >/dev/null 2>&1 || true
if grep -q "0" "${ACER_DIR}/sys/devices/platform/acer-wmi/turbo_mode"; then
    assert_test "Acer Mock: fan release restores turbo_mode = 0" 0
else
    assert_test "Acer Mock: fan release restores turbo_mode = 0" 1
fi

# ------------------------------------------------------------------------------
# Test 4: MSI Gaming Katana Mock Sysfs
# ------------------------------------------------------------------------------
MSI_DIR="${TMP_ROOT}/msi"
mkdir -p "${MSI_DIR}/sys/devices/virtual/dmi/id"
echo "Micro-Star International Co., Ltd." > "${MSI_DIR}/sys/devices/virtual/dmi/id/sys_vendor"
echo "Katana 15 B13VGK" > "${MSI_DIR}/sys/devices/virtual/dmi/id/product_name"
echo "10" > "${MSI_DIR}/sys/devices/virtual/dmi/id/chassis_type"
mkdir -p "${MSI_DIR}/sys/devices/platform/msi-ec"
echo "0" > "${MSI_DIR}/sys/devices/platform/msi-ec/cooler_boost"

OUTPUT=$("$BIN" --sysfs-root "${MSI_DIR}" detect)
if echo "$OUTPUT" | grep -q "Micro-Star" && echo "$OUTPUT" | grep -q "msi_ec"; then
    assert_test "MSI Mock: detect reports MSI vendor and msi_ec provider" 0
else
    assert_test "MSI Mock: detect reports MSI vendor and msi_ec provider" 1
fi

"$BIN" --sysfs-root "${MSI_DIR}" fan max >/dev/null 2>&1 || true
if grep -q "1" "${MSI_DIR}/sys/devices/platform/msi-ec/cooler_boost"; then
    assert_test "MSI Mock: fan max sets cooler_boost = 1" 0
else
    assert_test "MSI Mock: fan max sets cooler_boost = 1" 1
fi

"$BIN" --sysfs-root "${MSI_DIR}" fan release >/dev/null 2>&1 || true
if grep -q "0" "${MSI_DIR}/sys/devices/platform/msi-ec/cooler_boost"; then
    assert_test "MSI Mock: fan release restores cooler_boost = 0" 0
else
    assert_test "MSI Mock: fan release restores cooler_boost = 0" 1
fi

# ------------------------------------------------------------------------------
# Test 5: System76 Oryx Pro Mock Sysfs
# ------------------------------------------------------------------------------
SYS76_DIR="${TMP_ROOT}/sys76"
mkdir -p "${SYS76_DIR}/sys/devices/virtual/dmi/id"
echo "System76" > "${SYS76_DIR}/sys/devices/virtual/dmi/id/sys_vendor"
echo "Oryx Pro" > "${SYS76_DIR}/sys/devices/virtual/dmi/id/product_name"
echo "10" > "${SYS76_DIR}/sys/devices/virtual/dmi/id/chassis_type"
mkdir -p "${SYS76_DIR}/sys/devices/platform/system76_acpi"
echo "auto" > "${SYS76_DIR}/sys/devices/platform/system76_acpi/fan_mode"

OUTPUT=$("$BIN" --sysfs-root "${SYS76_DIR}" detect)
if echo "$OUTPUT" | grep -q "System76" && echo "$OUTPUT" | grep -q "system76_acpi"; then
    assert_test "System76 Mock: detect reports System76 vendor and system76_acpi provider" 0
else
    assert_test "System76 Mock: detect reports System76 vendor and system76_acpi provider" 1
fi

"$BIN" --sysfs-root "${SYS76_DIR}" fan max >/dev/null 2>&1 || true
if grep -q "full" "${SYS76_DIR}/sys/devices/platform/system76_acpi/fan_mode"; then
    assert_test "System76 Mock: fan max sets fan_mode = full" 0
else
    assert_test "System76 Mock: fan max sets fan_mode = full" 1
fi

"$BIN" --sysfs-root "${SYS76_DIR}" fan release >/dev/null 2>&1 || true
if grep -q "auto" "${SYS76_DIR}/sys/devices/platform/system76_acpi/fan_mode"; then
    assert_test "System76 Mock: fan release restores fan_mode = auto" 0
else
    assert_test "System76 Mock: fan release restores fan_mode = auto" 1
fi

# ------------------------------------------------------------------------------
# Test 6: Generic Custom Desktop PC Mock Sysfs
# ------------------------------------------------------------------------------
GEN_DIR="${TMP_ROOT}/generic"
mkdir -p "${GEN_DIR}/sys/devices/virtual/dmi/id"
echo "Custom Desktop PC" > "${GEN_DIR}/sys/devices/virtual/dmi/id/sys_vendor"
echo "Z790 Motherboard" > "${GEN_DIR}/sys/devices/virtual/dmi/id/product_name"
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
# Test 7: Unsupported PC Graceful Degradation
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
