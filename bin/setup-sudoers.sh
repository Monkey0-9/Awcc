#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
ROOT_DIR="$(dirname "${SCRIPT_DIR}")"
SUDOERS_FILE="/etc/sudoers.d/awcc-linux"

# Accurately resolve target desktop user even when invoked under sudo or pkexec
if [[ -n "${SUDO_USER:-}" && "${SUDO_USER}" != "root" ]]; then
    TARGET_USER="${SUDO_USER}"
elif [[ -n "${PKEXEC_UID:-}" ]]; then
    TARGET_USER="$(id -un "${PKEXEC_UID}" 2>/dev/null || stat -c '%U' "${ROOT_DIR}")"
elif [[ $EUID -ne 0 ]]; then
    TARGET_USER="$(id -un)"
else
    TARGET_USER="$(stat -c '%U' "${ROOT_DIR}")"
fi

USER_HOME="$(getent passwd "${TARGET_USER}" 2>/dev/null | cut -d: -f6 || echo "/home/${TARGET_USER}")"

echo "=== AWCC Linux: Complete Privilege & Authentication Setup ==="
echo "[*] Configuring permissions for user: ${TARGET_USER} (Home: ${USER_HOME})..."

CONTENT="# AWCC Linux: Passwordless hardware profile and fan control
${TARGET_USER} ALL=(ALL) NOPASSWD: /usr/bin/tee /sys/class/platform-profile/platform-profile-*/profile
${TARGET_USER} ALL=(ALL) NOPASSWD: /usr/bin/tee /sys/devices/platform/dell-pc/platform-profile/platform-profile-*/profile
${TARGET_USER} ALL=(ALL) NOPASSWD: /usr/bin/tee /sys/firmware/acpi/platform_profile
${TARGET_USER} ALL=(ALL) NOPASSWD: /usr/bin/tee /sys/class/hwmon/hwmon*/fan*_boost
${TARGET_USER} ALL=(ALL) NOPASSWD: /usr/sbin/modprobe alienware-wmi, /sbin/modprobe alienware-wmi
${TARGET_USER} ALL=(ALL) NOPASSWD: /usr/sbin/insmod *, /sbin/insmod *
${TARGET_USER} ALL=(ALL) NOPASSWD: /usr/local/bin/awcc*, /usr/bin/awcc*, ${USER_HOME}/.local/bin/awcc*, ${ROOT_DIR}/bin/awcc*
%wheel ALL=(ALL) NOPASSWD: /usr/local/bin/awcc*, /usr/bin/awcc*, ${USER_HOME}/.local/bin/awcc*, ${ROOT_DIR}/bin/awcc*
"

install_files() {
    # 1. Install Sudoers rule
    mkdir -p /etc/sudoers.d
    echo "${CONTENT}" > "${SUDOERS_FILE}"
    chmod 0440 "${SUDOERS_FILE}"
    if command -v visudo >/dev/null 2>&1; then
        visudo -cf "${SUDOERS_FILE}"
    fi
    echo "[+] Installed sudoers rule: ${SUDOERS_FILE} (0440 root:root)."

    # 2. Install PolicyKit Action (allow_active: yes)
    if [[ -d /usr/share/polkit-1/actions && -f "${ROOT_DIR}/packaging/io.github.monkey0_9.awcc.policy" ]]; then
        cp -f "${ROOT_DIR}/packaging/io.github.monkey0_9.awcc.policy" /usr/share/polkit-1/actions/io.github.monkey0_9.awcc.policy
        chmod 0644 /usr/share/polkit-1/actions/io.github.monkey0_9.awcc.policy
        echo "[+] Installed PolicyKit action: /usr/share/polkit-1/actions/io.github.monkey0_9.awcc.policy."
    fi

    # 3. Install PolicyKit Rules (passwordless pkexec for active desktop users and wheel group)
    if [[ -d /etc/polkit-1/rules.d && -f "${ROOT_DIR}/packaging/90-awcc.rules" ]]; then
        cp -f "${ROOT_DIR}/packaging/90-awcc.rules" /etc/polkit-1/rules.d/90-awcc.rules
        chmod 0644 /etc/polkit-1/rules.d/90-awcc.rules
        echo "[+] Installed PolicyKit rule: /etc/polkit-1/rules.d/90-awcc.rules."
    fi

    # 4. Synchronize system binaries to /usr/local/bin
    mkdir -p /usr/local/bin
    local CANDIDATE_CTL=""
    for cand in "${ROOT_DIR}/target/release/awccctl" "${USER_HOME}/.local/bin/awccctl" "${ROOT_DIR}/bin/awccctl"; do
        if [[ -x "$cand" ]]; then
            CANDIDATE_CTL="$cand"
            break
        fi
    done
    if [[ -n "${CANDIDATE_CTL}" ]]; then
        cp -f "${CANDIDATE_CTL}" /usr/local/bin/awccctl
        chmod 0755 /usr/local/bin/awccctl
        echo "[+] Synchronized system binary: /usr/local/bin/awccctl."
    fi

    local CANDIDATE_GUI=""
    for cand in "${ROOT_DIR}/bin/awcc-gui" "${USER_HOME}/.local/bin/awcc-gui"; do
        if [[ -x "$cand" ]]; then
            CANDIDATE_GUI="$cand"
            break
        fi
    done
    if [[ -n "${CANDIDATE_GUI}" ]]; then
        cp -f "${CANDIDATE_GUI}" /usr/local/bin/awcc-gui
        chmod 0755 /usr/local/bin/awcc-gui
        echo "[+] Synchronized system binary: /usr/local/bin/awcc-gui."
    fi

    # 5. Grant group wheel write permissions on active sysfs hardware nodes
    for p in /sys/class/platform-profile/platform-profile-*/profile /sys/firmware/acpi/platform_profile /sys/class/hwmon/hwmon*/fan*_boost; do
        if [[ -f "$p" ]]; then
            chgrp wheel "$p" 2>/dev/null || true
            chmod 0664 "$p" 2>/dev/null || true
        fi
    done
    echo "[+] Configured group wheel write access (0664) on sysfs hardware nodes."

    # 6. Install udev rule to keep sysfs permissions across reboots
    mkdir -p /etc/udev/rules.d
    cat << 'UDEV_EOF' > /etc/udev/rules.d/90-awcc-sysfs.rules
# AWCC Linux: Allow members of group wheel to adjust platform profile and fan boost
SUBSYSTEM=="platform-profile", ACTION=="add|change", GROUP="wheel", MODE="0664"
SUBSYSTEM=="hwmon", ACTION=="add|change", GROUP="wheel", MODE="0664"
KERNEL=="platform_profile", GROUP="wheel", MODE="0664"
UDEV_EOF
    chmod 0644 /etc/udev/rules.d/90-awcc-sysfs.rules
    udevadm control --reload-rules 2>/dev/null || true
    echo "[+] Installed persistent udev rule: /etc/udev/rules.d/90-awcc-sysfs.rules."
}

if [[ $EUID -eq 0 ]]; then
    install_files
elif command -v sudo >/dev/null 2>&1; then
    export CONTENT SUDOERS_FILE ROOT_DIR TARGET_USER USER_HOME
    sudo -E bash -c "$(declare -f install_files); install_files"
elif command -v pkexec >/dev/null 2>&1; then
    export CONTENT SUDOERS_FILE ROOT_DIR TARGET_USER USER_HOME
    pkexec bash -c "$(declare -f install_files); install_files"
else
    echo "[-] Error: Superuser privileges required to configure system permissions." >&2
    exit 1
fi

echo "=========================================================================="
echo "[+] AWCC Linux System Privileges & PolicyKit Configured Successfully!"
echo "    - Direct Sysfs:   Wheel group 0664 write access enabled"
echo "    - Passwordless:   Sudoers NOPASSWD enabled for ${TARGET_USER} and %wheel"
echo "    - PolicyKit:      allow_active: yes (zero GUI auth popups)"
echo "    - Desktop GUI:    Mode changes and fan adjustments will now actuate silently."
echo "=========================================================================="
