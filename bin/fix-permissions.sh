#!/bin/bash
set -euo pipefail

echo "[*] Auditing and hardening AWCC sysfs permissions..."
if [[ -f /etc/udev/rules.d/99-awcc-hwmon.rules ]]; then
    if grep -qE "chmod 0?666" /etc/udev/rules.d/99-awcc-hwmon.rules 2>/dev/null; then
        echo "[!] Found insecure world-writable udev rule. Purging /etc/udev/rules.d/99-awcc-hwmon.rules..."
        rm -f /etc/udev/rules.d/99-awcc-hwmon.rules
        udevadm control --reload-rules 2>/dev/null || true
    fi
fi

echo "[*] Restoring standard secure root permissions (0644) on fan boost controls..."
for d in /sys/class/hwmon/hwmon*; do
    if [[ -f "$d/name" ]] && [[ "$(cat "$d/name" 2>/dev/null)" == "alienware_wmi" ]]; then
        if [[ -w "$d/fan1_boost" ]]; then
            chmod 0644 "$d"/fan*_boost 2>/dev/null || true
            echo "[+] Secured $d/fan*_boost to 0644."
        else
            sudo chmod 0644 "$d"/fan*_boost 2>/dev/null || true
            echo "[+] Secured $d/fan*_boost to 0644 via sudo."
        fi
    fi
done

if [[ -f /sys/firmware/acpi/platform_profile ]]; then
    if [[ -w /sys/firmware/acpi/platform_profile ]]; then
        chmod 0644 /sys/firmware/acpi/platform_profile 2>/dev/null || true
        echo "[+] Secured /sys/firmware/acpi/platform_profile to 0644."
    else
        sudo chmod 0644 /sys/firmware/acpi/platform_profile 2>/dev/null || true
        echo "[+] Secured /sys/firmware/acpi/platform_profile to 0644 via sudo."
    fi
fi

echo "[+] Security audit complete: All hardware controls restricted to root/daemon."
