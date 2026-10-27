#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
MODULE="${REPO_DIR}/kernel/alienware-wmi.ko"
KVER="$(uname -r)"
DEST_DIR="/lib/modules/${KVER}/extra"

echo "=========================================================================="
echo " AWCC Linux: alienware-wmi Driver & Sudoers Setup Helper"
echo " Target Hardware: Dell G15 5530 / Alienware (Kernel ${KVER})"
echo "=========================================================================="

if [[ ! -f "${MODULE}" ]]; then
    echo "[*] Module ${MODULE} not found. Building in ${REPO_DIR}/kernel..."
    make -C "${REPO_DIR}/kernel"
fi

# Helper for running as root
as_root() {
    if [[ $EUID -eq 0 ]]; then
        "$@"
    else
        sudo "$@"
    fi
}

echo "[1/5] Loading alienware-wmi kernel module..."
if lsmod | grep -q "alienware_wmi"; then
    echo "[!] alienware-wmi is already loaded in kernel."
else
    as_root insmod "${MODULE}"
    echo "[+] alienware-wmi loaded successfully."
fi

echo "[2/5] Persisting module to /lib/modules/${KVER}/extra across reboots..."
as_root mkdir -p "${DEST_DIR}"
as_root cp -f "${MODULE}" "${DEST_DIR}/alienware-wmi.ko"
as_root depmod -a
as_root mkdir -p /etc/modules-load.d
if [[ $EUID -eq 0 ]]; then
    echo "alienware-wmi" > /etc/modules-load.d/alienware-wmi.conf
else
    echo "alienware-wmi" | sudo tee /etc/modules-load.d/alienware-wmi.conf >/dev/null
fi
echo "[+] Module configured to auto-load on boot."

echo "[3/5] Configuring passwordless sudoers rules (/etc/sudoers.d/awcc-linux)..."
if [[ -x "${REPO_DIR}/bin/setup-sudoers.sh" ]]; then
    "${REPO_DIR}/bin/setup-sudoers.sh"
fi

echo "[4/5] Synchronizing binaries to ~/.local/bin..."
if [[ -x "${REPO_DIR}/bin/sync-local-bin.sh" ]]; then
    "${REPO_DIR}/bin/sync-local-bin.sh"
fi

echo "[5/5] Discovering hardware hwmon nodes:"
for d in /sys/class/hwmon/*; do
    echo "  $d: $(cat "$d/name" 2>/dev/null || echo 'unknown')"
done

echo "=========================================================================="
echo "[+] Driver & Sudoers configured successfully!"
echo "    You can now run any mode command without password prompts:"
echo "      awcc-battery      (20% fan boost,  51/255, ~1,500 RPM)"
echo "      awcc-quiet        (40% fan boost, 102/255, ~2,200 RPM)"
echo "      awcc-balanced     (60% fan boost, 153/255, ~3,100 RPM)"
echo "      awcc-performance  (80% fan boost, 204/255, ~4,200 RPM)"
echo "      awcc-ultra       (100% max fans,  255/255, Max RPM)"
echo "=========================================================================="
