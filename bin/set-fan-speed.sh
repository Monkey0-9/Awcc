#!/bin/bash
set -euo pipefail

TARGET_BOOST="${1:-0}"

if ! [[ "${TARGET_BOOST}" =~ ^[0-9]+$ ]] || [ "${TARGET_BOOST}" -lt 0 ] || [ "${TARGET_BOOST}" -gt 255 ]; then
    echo "[-] Error: Boost value must be an integer between 0 and 255 (got '${TARGET_BOOST}')." >&2
    exit 1
fi

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
else
    sudo bash -c "echo ${TARGET_BOOST} > ${AW_HWMON}/fan1_boost && echo ${TARGET_BOOST} > ${AW_HWMON}/fan2_boost"
fi

echo "[+] Current Boost: $(cat "${AW_HWMON}/fan1_boost")/255"
echo "[*] Monitoring deceleration:"
for i in {1..5}; do
    sleep 1
    R1=$(cat "${AW_HWMON}/fan1_input" 2>/dev/null || echo "0")
    R2=$(cat "${AW_HWMON}/fan2_input" 2>/dev/null || echo "0")
    printf "  T+%ds: Fan 1: %4s RPM | Fan 2: %4s RPM\n" "$i" "$R1" "$R2"
done
