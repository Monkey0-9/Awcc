#!/bin/bash
set -euo pipefail

echo "================================================================="
echo " AWCC Linux: Releasing Maximum Fan RPM back to Firmware (0/255)"
echo "================================================================="

AW_HWMON=""
for d in /sys/class/hwmon/hwmon*; do
    if [[ -f "$d/name" ]] && [[ "$(cat "$d/name")" == "alienware_wmi" ]]; then
        AW_HWMON="$d"
        break
    fi
done

if [[ -n "${AW_HWMON}" ]]; then
    if [[ -w "${AW_HWMON}/fan1_boost" ]]; then
        echo "0" > "${AW_HWMON}/fan1_boost"
        echo "0" > "${AW_HWMON}/fan2_boost"
    else
        sudo bash -c "echo 0 > ${AW_HWMON}/fan1_boost && echo 0 > ${AW_HWMON}/fan2_boost"
    fi
    echo "[+] Boost released to 0/255 on both fans."
fi

if [[ -w /sys/firmware/acpi/platform_profile ]]; then
    echo "balanced" > /sys/firmware/acpi/platform_profile
else
    sudo bash -c 'echo "balanced" > /sys/firmware/acpi/platform_profile' || true
fi

echo "[+] Profile restored to balanced. Fans returned to OEM BIOS curves."
echo "================================================================="
