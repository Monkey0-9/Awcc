#!/usr/bin/env bash
set -euo pipefail

PROFILE="${1:-balanced}"
BOOST="${2:-0}"
TITLE="${3:-BALANCED}"

echo "================================================================="
echo " AWCC Linux: Actuating ${TITLE} Mode"
echo " Target Machine: Dell G15 5530 (RHEL 10.2)"
echo "================================================================="

# Locate AWCC platform-profile class device or fallback to firmware node
PPROF_FILE=""
for p in /sys/class/platform-profile/platform-profile-*; do
    if [[ -f "$p/name" ]] && [[ "$(cat "$p/name" 2>/dev/null)" == "alienware-wmi" ]]; then
        PPROF_FILE="$p/profile"
        break
    fi
done
if [[ -z "${PPROF_FILE}" ]] && [[ -f /sys/firmware/acpi/platform_profile ]]; then
    PPROF_FILE="/sys/firmware/acpi/platform_profile"
fi

# Locate alienware_wmi hwmon
AW_HWMON=""
for d in /sys/class/hwmon/hwmon*; do
    if [[ -f "$d/name" ]] && [[ "$(cat "$d/name" 2>/dev/null)" == "alienware_wmi" ]]; then
        AW_HWMON="$d"
        break
    fi
done

if [[ -z "${AW_HWMON}" ]]; then
    echo "[-] Error: alienware_wmi driver not found. Please ensure kernel module is loaded." >&2
    exit 1
fi

# 1. Apply Platform Profile
echo "[1/3] Setting Platform Profile to '${PROFILE}'..."
PROFILE_APPLIED=0
if [[ -n "${PPROF_FILE}" ]]; then
    if [[ -w "${PPROF_FILE}" ]]; then
        echo "${PROFILE}" > "${PPROF_FILE}"
        PROFILE_APPLIED=1
    elif command -v sudo >/dev/null 2>&1; then
        echo "${PROFILE}" | sudo tee "${PPROF_FILE}" >/dev/null && PROFILE_APPLIED=1
    fi
fi

if [[ "${PROFILE_APPLIED}" -eq 1 ]]; then
    READ_PROFILE="$(cat "${PPROF_FILE}" 2>/dev/null || echo 'unknown')"
    if [[ "${READ_PROFILE}" != "${PROFILE}" ]]; then
        echo "[-] Warning: Platform profile readback mismatch: requested '${PROFILE}', read '${READ_PROFILE}'." >&2
    else
        echo "[+] Platform Profile confirmed: ${READ_PROFILE}"
    fi
else
    echo "[-] Warning: Unable to write platform profile (requires root/sudo)." >&2
fi

# 2. Apply Fan Boost
echo "[2/3] Setting Fan Boost to ${BOOST}/255..."
BOOST_APPLIED=0
if [[ -w "${AW_HWMON}/fan1_boost" ]]; then
    echo "${BOOST}" > "${AW_HWMON}/fan1_boost"
    echo "${BOOST}" > "${AW_HWMON}/fan2_boost"
    BOOST_APPLIED=1
elif command -v sudo >/dev/null 2>&1; then
    sudo bash -c "echo ${BOOST} > '${AW_HWMON}/fan1_boost' && echo ${BOOST} > '${AW_HWMON}/fan2_boost'" && BOOST_APPLIED=1
fi

if [[ "${BOOST_APPLIED}" -eq 1 ]]; then
    READ_B1="$(cat "${AW_HWMON}/fan1_boost" 2>/dev/null || echo '?')"
    READ_B2="$(cat "${AW_HWMON}/fan2_boost" 2>/dev/null || echo '?')"
    if [[ "${READ_B1}" == "${BOOST}" ]] && [[ "${READ_B2}" == "${BOOST}" ]]; then
        echo "[+] Fan boost confirmed: Fan 1 = ${READ_B1}/255, Fan 2 = ${READ_B2}/255"
    else
        echo "[-] Error: Fan boost readback mismatch: requested ${BOOST}, got Fan 1: ${READ_B1}, Fan 2: ${READ_B2}" >&2
        exit 1
    fi
else
    echo "[-] Error: Unable to write fan boost controls. Root or sudo required." >&2
    exit 1
fi

# 3. Present Confirmation and Enter Real-Time Monitoring
echo "================================================================="
echo "[+] ${TITLE} Mode successfully actuated and readback verified."
echo "================================================================="

AWCCCTL_EXEC=""
if command -v awccctl >/dev/null 2>&1; then
    AWCCCTL_EXEC="awccctl"
elif [[ -x "$(dirname "$0")/../target/release/awccctl" ]]; then
    AWCCCTL_EXEC="$(dirname "$0")/../target/release/awccctl"
elif [[ -x "$(dirname "$0")/../target/debug/awccctl" ]]; then
    AWCCCTL_EXEC="$(dirname "$0")/../target/debug/awccctl"
fi

if [[ "${4:-}" == "--once" ]] || [[ "${4:-}" == "--snapshot" ]] || [[ "${4:-}" == "--no-monitor" ]]; then
    echo "[3/3] Current System & Thermal Status:"
    if [[ -n "${AWCCCTL_EXEC}" ]]; then
        "${AWCCCTL_EXEC}" status
    fi
elif [[ -t 1 ]] && [[ -t 0 ]] && [[ "${AWCC_NONINTERACTIVE:-0}" != "1" ]] && [[ -z "${CI:-}" ]]; then
    echo ""
    echo "[*] Entering continuous real-time monitoring (Press Ctrl+C to stop)..."
    sleep 0.5
    if [[ -n "${AWCCCTL_EXEC}" ]]; then
        exec "${AWCCCTL_EXEC}" monitor --interval 1.0
    fi
else
    echo "[3/3] Current System & Thermal Status:"
    if [[ -n "${AWCCCTL_EXEC}" ]]; then
        "${AWCCCTL_EXEC}" status
    fi
fi
