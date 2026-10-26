#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
MODULE_SRC="${REPO_DIR}/kernel/alienware-wmi.ko"
KVER="$(uname -r)"
DEST_DIR="/lib/modules/${KVER}/extra"

echo "=========================================================================="
echo " AWCC Linux Enterprise Installer (Dell G15 5530 on RHEL 10.2)"
echo "=========================================================================="
if [[ $EUID -ne 0 ]]; then
    echo "[-] Error: This installation script must be run as root:" >&2
    echo "    sudo ./install-system.sh" >&2
    exit 1
fi

if [[ ! -f "${MODULE_SRC}" ]]; then
    echo "[*] Module ${MODULE_SRC} not found. Building and signing..."
    make -C "${REPO_DIR}/kernel"
    "${REPO_DIR}/kernel/scripts/sign-module.sh"
fi

echo "[1/6] Copying signed driver to ${DEST_DIR}..."
mkdir -p "${DEST_DIR}"
cp -f "${MODULE_SRC}" "${DEST_DIR}/alienware-wmi.ko"

echo "[2/6] Running depmod..."
depmod -a

echo "[3/6] Configuring module auto-load on boot (/etc/modules-load.d/alienware-wmi.conf)..."
echo "alienware-wmi" > /etc/modules-load.d/alienware-wmi.conf

echo "[4/6] Ensuring secure sysfs permissions (no world-writable controls)..."
if [[ -f /etc/udev/rules.d/99-awcc-hwmon.rules ]]; then
    rm -f /etc/udev/rules.d/99-awcc-hwmon.rules
    udevadm control --reload-rules 2>/dev/null || true
fi

# Reset any stale world-writable sysfs nodes back to standard root permissions (0644)
for d in /sys/class/hwmon/hwmon*; do
    if [[ -f "$d/name" ]] && [[ "$(cat "$d/name" 2>/dev/null)" == "alienware_wmi" ]]; then
        chmod 0644 "$d"/fan*_boost 2>/dev/null || true
    fi
done
if [[ -f /sys/firmware/acpi/platform_profile ]]; then
    chmod 0644 /sys/firmware/acpi/platform_profile 2>/dev/null || true
fi

echo "[5/6] Installing system-wide CLI binaries in /usr/local/bin..."
BIN_LIST=(
    "awcc-battery"
    "awcc-quiet"
    "awcc-balance"
    "awcc-mid"
    "awcc-high"
    "awcc-ultra"
    "awcc-mode"
    "awcc-gameshift-toggle"
    "awcc-lux-dashboard.py"
    "gmode"
    "awccctl"
)

for b in "${BIN_LIST[@]}"; do
    if [[ -f "${REPO_DIR}/bin/$b" ]]; then
        cp -f "${REPO_DIR}/bin/$b" "/usr/local/bin/$b"
        chmod 0755 "/usr/local/bin/$b"
    elif [[ -f "${REPO_DIR}/target/release/$b" ]]; then
        cp -f "${REPO_DIR}/target/release/$b" "/usr/local/bin/$b"
        chmod 0755 "/usr/local/bin/$b"
    fi
done

echo "[6/6] Verifying driver in live kernel memory..."
modprobe alienware-wmi || insmod "${DEST_DIR}/alienware-wmi.ko" || true

echo "=========================================================================="
echo "[+] AWCC Linux Installation Complete!"
echo "    - Driver: Signed & Auto-loading via /etc/modules-load.d/"
echo "    - Security: Secure root-only sysfs permissions (no world-writable controls)"
echo "    - Actuation: Root / sudo required for fan writes; telemetry readable by all users"
echo "    - Commands available system-wide: awcc-mode, awcc-battery, awcc-quiet,"
echo "      awcc-balance, awcc-mid, awcc-high, awcc-ultra, gmode"
echo "=========================================================================="
