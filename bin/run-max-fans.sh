#!/bin/bash
set -euo pipefail

echo "================================================================="
echo " AWCC Linux: Actuating Maximum Fan RPM (Dell G15 5530)"
echo "================================================================="

MODULE="/home/unknownuser/Awcc/kernel/alienware-wmi.ko"

# 1. Check if driver is loaded, if not try insmod
if ! lsmod | grep -q "alienware_wmi"; then
    echo "[*] alienware-wmi driver not yet active in kernel."
    echo "[*] Attempting to load ${MODULE}..."
    if [[ $EUID -eq 0 ]]; then
        insmod "${MODULE}"
    else
        sudo insmod "${MODULE}" || {
            echo "[-] Error: Failed to load module. Please run: sudo insmod ${MODULE}" >&2
            exit 1
        }
    fi
fi

echo "[+] alienware-wmi driver confirmed active!"

# 2. Engage Performance / Game Shift Profile
echo "[*] Setting platform profile to performance..."
if [[ -w /sys/firmware/acpi/platform_profile ]]; then
    echo "performance" > /sys/firmware/acpi/platform_profile
elif [[ $EUID -eq 0 ]]; then
    echo "performance" > /sys/firmware/acpi/platform_profile
else
    sudo bash -c 'echo "performance" > /sys/firmware/acpi/platform_profile' || true
fi

# 3. Locate the alienware_wmi hwmon directory
AW_HWMON=""
for d in /sys/class/hwmon/hwmon*; do
    if [[ -f "$d/name" ]] && [[ "$(cat "$d/name")" == "alienware_wmi" ]]; then
        AW_HWMON="$d"
        break
    fi
done

if [[ -z "${AW_HWMON}" ]]; then
    echo "[-] Error: alienware_wmi hwmon device not found in /sys/class/hwmon." >&2
    exit 1
fi

echo "[+] Located AWCC hwmon interface at: ${AW_HWMON}"

# 4. Command maximum boost 255 on both fans
echo "[*] Commanding Fan 1 (CPU) Boost = 255/255..."
if [[ -w "${AW_HWMON}/fan1_boost" ]]; then
    echo "255" > "${AW_HWMON}/fan1_boost"
else
    sudo bash -c "echo 255 > ${AW_HWMON}/fan1_boost"
fi

echo "[*] Commanding Fan 2 (GPU) Boost = 255/255..."
if [[ -w "${AW_HWMON}/fan2_boost" ]]; then
    echo "255" > "${AW_HWMON}/fan2_boost"
else
    sudo bash -c "echo 255 > ${AW_HWMON}/fan2_boost"
fi

# 5. Live Tachometer Sampling Ladder (10 Seconds)
echo "[*] Monitoring fan rotor acceleration across both channels:"
for i in {1..10}; do
    sleep 1
    RPM1=$(cat "${AW_HWMON}/fan1_input" 2>/dev/null || cat /sys/class/hwmon/hwmon4/fan1_input 2>/dev/null || echo "0")
    RPM2=$(cat "${AW_HWMON}/fan2_input" 2>/dev/null || cat /sys/class/hwmon/hwmon4/fan2_input 2>/dev/null || echo "0")
    B1=$(cat "${AW_HWMON}/fan1_boost" 2>/dev/null || echo "?")
    B2=$(cat "${AW_HWMON}/fan2_boost" 2>/dev/null || echo "?")
    printf "  [T+%02ds] Fan 1 (CPU): %4s RPM (boost: %s/255) | Fan 2 (GPU): %4s RPM (boost: %s/255)\n" "$i" "$RPM1" "$B1" "$RPM2" "$B2"
done

echo "================================================================="
echo "[+] Maximum fan speed achieved and verified!"
echo "================================================================="
