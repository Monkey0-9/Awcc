#!/bin/bash
set -euo pipefail

MODE="${1:-}"
FLAG="${2:-}"

AW_HWMON=""
for d in /sys/class/hwmon/hwmon*; do
    if [[ -f "$d/name" ]] && [[ "$(cat "$d/name" 2>/dev/null)" == "alienware_wmi" ]]; then
        AW_HWMON="$d"
        break
    fi
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
DASHBOARD_SCRIPT=""
for candidate in \
    "${SCRIPT_DIR}/awcc-lux-dashboard.py" \
    "/home/unknownuser/.local/bin/awcc-lux-dashboard.py" \
    "/usr/local/bin/awcc-lux-dashboard.py" \
    "/usr/bin/awcc-lux-dashboard.py"; do
    if [[ -f "$candidate" ]]; then
        DASHBOARD_SCRIPT="$candidate"
        break
    fi
done

# If no mode argument is provided or a flag is passed, launch the live monitor on current hardware state
if [[ -z "${MODE}" ]] || [[ "${MODE}" =~ ^- ]] || [[ "${MODE,,}" == "monitor" ]] || [[ "${MODE,,}" == "live" ]]; then
    if [[ -n "$DASHBOARD_SCRIPT" ]]; then
        exec python3 "$DASHBOARD_SCRIPT" "$@"
    else
        echo "[-] Error: awcc-lux-dashboard.py not found." >&2
        exit 1
    fi
fi

case "${MODE,,}" in
    battery|powersave|low|15|10|26)
        PROFILE="quiet"
        BOOST=15
        TITLE="BATTERY SAVER (Silent Whisper, Boost 15, ~1,250 RPM)"
        ;;
    quiet|silent|35|20|51)
        PROFILE="quiet"
        BOOST=35
        TITLE="QUIET (Acoustic Silence, Boost 35, ~1,650 RPM)"
        ;;
    balance|balanced|50|40|102)
        PROFILE="balanced"
        BOOST=50
        TITLE="BALANCE (Daily Productivity, Boost 50, ~2,460 RPM)"
        ;;
    mid|65|60|153)
        PROFILE="performance"
        BOOST=65
        TITLE="MID (Intermediate Performance, Boost 65, ~3,400 RPM)"
        ;;
    high|performance|85|80|204)
        PROFILE="performance"
        BOOST=85
        TITLE="HIGH PERFORMANCE (Sustained Workloads, Boost 85, ~4,200 RPM)"
        ;;
    ultra|gmode|g-mode|max|255|100)
        PROFILE="performance"
        BOOST=255
        TITLE="ULTRA PERFORMANCE & GAME SHIFT (G-MODE Max Fans, Boost 255, 5,029 RPM)"
        ;;
    *)
        echo "[-] Unknown mode '${MODE}'. Choose from: battery, quiet, balance, mid, high, ultra." >&2
        exit 1
        ;;
esac

# 1. Apply Platform Profile
if [[ -w /sys/firmware/acpi/platform_profile ]]; then
    echo "${PROFILE}" > /sys/firmware/acpi/platform_profile 2>/dev/null || true
fi

# 2. Apply Fan Boost
if [[ -n "${AW_HWMON}" ]]; then
    if [[ -w "${AW_HWMON}/fan1_boost" ]]; then
        echo "${BOOST}" > "${AW_HWMON}/fan1_boost" 2>/dev/null || true
        echo "${BOOST}" > "${AW_HWMON}/fan2_boost" 2>/dev/null || true
    fi
fi

# 3. Synchronize Game Shift State File
STATE_FILE="/tmp/.awcc_gmode_state"
if [[ "$BOOST" -ge 240 ]]; then
    echo "ON" > "$STATE_FILE"
else
    echo "OFF" > "$STATE_FILE"
fi

# 4. Execution Mode: Live Monitor (Default) vs Snapshot vs Silent
if [[ "$FLAG" == "--quiet" ]] || [[ "$FLAG" == "-q" ]]; then
    exit 0
elif [[ "$FLAG" == "--snapshot" ]] || [[ "$FLAG" == "-s" ]] || [[ "$FLAG" == "--once" ]]; then
    if [[ -n "$DASHBOARD_SCRIPT" ]]; then
        python3 "$DASHBOARD_SCRIPT" --snapshot
    fi
else
    # Automatically start live tracking monitor
    if [[ -n "$DASHBOARD_SCRIPT" ]]; then
        exec python3 "$DASHBOARD_SCRIPT"
    fi
fi
