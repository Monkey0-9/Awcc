#!/bin/bash
set -euo pipefail

echo "[*] Granting user write permissions on AWCC boost attributes..."
for d in /sys/class/hwmon/hwmon*; do
    if [[ -f "$d/name" ]] && [[ "$(cat "$d/name" 2>/dev/null)" == "alienware_wmi" ]]; then
        chmod 0666 "$d"/fan*_boost || true
        echo "[+] Made $d/fan*_boost writable without sudo."
    fi
done

if [[ -f /sys/firmware/acpi/platform_profile ]]; then
    chmod 0666 /sys/firmware/acpi/platform_profile || true
    echo "[+] Made /sys/firmware/acpi/platform_profile writable without sudo."
fi
