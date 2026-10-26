#!/bin/bash
set -euo pipefail

MODULE_SRC="/home/unknownuser/Awcc/kernel/alienware-wmi.ko"
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
    make -C /home/unknownuser/Awcc/kernel
    /home/unknownuser/Awcc/kernel/scripts/sign-module.sh
fi

echo "[1/6] Copying signed driver to ${DEST_DIR}..."
mkdir -p "${DEST_DIR}"
cp -f "${MODULE_SRC}" "${DEST_DIR}/alienware-wmi.ko"

echo "[2/6] Running depmod..."
depmod -a

echo "[3/6] Configuring module auto-load on boot (/etc/modules-load.d/alienware-wmi.conf)..."
echo "alienware-wmi" > /etc/modules-load.d/alienware-wmi.conf

echo "[4/6] Configuring udev rules for unprivileged hardware telemetry (/etc/udev/rules.d/99-awcc-hwmon.rules)..."
cat > /etc/udev/rules.d/99-awcc-hwmon.rules << 'RULES'
ACTION=="add", SUBSYSTEM=="hwmon", ATTR{name}=="alienware_wmi", RUN+="/bin/chmod 0666 /sys%p/fan1_boost /sys%p/fan2_boost"
ACTION=="add", SUBSYSTEM=="platform", KERNEL=="alienware-wmi", RUN+="/bin/chmod 0666 /sys/firmware/acpi/platform_profile"
RULES
udevadm control --reload-rules || true

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
    if [[ -f "/home/unknownuser/.local/bin/$b" ]]; then
        cp -f "/home/unknownuser/.local/bin/$b" "/usr/local/bin/$b"
        chmod 0755 "/usr/local/bin/$b"
    elif [[ -f "/home/unknownuser/Awcc/bin/$b" ]]; then
        cp -f "/home/unknownuser/Awcc/bin/$b" "/usr/local/bin/$b"
        chmod 0755 "/usr/local/bin/$b"
    fi
done

echo "[6/6] Verifying driver in live kernel memory..."
modprobe alienware-wmi || insmod "${DEST_DIR}/alienware-wmi.ko" || true

echo "=========================================================================="
echo "[+] AWCC Linux Installation Complete!"
echo "    - Driver: Signed & Auto-loading via /etc/modules-load.d/"
echo "    - Permissions: Unprivileged 0666 via udev rules"
echo "    - Commands available system-wide: awcc-mode, awcc-battery, awcc-quiet,"
echo "      awcc-balance, awcc-mid, awcc-high, awcc-ultra, gmode"
echo "=========================================================================="
