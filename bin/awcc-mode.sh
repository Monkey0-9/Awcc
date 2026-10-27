#!/usr/bin/env bash
set -euo pipefail

# Clean terminal restoration trap
cleanup() {
    printf "\x1b[?1049l\x1b[?25h\x1b[0m" 2>/dev/null || true
}
trap cleanup EXIT INT TERM HUP

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

REAL_SOURCE="$(readlink -f "${BASH_SOURCE[0]}" 2>/dev/null || echo "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd -P "$(dirname "${REAL_SOURCE}")" >/dev/null 2>&1 && pwd)"
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

ensure_alienware_driver() {
    # If already found, return success
    if [[ -n "${AW_HWMON}" ]]; then
        return 0
    fi

    # Only attempt on Dell / Alienware hardware
    if ! echo "${DISPLAY_MACHINE}" | grep -qiE "Dell|Alienware"; then
        return 0
    fi

    local MODULE_KO=""
    for cand in \
        "${ROOT_DIR}/kernel/alienware-wmi.ko" \
        "${HOME}/Awcc/kernel/alienware-wmi.ko" \
        "/lib/modules/$(uname -r)/extra/alienware-wmi.ko"; do
        if [[ -f "$cand" ]]; then
            MODULE_KO="$cand"
            break
        fi
    done

    echo "[*] Notice: alienware_wmi driver is not loaded. Loading kernel module..."

    # 1. Try modprobe if installed in /lib/modules
    if sudo -n modprobe alienware-wmi 2>/dev/null || { [[ -t 0 && -t 1 && -z "${AWCC_NONINTERACTIVE:-}" ]] && sudo modprobe alienware-wmi 2>/dev/null; }; then
        :
    # 2. Try insmod with candidate module file
    elif [[ -n "${MODULE_KO}" ]]; then
        if sudo -n insmod "${MODULE_KO}" 2>/dev/null || { [[ -t 0 && -t 1 && -z "${AWCC_NONINTERACTIVE:-}" ]] && sudo insmod "${MODULE_KO}"; }; then
            :
        fi
    fi

    # Re-discover hwmon
    for d in /sys/class/hwmon/hwmon*; do
        if [[ -f "$d/name" ]] && [[ "$(cat "$d/name" 2>/dev/null)" == "alienware_wmi" ]]; then
            AW_HWMON="$d"
            echo "[+] alienware-wmi module loaded successfully ($d)."
            break
        fi
    done

    # Re-discover platform-profile
    for p in /sys/class/platform-profile/platform-profile-*; do
        if [[ -f "$p/name" ]] && [[ "$(cat "$p/name" 2>/dev/null)" == "alienware-wmi" ]]; then
            PPROF_FILE="$p/profile"
            break
        fi
    done
    if [[ -z "${PPROF_FILE}" ]] && [[ -f /sys/firmware/acpi/platform_profile ]]; then
        PPROF_FILE="/sys/firmware/acpi/platform_profile"
    fi

    if [[ -n "${AW_HWMON}" ]]; then
        return 0
    else
        return 1
    fi
}

# Help documentation
if [[ "${MODE}" == "--help" ]] || [[ "${MODE}" == "-h" ]] || [[ "${FLAG}" == "--help" ]] || [[ "${FLAG}" == "-h" ]]; then
    echo "Usage: awcc-mode [MODE] [OPTIONS]"
    echo ""
    echo "Modes (20% Fan Boost Ladder up to 255 Max RPM):"
    echo "  1 | battery      Battery Saver (20% Boost, 51/255, ~1,500 RPM)"
    echo "  2 | quiet        Acoustic Silence (40% Boost, 102/255, ~2,200 RPM)"
    echo "  3 | balanced     Daily Productivity (60% Boost, 153/255, ~3,100 RPM)"
    echo "  4 | performance  High Performance (80% Boost, 204/255, ~4,200 RPM)"
    echo "  5 | ultra        Ultra Performance & Game Shift (100% Boost, 255/255 Max RPM)"
    echo ""
    echo "Options:"
    echo "  --once, -s       Print single status snapshot and exit (no live monitoring)"
    echo "  --monitor, -m    Explicitly enter continuous live monitoring until Ctrl+C"
    echo "  --tui            Launch full interactive 24-bit TrueColor cockpit"
    echo "  --quiet, -q      Actuate profile silently without status output"
    echo "  --help, -h       Display this help documentation"
    exit 0
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
    1|battery|powersave|low|bat)
        PROFILE="quiet"
        BOOST=51
        TITLE="BATTERY SAVER (20% Fan Boost 51/255, ~1,500 RPM)"
        ;;
    2|quiet|silent)
        PROFILE="quiet"
        BOOST=102
        TITLE="QUIET (40% Fan Boost 102/255, ~2,200 RPM)"
        ;;
    3|balance|balanced|normal)
        PROFILE="balanced"
        BOOST=153
        TITLE="BALANCED (60% Fan Boost 153/255, ~3,100 RPM)"
        ;;
    4|performance|high|perf|mid)
        PROFILE="performance"
        BOOST=204
        TITLE="PERFORMANCE (80% Fan Boost 204/255, ~4,200 RPM)"
        ;;
    5|ultra|gmode|g-mode|max|255|6)
        PROFILE="performance"
        BOOST=255
        TITLE="ULTRA PERFORMANCE & GAME SHIFT (100% Max Fans 255/255)"
        ;;
    *)
        echo "[-] Unknown mode '${MODE}'. Choose from: 1-5 or battery, quiet, balanced, performance, ultra." >&2
        exit 1
        ;;
esac

echo "=========================================================================="
echo " AWCC Linux: Actuating ${TITLE}"
echo " Target Machine: ${DISPLAY_MACHINE}"
echo "=========================================================================="

# Ensure alienware-wmi is loaded if on Dell/Alienware hardware
ensure_alienware_driver || true

# 1. Apply Platform Profile
echo "[1/3] Setting Platform Profile to '${PROFILE}'..."
if [[ -n "${PPROF_FILE}" ]]; then
    if [[ -w "${PPROF_FILE}" ]]; then
        echo "${PROFILE}" > "${PPROF_FILE}"
    elif echo "${PROFILE}" | sudo -n tee "${PPROF_FILE}" >/dev/null 2>&1; then
        :
    elif [[ -t 0 && -t 1 ]] && command -v sudo >/dev/null 2>&1 && [[ -z "${AWCC_NONINTERACTIVE:-}" ]]; then
        echo "${PROFILE}" | sudo tee "${PPROF_FILE}" >/dev/null
    else
        echo "[-] Notice: Writing platform profile requires root privileges." >&2
        echo "    Tip: Run with 'sudo awcc-mode ${MODE}' or configure passwordless sudo via 'sudo ./bin/setup-sudoers.sh'." >&2
    fi
fi

# 2. Apply Fan Boost & Game Shift
echo "[2/3] Setting Fan Boost to ${BOOST}/255..."
if [[ -z "${AW_HWMON}" ]]; then
    ensure_alienware_driver || true
fi

if [[ -n "${AW_HWMON}" ]]; then
    # Actuate hardware fan boost offsets (0-255)
    if [[ -w "${AW_HWMON}/fan1_boost" ]]; then
        echo "${BOOST}" > "${AW_HWMON}/fan1_boost"
        echo "${BOOST}" > "${AW_HWMON}/fan2_boost"
    elif echo "${BOOST}" | sudo -n tee "${AW_HWMON}/fan1_boost" "${AW_HWMON}/fan2_boost" >/dev/null 2>&1; then
        :
    elif [[ -t 0 && -t 1 ]] && command -v sudo >/dev/null 2>&1 && [[ -z "${AWCC_NONINTERACTIVE:-}" ]]; then
        echo "${BOOST}" | sudo tee "${AW_HWMON}/fan1_boost" "${AW_HWMON}/fan2_boost" >/dev/null
    else
        echo "[-] Notice: Writing fan boost requires root privileges." >&2
        echo "    Tip: Run with 'sudo awcc-mode ${MODE}' or configure passwordless sudo via 'sudo ./bin/setup-sudoers.sh'." >&2
    fi

    # Actuate Game Shift toggle if exposed by alienware-wmi
    if [[ -f "${AW_HWMON}/gmode" ]]; then
        G_TARGET=$([[ "${BOOST}" -eq 255 ]] && echo 1 || echo 0)
        if [[ -w "${AW_HWMON}/gmode" ]]; then
            echo "${G_TARGET}" > "${AW_HWMON}/gmode" 2>/dev/null || true
        elif sudo -n tee "${AW_HWMON}/gmode" >/dev/null 2>&1 <<< "${G_TARGET}"; then
            :
        fi
    fi

    READ_B1="$(cat "${AW_HWMON}/fan1_boost" 2>/dev/null || echo '?')"
    READ_B2="$(cat "${AW_HWMON}/fan2_boost" 2>/dev/null || echo '?')"
    if { [[ "${BOOST}" -eq 255 ]] && [[ "${READ_B1}" -ge 240 ]] && [[ "${READ_B2}" -ge 240 ]]; } || { [[ "${READ_B1}" == "${BOOST}" ]] && [[ "${READ_B2}" == "${BOOST}" ]]; }; then
        echo "[+] Fan boost readback verified: Fan 1 = ${READ_B1}/255 | Fan 2 = ${READ_B2}/255"
    else
        echo "[-] Error: Fan boost readback mismatch: requested ${BOOST}, got Fan 1: ${READ_B1}, Fan 2: ${READ_B2}" >&2
        exit 1
    fi
else
    echo "[-] Notice: alienware_wmi driver not loaded (hardware fan boost node absent)."
    echo "    Run 'sudo ./bin/load-driver.sh' to actuate fan boost offset."
fi

# 3. Synchronize Game Shift State File
STATE_FILE="/tmp/.awcc_gmode_state"
if [[ "$BOOST" -eq 255 ]]; then
    echo "ON" > "$STATE_FILE"
else
    echo "OFF" > "$STATE_FILE"
fi

# 4. Handle flag transitions
AWCCCTL_BIN=""
if [[ -x "${ROOT_DIR}/target/release/awccctl" ]]; then
    AWCCCTL_BIN="${ROOT_DIR}/target/release/awccctl"
elif [[ -x "${HOME}/.local/bin/awccctl" ]]; then
    AWCCCTL_BIN="${HOME}/.local/bin/awccctl"
elif [[ -x "${ROOT_DIR}/target/debug/awccctl" ]]; then
    AWCCCTL_BIN="${ROOT_DIR}/target/debug/awccctl"
elif command -v awccctl >/dev/null 2>&1; then
    AWCCCTL_BIN="$(command -v awccctl)"
fi

if [[ "$FLAG" == "--quiet" ]] || [[ "$FLAG" == "-q" ]]; then
    exit 0
elif [[ "$FLAG" == "--tui" ]] || [[ "$FLAG" == "--dashboard" ]]; then
    echo "=========================================================================="
    echo "[+] ${TITLE} activated successfully."
    echo "=========================================================================="
    echo "[*] Launching interactive TrueColor dashboard..."
    sleep 0.4
    if [[ -n "$DASHBOARD_SCRIPT" ]]; then
        exec python3 "$DASHBOARD_SCRIPT"
    fi
elif [[ "$FLAG" == "--track" ]] || [[ "$FLAG" == "-t" ]] || [[ "$FLAG" == "--watch" ]] || [[ "$FLAG" == "-w" ]] || [[ "$FLAG" == "--monitor" ]] || [[ "$FLAG" == "-m" ]]; then
    echo "=========================================================================="
    echo "[+] ${TITLE} activated successfully."
    echo "=========================================================================="
    echo "[*] Entering continuous real-time hardware monitoring (Press Ctrl+C to stop)..."
    sleep 0.4
    if [[ -n "${AWCCCTL_BIN}" ]]; then
        exec "${AWCCCTL_BIN}" track --interval 1.0
    elif [[ -x "${SCRIPT_DIR}/awcc-track" ]]; then
        exec "${SCRIPT_DIR}/awcc-track" --cli
    elif [[ -n "$DASHBOARD_SCRIPT" ]]; then
        exec python3 "$DASHBOARD_SCRIPT"
    fi
elif [[ "$FLAG" == "--snapshot" ]] || [[ "$FLAG" == "-s" ]] || [[ "$FLAG" == "--once" ]] || [[ "$FLAG" == "--no-monitor" ]]; then
    echo "[3/3] Current System & Thermal Status:"
    if [[ -n "${AWCCCTL_BIN}" ]]; then
        "${AWCCCTL_BIN}" status
    elif [[ -n "$DASHBOARD_SCRIPT" ]]; then
        python3 "$DASHBOARD_SCRIPT" --snapshot
    fi
    echo "=========================================================================="
    echo "[+] ${TITLE} activated successfully."
    echo "=========================================================================="
else
    echo "=========================================================================="
    echo "[+] ${TITLE} activated successfully."
    echo "=========================================================================="

    # If running in an interactive terminal, enter continuous live monitoring until stopped
    if [[ -t 1 ]] && [[ -t 0 ]] && [[ "${AWCC_NONINTERACTIVE:-0}" != "1" ]] && [[ -z "${CI:-}" ]]; then
        echo ""
        echo "[*] Entering continuous real-time monitoring (Press Ctrl+C to stop)..."
        sleep 0.5
        if [[ -n "${AWCCCTL_BIN}" ]]; then
            exec "${AWCCCTL_BIN}" track --interval 1.0
        elif [[ -x "${SCRIPT_DIR}/awcc-track" ]]; then
            exec "${SCRIPT_DIR}/awcc-track" --cli
        elif [[ -n "$DASHBOARD_SCRIPT" ]]; then
            exec python3 "$DASHBOARD_SCRIPT"
        fi
    else
        # Single-shot snapshot for non-interactive / piped / automated invocations
        echo "[3/3] Current System & Thermal Status:"
        if [[ -n "${AWCCCTL_BIN}" ]]; then
            "${AWCCCTL_BIN}" status
        fi
    fi
fi
