#!/bin/bash
set -euo pipefail

MODULE="/home/unknownuser/Awcc/kernel/alienware-wmi.ko"

if [[ ! -f "${MODULE}" ]]; then
    echo "[-] Error: ${MODULE} does not exist. Run 'make' in Awcc/kernel." >&2
    exit 1
fi

echo "[*] Loading AWCC Linux driver (${MODULE})..."
if lsmod | grep -q "alienware_wmi"; then
    echo "[!] alienware-wmi is already loaded."
else
    sudo insmod "${MODULE}"
    echo "[+] alienware-wmi loaded successfully."
fi

echo "[*] Inspecting kernel ring buffer (dmesg):"
dmesg | grep -i alienware | tail -n 20 || true

echo "[*] Discovering hwmon nodes:"
for d in /sys/class/hwmon/*; do
    echo "  $d: $(cat "$d/name" 2>/dev/null || echo 'unknown')"
done

echo "[+] Done. Now run 'rhel-ultra' or 'rhel-performance'."
