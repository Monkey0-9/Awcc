#!/usr/bin/env bash
set -euo pipefail

PROFILE="${1:-balanced}"
BOOST="${2:-0}"
TITLE="${3:-BALANCED}"

echo "================================================================="
echo " AWCC Linux: Activating ${TITLE} Mode"
echo " Target Machine: Dell G15 5530 (RHEL 10.2)"
echo "================================================================="

echo "[1/3] Setting Platform Profile to '${PROFILE}'..."
if [[ -w /sys/firmware/acpi/platform_profile ]]; then
    echo "${PROFILE}" > /sys/firmware/acpi/platform_profile
else
    echo "Notice: Non-root mode, current profile: $(cat /sys/firmware/acpi/platform_profile 2>/dev/null || echo 'unknown')"
fi

echo "[2/3] Setting Fan Boost to ${BOOST}/255..."
AW_HWMON=""
for d in /sys/class/hwmon/hwmon*; do
    if [[ -f "$d/name" ]] && [[ "$(cat "$d/name" 2>/dev/null)" == "alienware_wmi" ]]; then
        AW_HWMON="$d"
        break
    fi
done

if [[ -n "${AW_HWMON}" ]] && [[ -w "${AW_HWMON}/fan1_boost" ]]; then
    echo "${BOOST}" > "${AW_HWMON}/fan1_boost"
    echo "${BOOST}" > "${AW_HWMON}/fan2_boost"
    echo "[+] Boost ${BOOST}/255 written successfully to hardware."
else
    if [[ "${BOOST}" -eq 0 ]]; then
        awccctl fan release 2>/dev/null || true
    elif [[ "${BOOST}" -eq 255 ]]; then
        awccctl fan max 2>/dev/null || true
    else
        awccctl fan set --fan 1 --boost "${BOOST}" 2>/dev/null || true
        awccctl fan set --fan 2 --boost "${BOOST}" 2>/dev/null || true
    fi
fi

echo "[3/3] Current System & Thermal Status:"
awccctl status

echo "================================================================="
echo "[+] ${TITLE} Mode orchestration complete."
echo "================================================================="
