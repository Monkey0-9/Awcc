#!/bin/bash
set -euo pipefail

echo "================================================================="
echo " AWCC Linux: Releasing Maximum Fan RPM back to Firmware (0/255)"
echo "================================================================="

AW_HWMON=""
for d in /sys/class/hwmon/hwmon*; do
    if [[ -f "$d/name" ]] && [[ "$(cat "$d/name" 2>/dev/null)" == "alienware_wmi" ]]; then
        AW_HWMON="$d"
        break
    fi
done

if [[ -z "${AW_HWMON}" ]]; then
    echo "[-] Error: alienware_wmi device not found in /sys/class/hwmon." >&2
    exit 1
fi

if [[ -w "${AW_HWMON}/fan1_boost" ]]; then
    echo "0" > "${AW_HWMON}/fan1_boost"
    echo "0" > "${AW_HWMON}/fan2_boost"
elif command -v sudo >/dev/null 2>&1; then
    sudo bash -c "echo 0 > '${AW_HWMON}/fan1_boost' && echo 0 > '${AW_HWMON}/fan2_boost'"
else
    echo "[-] Error: Writing to fan boost requires root/sudo." >&2
    exit 1
fi

B1="$(cat "${AW_HWMON}/fan1_boost" 2>/dev/null || echo '?')"
B2="$(cat "${AW_HWMON}/fan2_boost" 2>/dev/null || echo '?')"

if [[ "$B1" -ne 0 ]] || [[ "$B2" -ne 0 ]]; then
    echo "[-] Error: Failed to release fan boost (Readback: Fan 1 = $B1, Fan 2 = $B2)." >&2
    exit 1
fi
echo "[+] Boost release confirmed: Fan 1 = 0/255 | Fan 2 = 0/255."

# Restore platform profile
PPROF=""
for p in /sys/class/platform-profile/platform-profile-*; do
    if [[ -f "$p/name" ]] && [[ "$(cat "$p/name" 2>/dev/null)" == "alienware-wmi" ]]; then
        PPROF="$p/profile"
        break
    fi
done
if [[ -z "$PPROF" ]] && [[ -f /sys/firmware/acpi/platform_profile ]]; then
    PPROF="/sys/firmware/acpi/platform_profile"
fi

if [[ -n "$PPROF" ]]; then
    if [[ -w "$PPROF" ]]; then
        echo "balanced" > "$PPROF"
    elif command -v sudo >/dev/null 2>&1; then
        echo "balanced" | sudo tee "$PPROF" >/dev/null || true
    fi
    echo "[+] Profile restored to balanced ($(cat "$PPROF" 2>/dev/null || echo 'unknown'))."
fi

echo "[+] Profile restored to balanced. Fans returned to OEM BIOS curves."
echo "================================================================="
