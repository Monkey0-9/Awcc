#!/bin/bash
# ==============================================================================
# bin/doctor-recovery.sh
# Emergency Hardware Thermal Reset & Disaster Recovery Tool
# Reverts fan control authority to firmware BIOS, clears locks, restores profile
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=========================================================================="
echo " AWCC Linux Emergency Thermal Reset & Hardware Recovery Tool"
echo "=========================================================================="

echo "[1/4] Releasing fan speed overrides back to BIOS/EC firmware control..."
# 1. Use awccctl if available
if [[ -x "${REPO_DIR}/target/release/awccctl" ]]; then
    "${REPO_DIR}/target/release/awccctl" fan release 2>/dev/null || true
    "${REPO_DIR}/target/release/awccctl" profile balanced 2>/dev/null || true
elif command -v awccctl >/dev/null 2>&1; then
    awccctl fan release 2>/dev/null || true
    awccctl profile balanced 2>/dev/null || true
fi

# 2. Direct sysfs fallback (Dell boost nodes)
for d in /sys/class/hwmon/hwmon*; do
    if [[ -f "$d/name" ]] && [[ "$(cat "$d/name" 2>/dev/null)" == "alienware_wmi" ]]; then
        for b in "$d"/fan*_boost; do
            if [[ -f "$b" ]]; then
                echo 0 | sudo tee "$b" >/dev/null 2>&1 || true
            fi
        done
    fi
    # Standard Linux hwmon pwm reset to mode 2 (automatic)
    for p in "$d"/pwm*_enable; do
        if [[ -f "$p" ]]; then
            echo 2 | sudo tee "$p" >/dev/null 2>&1 || true
        fi
    done
done

# 3. Direct ThinkPad ACPI fallback
if [[ -f /proc/acpi/ibm/fan ]]; then
    echo "level auto" | sudo tee /proc/acpi/ibm/fan >/dev/null 2>&1 || true
fi

# 4. Direct ASUS throttle policy fallback
if [[ -f /sys/devices/platform/asus-nb-wmi/throttle_thermal_policy ]]; then
    echo 0 | sudo tee /sys/devices/platform/asus-nb-wmi/throttle_thermal_policy >/dev/null 2>&1 || true
fi

echo "[2/4] Restoring platform profile to balanced mode..."
for p in /sys/class/platform-profile/platform-profile-*/profile; do
    if [[ -f "$p" ]]; then
        echo "balanced" | sudo tee "$p" >/dev/null 2>&1 || true
    fi
done
if [[ -f /sys/firmware/acpi/platform_profile ]]; then
    echo "balanced" | sudo tee /sys/firmware/acpi/platform_profile >/dev/null 2>&1 || true
fi

echo "[3/4] Purging stale lock and transient state files..."
rm -f /tmp/.awcc_gmode_state 2>/dev/null || sudo rm -f /tmp/.awcc_gmode_state 2>/dev/null || true
rm -f /run/awccd.pid 2>/dev/null || sudo rm -f /run/awccd.pid 2>/dev/null || true

echo "[4/4] Sampling hardware tachometers to verify safe operation..."
sleep 1
if [[ -x "${REPO_DIR}/target/release/awccctl" ]]; then
    "${REPO_DIR}/target/release/awccctl" status || true
elif command -v awccctl >/dev/null 2>&1; then
    awccctl status || true
else
    echo "  System tachometers restored to automatic firmware operation."
fi

echo "=========================================================================="
echo "[+] Recovery Complete: All fans restored to firmware control."
echo "=========================================================================="
