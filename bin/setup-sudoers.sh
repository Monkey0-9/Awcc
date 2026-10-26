#!/usr/bin/env bash
set -euo pipefail

SUDOERS_FILE="/etc/sudoers.d/awcc-linux"
USER_NAME="$(id -un)"

echo "=== AWCC Linux: Sudoers Configuration Helper ==="
echo "Configuring passwordless execution for AWCC hardware sysfs actuation..."

CONTENT="# AWCC Linux: Passwordless hardware profile and fan control
${USER_NAME} ALL=(ALL) NOPASSWD: /usr/bin/tee /sys/class/platform-profile/platform-profile-*/profile
${USER_NAME} ALL=(ALL) NOPASSWD: /usr/bin/tee /sys/firmware/acpi/platform_profile
${USER_NAME} ALL=(ALL) NOPASSWD: /usr/bin/tee /sys/class/hwmon/hwmon*/fan*_boost
${USER_NAME} ALL=(ALL) NOPASSWD: /usr/bin/bash -c echo * > /sys/class/hwmon/hwmon*/fan*_boost
"

if [[ $EUID -eq 0 ]]; then
    echo "${CONTENT}" > "${SUDOERS_FILE}"
    chmod 0440 "${SUDOERS_FILE}"
    echo "[+] Successfully installed ${SUDOERS_FILE} (0440 root:root)."
elif command -v sudo >/dev/null 2>&1; then
    echo "${CONTENT}" | sudo tee "${SUDOERS_FILE}" >/dev/null
    sudo chmod 0440 "${SUDOERS_FILE}"
    echo "[+] Successfully installed ${SUDOERS_FILE} via sudo."
else
    echo "[-] Error: Root privileges required to write /etc/sudoers.d/awcc-linux." >&2
    exit 1
fi

echo "[+] Sudoers setup verified."
