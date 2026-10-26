#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-}"
FLAG="${2:-}"

# Discover alienware_wmi hwmon
AW_HWMON=""
for d in /sys/class/hwmon/hwmon*; do
    if [[ -f "$d/name" ]] && [[ "$(cat "$d/name" 2>/dev/null)" == "alienware_wmi" ]]; then
        AW_HWMON="$d"
        break
    fi
done

# Discover platform-profile path (prefer alienware-wmi class device)
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

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
ROOT_DIR="$(dirname "${SCRIPT_DIR}")"
DASHBOARD_SCRIPT=""
for candidate in \
    "${SCRIPT_DIR}/awcc-lux-dashboard.py" \
    "${HOME}/.local/bin/awcc-lux-dashboard.py" \
    "/usr/local/bin/awcc-lux-dashboard.py" \
    "/usr/bin/awcc-lux-dashboard.py"; do
    if [[ -f "$candidate" ]]; then
        DASHBOARD_SCRIPT="$candidate"
        break
    fi
done

# Dynamic hardware identification
VENDOR="Generic"
PRODUCT="Linux PC"
if [[ -f /sys/devices/virtual/dmi/id/sys_vendor ]]; then
    VENDOR="$(cat /sys/devices/virtual/dmi/id/sys_vendor 2>/dev/null | tr -d '\0' | xargs)"
fi
if [[ -f /sys/devices/virtual/dmi/id/product_name ]]; then
    PRODUCT="$(cat /sys/devices/virtual/dmi/id/product_name 2>/dev/null | tr -d '\0' | xargs)"
fi
CLEAN_VENDOR="$(echo "${VENDOR}" | sed -E 's/ inc\.?| ltd\.?| corp\.?| corporation//gI' | xargs)"
if [[ "${PRODUCT,,}" == "${CLEAN_VENDOR,,}"* ]]; then
    DISPLAY_MACHINE="${PRODUCT}"
else
    DISPLAY_MACHINE="${CLEAN_VENDOR} ${PRODUCT}"
fi

# If no mode argument is provided or a flag is passed, launch the live monitor
if [[ -z "${MODE}" ]] || [[ "${MODE}" =~ ^- ]] || [[ "${MODE,,}" == "monitor" ]] || [[ "${MODE,,}" == "live" ]] || [[ "${MODE,,}" == "track" ]]; then
    if [[ -n "$DASHBOARD_SCRIPT" ]]; then
        exec python3 "$DASHBOARD_SCRIPT" "$@"
    else
        echo "[-] Error: awcc-lux-dashboard.py not found." >&2
        exit 1
    fi
fi

case "${MODE,,}" in
    1|battery|powersave|low)
        PROFILE="quiet"
        BOOST=10
        TITLE="BATTERY SAVER (Silent Whisper, Boost 10/255, ~1,000 RPM)"
        ;;
    2|quiet|silent)
        PROFILE="quiet"
        BOOST=20
        TITLE="QUIET (Acoustic Silence, Boost 20/255, ~1,450 RPM)"
        ;;
    3|balance|balanced)
        PROFILE="balanced"
        BOOST=40
        TITLE="BALANCE (Daily Productivity, Boost 40/255, ~1,980 RPM)"
        ;;
    4|mid)
        PROFILE="performance"
        BOOST=65
        TITLE="MID (Intermediate Performance, Boost 65/255, ~3,100 RPM)"
        ;;
    5|high|performance)
        PROFILE="performance"
        BOOST=85
        TITLE="HIGH PERFORMANCE (Sustained Workloads, Boost 85/255, ~4,200 RPM)"
        ;;
    6|ultra|gmode|g-mode|max|255)
        PROFILE="performance"
        BOOST=255
        TITLE="ULTRA PERFORMANCE & GAME SHIFT (G-MODE Max Fans, Boost 255/255)"
        ;;
    *)
        echo "[-] Unknown mode '${MODE}'. Choose from: 1-6 or battery, quiet, balance, mid, high, ultra." >&2
        exit 1
        ;;
esac

echo "=========================================================================="
echo " AWCC Linux: Actuating ${TITLE}"
echo " Target Machine: ${DISPLAY_MACHINE}"
echo "=========================================================================="

# 1. Apply Platform Profile
echo "[1/3] Setting Platform Profile to '${PROFILE}'..."
if [[ -n "${PPROF_FILE}" ]]; then
    if [[ -w "${PPROF_FILE}" ]]; then
        echo "${PROFILE}" > "${PPROF_FILE}"
    elif command -v sudo >/dev/null 2>&1; then
        echo "${PROFILE}" | sudo tee "${PPROF_FILE}" >/dev/null
    else
        echo "[-] Warning: Unable to write platform profile (root/sudo required)." >&2
    fi
fi

# 2. Apply Fan Boost
echo "[2/3] Setting Fan Boost to ${BOOST}/255..."
if [[ -n "${AW_HWMON}" ]]; then
    if [[ -w "${AW_HWMON}/fan1_boost" ]]; then
        echo "${BOOST}" > "${AW_HWMON}/fan1_boost"
        echo "${BOOST}" > "${AW_HWMON}/fan2_boost"
    elif command -v sudo >/dev/null 2>&1; then
        echo "${BOOST}" | sudo tee "${AW_HWMON}/fan1_boost" "${AW_HWMON}/fan2_boost" >/dev/null
    else
        echo "[-] Error: Writing fan boost requires root/sudo." >&2
        exit 1
    fi

    READ_B1="$(cat "${AW_HWMON}/fan1_boost" 2>/dev/null || echo '?')"
    READ_B2="$(cat "${AW_HWMON}/fan2_boost" 2>/dev/null || echo '?')"
    if [[ "${READ_B1}" == "${BOOST}" ]] && [[ "${READ_B2}" == "${BOOST}" ]]; then
        echo "[+] Fan boost readback verified: Fan 1 = ${READ_B1}/255 | Fan 2 = ${READ_B2}/255"
    else
        echo "[-] Error: Fan boost readback mismatch: requested ${BOOST}, got Fan 1: ${READ_B1}, Fan 2: ${READ_B2}" >&2
        exit 1
    fi
fi

# 3. Synchronize Game Shift State File
STATE_FILE="/tmp/.awcc_gmode_state"
if [[ "$BOOST" -eq 255 ]]; then
    echo "ON" > "$STATE_FILE"
else
    echo "OFF" > "$STATE_FILE"
fi

# 4. Handle flag transitions
if [[ "$FLAG" == "--quiet" ]] || [[ "$FLAG" == "-q" ]]; then
    exit 0
elif [[ "$FLAG" == "--track" ]] || [[ "$FLAG" == "-t" ]] || [[ "$FLAG" == "--watch" ]] || [[ "$FLAG" == "-w" ]]; then
    echo "[*] Transitioning directly to real-time hardware tracking cockpit..."
    sleep 0.4
    if [[ -x "${SCRIPT_DIR}/awcc-track" ]]; then
        exec "${SCRIPT_DIR}/awcc-track"
    elif command -v awcc-track >/dev/null 2>&1; then
        exec awcc-track
    elif [[ -n "$DASHBOARD_SCRIPT" ]]; then
        exec python3 "$DASHBOARD_SCRIPT"
    elif [[ -x "${ROOT_DIR}/target/release/awccctl" ]]; then
        exec "${ROOT_DIR}/target/release/awccctl" track
    fi
elif [[ "$FLAG" == "--snapshot" ]] || [[ "$FLAG" == "-s" ]] || [[ "$FLAG" == "--once" ]]; then
    if [[ -n "$DASHBOARD_SCRIPT" ]]; then
        python3 "$DASHBOARD_SCRIPT" --snapshot
    fi
else
    echo "[3/3] Current System & Thermal Status:"
    AWCCCTL_BIN=""
    if [[ -x "${ROOT_DIR}/target/release/awccctl" ]]; then
        AWCCCTL_BIN="${ROOT_DIR}/target/release/awccctl"
    elif [[ -x "${HOME}/.local/bin/awccctl" ]]; then
        AWCCCTL_BIN="${HOME}/.local/bin/awccctl"
    elif command -v awccctl >/dev/null 2>&1; then
        AWCCCTL_BIN="$(command -v awccctl)"
    fi

    if [[ -n "${AWCCCTL_BIN}" ]]; then
        "${AWCCCTL_BIN}" status
    fi
    echo "=========================================================================="
    echo "[+] ${TITLE} activated successfully."
    echo "=========================================================================="
fi
