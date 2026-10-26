#!/bin/bash
# ==============================================================================
# bin/install-system.sh
# Universal Multi-Distribution Installer for AWCC Linux & Hardware Governor
# Supports RHEL, Fedora, Ubuntu, Debian, Arch, openSUSE, Pop!_OS, and Generic Linux
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=========================================================================="
echo " AWCC Universal Linux Hardware Control & Performance Suite Installer"
echo "=========================================================================="

if [[ $EUID -ne 0 ]]; then
    echo "[-] Error: This installation script must be run as root:" >&2
    echo "    sudo ./install-system.sh" >&2
    exit 1
fi

# 1. Detect Host Distribution
DISTRO="Generic Linux"
if [[ -f /etc/os-release ]]; then
    DISTRO="$(grep -E '^PRETTY_NAME=' /etc/os-release | cut -d= -f2 | tr -d '"' || true)"
fi
echo "[*] Detected Host Operating System: ${DISTRO}"

# 2. Detect Hardware Vendor
DMI_VENDOR="Unknown"
if [[ -f /sys/devices/virtual/dmi/id/sys_vendor ]]; then
    DMI_VENDOR="$(cat /sys/devices/virtual/dmi/id/sys_vendor | tr -d '\n' || true)"
fi
DMI_PRODUCT="Unknown"
if [[ -f /sys/devices/virtual/dmi/id/product_name ]]; then
    DMI_PRODUCT="$(cat /sys/devices/virtual/dmi/id/product_name | tr -d '\n' || true)"
fi
echo "[*] Detected Hardware: ${DMI_VENDOR} ${DMI_PRODUCT}"

# 3. Kernel Module Handling
IS_DELL=false
if echo "$DMI_VENDOR" | grep -qiE "Dell|Alienware"; then
    IS_DELL=true
fi

if [[ "$IS_DELL" == "true" ]]; then
    echo "[1/7] Dell/Alienware system detected: Verifying alienware-wmi driver..."
    KVER="$(uname -r)"
    DEST_DIR="/lib/modules/${KVER}/extra"
    MODULE_SRC="${REPO_DIR}/kernel/alienware-wmi.ko"

    if [[ ! -f "${MODULE_SRC}" ]]; then
        echo "[*] Module ${MODULE_SRC} not found. Building and signing..."
        if make -C "${REPO_DIR}/kernel" >/dev/null 2>&1; then
            if [[ -x "${REPO_DIR}/kernel/scripts/sign-module.sh" ]]; then
                "${REPO_DIR}/kernel/scripts/sign-module.sh" || true
            fi
        else
            echo "[!] Notice: Kernel headers not found or build failed; skipping local kernel module compile."
        fi
    fi

    if [[ -f "${MODULE_SRC}" ]]; then
        mkdir -p "${DEST_DIR}"
        cp -f "${MODULE_SRC}" "${DEST_DIR}/alienware-wmi.ko"
        depmod -a
        mkdir -p /etc/modules-load.d
        echo "alienware-wmi" > /etc/modules-load.d/alienware-wmi.conf
        modprobe alienware-wmi 2>/dev/null || insmod "${DEST_DIR}/alienware-wmi.ko" 2>/dev/null || true
    fi
else
    echo "[1/7] Non-Dell hardware detected (${DMI_VENDOR}):"
    echo "      Using native Linux upstream kernel drivers (asus-wmi, thinkpad_acpi, hp-wmi, cros_ec, hwmon)."
fi

# 4. Security Audit & Sysfs Permissions
echo "[2/7] Enforcing secure sysfs permissions (zero world-writable files)..."
if [[ -f /etc/udev/rules.d/99-awcc-hwmon.rules ]]; then
    rm -f /etc/udev/rules.d/99-awcc-hwmon.rules
    udevadm control --reload-rules 2>/dev/null || true
fi

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

# 5. Build Rust Binaries if Needed
echo "[3/7] Verifying and building release binaries..."
if [[ ! -f "${REPO_DIR}/target/release/awccctl" || ! -f "${REPO_DIR}/target/release/awccd" ]]; then
    echo "[*] Compiling awccctl and awccd via cargo..."
    cargo build --release --manifest-path "${REPO_DIR}/Cargo.toml"
fi

# 6. Install Binaries and Scripts
echo "[4/7] Installing system binaries and management utilities to /usr/local/bin..."
BIN_MAP=(
    "bin/awcc-battery:/usr/local/bin/awcc-battery"
    "bin/awcc-quiet:/usr/local/bin/awcc-quiet"
    "bin/awcc-balance:/usr/local/bin/awcc-balance"
    "bin/awcc-mid:/usr/local/bin/awcc-mid"
    "bin/awcc-high:/usr/local/bin/awcc-high"
    "bin/awcc-ultra:/usr/local/bin/awcc-ultra"
    "bin/awcc-mode:/usr/local/bin/awcc-mode"
    "bin/awcc-mode.sh:/usr/local/bin/awcc-mode.sh"
    "bin/awcc-track:/usr/local/bin/awcc-track"
    "bin/awcc-monitor:/usr/local/bin/awcc-monitor"
    "bin/awcc-gameshift-toggle.sh:/usr/local/bin/awcc-gameshift-toggle"
    "bin/awcc-lux-dashboard.py:/usr/local/bin/awcc-lux-dashboard.py"
    "bin/awcc-gui:/usr/local/bin/awcc-gui"
    "bin/doctor-recovery.sh:/usr/local/bin/doctor-recovery"
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

# 7. Install FreeDesktop Desktop Entry and Icon
echo "[5/7] Installing desktop launcher and FreeDesktop application icon..."
mkdir -p /usr/local/share/applications /usr/share/applications
if [[ -f "${REPO_DIR}/packaging/awcc-gui.desktop" ]]; then
    cp -f "${REPO_DIR}/packaging/awcc-gui.desktop" /usr/share/applications/awcc-gui.desktop 2>/dev/null || \
    cp -f "${REPO_DIR}/packaging/awcc-gui.desktop" /usr/local/share/applications/awcc-gui.desktop
fi

mkdir -p /usr/share/icons/hicolor/scalable/apps
if [[ -f "${REPO_DIR}/packaging/icons/awcc-gui.svg" ]]; then
    cp -f "${REPO_DIR}/packaging/icons/awcc-gui.svg" /usr/share/icons/hicolor/scalable/apps/awcc-gui.svg
fi

# Install Shell Completions
echo "[*] Installing shell completions (Bash, Zsh, Fish)..."
if [[ -f "${REPO_DIR}/completions/bash/awccctl" ]]; then
    mkdir -p /usr/share/bash-completion/completions
    cp -f "${REPO_DIR}/completions/bash/awccctl" /usr/share/bash-completion/completions/awccctl
    chmod 0644 /usr/share/bash-completion/completions/awccctl
fi
if [[ -f "${REPO_DIR}/completions/zsh/_awccctl" ]]; then
    mkdir -p /usr/share/zsh/site-functions
    cp -f "${REPO_DIR}/completions/zsh/_awccctl" /usr/share/zsh/site-functions/_awccctl
    chmod 0644 /usr/share/zsh/site-functions/_awccctl
fi
if [[ -f "${REPO_DIR}/completions/fish/awccctl.fish" ]]; then
    mkdir -p /usr/share/fish/vendor_completions.d
    cp -f "${REPO_DIR}/completions/fish/awccctl.fish" /usr/share/fish/vendor_completions.d/awccctl.fish
    chmod 0644 /usr/share/fish/vendor_completions.d/awccctl.fish
fi

# Install Manual Pages
echo "[*] Installing Unix manual pages to /usr/share/man/man1..."
mkdir -p /usr/share/man/man1
for mp in awccctl.1 awcc-lux-dashboard.1 awcc-mode.1; do
    if [[ -f "${REPO_DIR}/man/man1/${mp}" ]]; then
        cp -f "${REPO_DIR}/man/man1/${mp}" "/usr/share/man/man1/${mp}"
        chmod 0644 "/usr/share/man/man1/${mp}"
    fi
done

# Install PolicyKit policy and sudoers configuration
if [[ -d /usr/share/polkit-1/actions ]] && [[ -f "${REPO_DIR}/packaging/io.github.monkey0_9.awcc.policy" ]]; then
    echo "[*] Installing PolicyKit action policy..."
    cp -f "${REPO_DIR}/packaging/io.github.monkey0_9.awcc.policy" /usr/share/polkit-1/actions/io.github.monkey0_9.awcc.policy
    chmod 0644 /usr/share/polkit-1/actions/io.github.monkey0_9.awcc.policy
fi

if [[ -x "${REPO_DIR}/bin/setup-sudoers.sh" ]]; then
    echo "[*] Configuring sudoers rules..."
    "${REPO_DIR}/bin/setup-sudoers.sh" || true
fi

# 8. Configure System Daemon & Service
echo "[6/7] Configuring background thermal governor service..."
if pidof systemd >/dev/null 2>&1 || [[ -d /run/systemd/system ]]; then
    echo "[*] Systemd detected: installing awccd.service..."
    mkdir -p /etc/systemd/system
    cp -f "${REPO_DIR}/systemd/awccd.service" /etc/systemd/system/awccd.service
    chmod 0644 /etc/systemd/system/awccd.service
    systemctl daemon-reload
    systemctl enable awccd.service 2>/dev/null || true
    systemctl restart awccd.service 2>/dev/null || true
elif [[ -d /etc/init.d ]] && [[ -f "${REPO_DIR}/systemd/openrc-awccd.initd" ]]; then
    echo "[*] OpenRC detected: installing /etc/init.d/awccd..."
    cp -f "${REPO_DIR}/systemd/openrc-awccd.initd" /etc/init.d/awccd
    chmod 0755 /etc/init.d/awccd
    rc-update add awccd default 2>/dev/null || true
elif [[ -d /etc/runit/runsvdir ]] && [[ -f "${REPO_DIR}/systemd/runit-awccd.run" ]]; then
    echo "[*] Runit detected: installing /etc/sv/awccd..."
    mkdir -p /etc/sv/awccd
    cp -f "${REPO_DIR}/systemd/runit-awccd.run" /etc/sv/awccd/run
    chmod 0755 /etc/sv/awccd/run
fi

echo "[7/7] Running initial hardware diagnostics..."
if [[ -x /usr/local/bin/awccctl ]]; then
    /usr/local/bin/awccctl detect || true
fi

echo "=========================================================================="
echo "[+] Universal AWCC Linux Suite Installed Successfully!"
echo "    - CLI Utilities:   awccctl, awcc-mode, gmode in /usr/local/bin"
echo "    - Desktop GUI:     awcc-gui (Launched from App Menu or 'awcc-gui')"
echo "    - TUI Dashboard:   awcc-lux-dashboard.py (or 'awccctl status')"
echo "    - Autonomous Svc:  awccd (Governs thermal ramp & hysteresis)"
echo "    - Recovery Tool:   doctor-recovery (Reverts fans if needed)"
echo "=========================================================================="
