#!/bin/bash
# ==============================================================================
# bin/uninstall-system.sh
# Universal Clean Uninstaller for AWCC Linux & Hardware Governor
# ==============================================================================
set -euo pipefail

echo "=========================================================================="
echo " AWCC Linux Hardware Control Suite — Clean Uninstaller"
echo "=========================================================================="

if [[ $EUID -ne 0 ]]; then
    echo "[-] Error: This uninstallation script must be run as root:" >&2
    echo "    sudo ./uninstall-system.sh" >&2
    exit 1
fi

# 1. Revert fans to firmware control
echo "[1/6] Reverting fans to BIOS firmware control..."
if command -v awccctl >/dev/null 2>&1; then
    awccctl fan release 2>/dev/null || true
    awccctl profile balanced 2>/dev/null || true
fi
for d in /sys/class/hwmon/hwmon*; do
    if [[ -f "$d/name" ]] && [[ "$(cat "$d/name" 2>/dev/null)" == "alienware_wmi" ]]; then
        for b in "$d"/fan*_boost; do
            if [[ -f "$b" ]]; then
                echo 0 > "$b" 2>/dev/null || true
            fi
        done
    fi
done

# 2. Stop and remove system daemon
echo "[2/6] Stopping and removing background thermal daemon..."
if pidof systemd >/dev/null 2>&1 || [[ -d /run/systemd/system ]]; then
    systemctl stop awccd.service 2>/dev/null || true
    systemctl disable awccd.service 2>/dev/null || true
    rm -f /etc/systemd/system/awccd.service
    systemctl daemon-reload
fi
if [[ -f /etc/init.d/awccd ]]; then
    rc-service awccd stop 2>/dev/null || true
    rc-update del awccd default 2>/dev/null || true
    rm -f /etc/init.d/awccd
fi
if [[ -d /etc/sv/awccd ]]; then
    rm -rf /etc/sv/awccd
fi

# 3. Remove installed binaries and symlinks
echo "[3/6] Removing installed binaries from /usr/local/bin..."
BINS=(
    awccctl
    awccd
    awcc-gui
    awcc-lux-dashboard.py
    awcc-mode
    awcc-mode.sh
    awcc-gameshift-toggle
    doctor-recovery
    gmode
    awcc-battery
    awcc-quiet
    awcc-balance
    awcc-mid
    awcc-high
    awcc-ultra
    rhel-performance
    rhel-balanced
    rhel-ultra
    rhel-quiet
    rhel-mid
    rhel-cool
    set-profile-helper.sh
    run-max-fans.sh
    stop-max-fans.sh
)

for b in "${BINS[@]}"; do
    rm -f "/usr/local/bin/${b}"
done

# 4. Remove FreeDesktop application entry and icons
echo "[4/6] Removing desktop launchers and icons..."
rm -f /usr/share/applications/awcc-gui.desktop
rm -f /usr/local/share/applications/awcc-gui.desktop
rm -f /usr/share/icons/hicolor/scalable/apps/awcc-gui.svg

# 5. Remove kernel module extra if installed
echo "[5/6] Cleaning extra kernel modules..."
KVER="$(uname -r)"
if [[ -f "/lib/modules/${KVER}/extra/alienware-wmi.ko" ]]; then
    rm -f "/lib/modules/${KVER}/extra/alienware-wmi.ko"
    depmod -a
fi
rm -f /etc/modules-load.d/alienware-wmi.conf

# 6. Remove transient state files
echo "[6/6] Cleaning lock and state files..."
rm -f /tmp/.awcc_gmode_state
rm -f /run/awccd.pid

echo "=========================================================================="
echo "[+] AWCC Linux has been cleanly uninstalled from your system."
echo "=========================================================================="
