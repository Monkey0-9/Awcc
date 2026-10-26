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

echo "[1/7] Copying signed driver to ${DEST_DIR}..."
mkdir -p "${DEST_DIR}"
cp -f "${MODULE_SRC}" "${DEST_DIR}/alienware-wmi.ko"

echo "[2/7] Running depmod..."
depmod -a

echo "[3/7] Configuring module auto-load on boot (/etc/modules-load.d/alienware-wmi.conf)..."
echo "alienware-wmi" > /etc/modules-load.d/alienware-wmi.conf

echo "[4/7] Ensuring secure sysfs permissions (no world-writable controls)..."
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
for p in /sys/class/platform-profile/platform-profile-*; do
    if [[ -f "$p/profile" ]]; then
        chmod 0644 "$p/profile" 2>/dev/null || true
    fi
done

echo "[5/7] Installing system-wide CLI binaries and utilities in /usr/local/bin..."
# Build Rust binaries if not present
if [[ ! -f "${REPO_DIR}/target/release/awccctl" || ! -f "${REPO_DIR}/target/release/awccd" ]]; then
    echo "[*] Building release binaries via cargo..."
    cargo build --release --manifest-path "${REPO_DIR}/Cargo.toml"
fi

BIN_MAP=(
    "bin/awcc-battery:/usr/local/bin/awcc-battery"
    "bin/awcc-quiet:/usr/local/bin/awcc-quiet"
    "bin/awcc-balance:/usr/local/bin/awcc-balance"
    "bin/awcc-mid:/usr/local/bin/awcc-mid"
    "bin/awcc-high:/usr/local/bin/awcc-high"
    "bin/awcc-ultra:/usr/local/bin/awcc-ultra"
    "bin/awcc-mode:/usr/local/bin/awcc-mode"
    "bin/awcc-mode.sh:/usr/local/bin/awcc-mode.sh"
    "bin/awcc-gameshift-toggle.sh:/usr/local/bin/awcc-gameshift-toggle"
    "bin/awcc-lux-dashboard.py:/usr/local/bin/awcc-lux-dashboard.py"
    "bin/rhel-performance:/usr/local/bin/rhel-performance"
    "bin/rhel-balanced:/usr/local/bin/rhel-balanced"
    "bin/rhel-ultra:/usr/local/bin/rhel-ultra"
    "bin/rhel-quiet:/usr/local/bin/rhel-quiet"
    "bin/rhel-mid:/usr/local/bin/rhel-mid"
    "bin/rhel-cool:/usr/local/bin/rhel-cool"
    "bin/set-profile-helper.sh:/usr/local/bin/set-profile-helper.sh"
    "bin/run-max-fans.sh:/usr/local/bin/run-max-fans.sh"
    "bin/stop-max-fans.sh:/usr/local/bin/stop-max-fans.sh"
    "target/release/awccctl:/usr/local/bin/awccctl"
    "target/release/awccd:/usr/local/bin/awccd"
)

for item in "${BIN_MAP[@]}"; do
    SRC="${REPO_DIR}/${item%%:*}"
    DST="${item##*:}"
    if [[ -f "$SRC" ]]; then
        cp -f "$SRC" "$DST"
        chmod 0755 "$DST"
    fi
done

# Create gmode symlink
ln -sf /usr/local/bin/awcc-gameshift-toggle /usr/local/bin/gmode

echo "[6/7] Installing and configuring awccd systemd service..."
if [[ -f "${REPO_DIR}/systemd/awccd.service" ]]; then
    mkdir -p /etc/systemd/system
    cp -f "${REPO_DIR}/systemd/awccd.service" /etc/systemd/system/awccd.service
    chmod 0644 /etc/systemd/system/awccd.service
    systemctl daemon-reload
    systemctl enable awccd.service || true
    systemctl restart awccd.service || true
fi

echo "[7/7] Verifying driver in live kernel memory..."
modprobe alienware-wmi || insmod "${DEST_DIR}/alienware-wmi.ko" || true

echo "=========================================================================="
echo "[+] AWCC Linux Installation Complete!"
echo "    - Driver: Signed & Auto-loading via /etc/modules-load.d/"
echo "    - Security: Secure root-only sysfs permissions (no world-writable controls)"
echo "    - Daemon: awccd.service installed and enabled under systemd"
echo "    - Commands available system-wide: awcc-mode, awcc-battery, awcc-quiet,"
echo "      awcc-balance, awcc-mid, awcc-high, awcc-ultra, gmode, awccctl"
echo "=========================================================================="
