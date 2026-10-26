#!/bin/bash
set -euo pipefail

TARGET_BOOST="${1:-0}"

AW_HWMON=""
for d in /sys/class/hwmon/hwmon*; do
    if [[ -f "$d/name" ]] && [[ "$(cat "$d/name" 2>/dev/null)" == "alienware_wmi" ]]; then
        AW_HWMON="$d"
        break
    fi
done

if [[ -z "${AW_HWMON}" ]]; then
    echo "[-] Error: alienware_wmi device not found." >&2
    exit 1
fi

echo "[*] Setting fan boost to ${TARGET_BOOST}/255..."
if [[ $EUID -eq 0 ]]; then
    echo "${TARGET_BOOST}" > "${AW_HWMON}/fan1_boost"
    echo "${TARGET_BOOST}" > "${AW_HWMON}/fan2_boost"
    chmod 0666 "${AW_HWMON}"/fan*_boost /sys/firmware/acpi/platform_profile 2>/dev/null || true
else
    sudo bash -c "echo ${TARGET_BOOST} > ${AW_HWMON}/fan1_boost && echo ${TARGET_BOOST} > ${AW_HWMON}/fan2_boost && chmod 0666 ${AW_HWMON}/fan*_boost /sys/firmware/acpi/platform_profile"
fi

echo "[+] Current Boost: $(cat "${AW_HWMON}/fan1_boost")/255"
echo "[*] Monitoring deceleration:"
for i in {1..5}; do
    sleep 1
    R1=$(cat "${AW_HWMON}/fan1_input" 2>/dev/null || cat /sys/class/hwmon/hwmon4/fan1_input 2>/dev/null || echo "0")
    R2=$(cat "${AW_HWMON}/fan2_input" 2>/dev/null || cat /sys/class/hwmon/hwmon4/fan2_input 2>/dev/null || echo "0")
    printf "  T+%ds: Fan 1: %4s RPM | Fan 2: %4s RPM\n" "$i" "$R1" "$R2"
done
