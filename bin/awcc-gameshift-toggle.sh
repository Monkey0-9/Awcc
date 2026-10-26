#!/bin/bash
set -euo pipefail

# Ensure PATH includes user and system binaries
export PATH="/home/unknownuser/.local/bin:/home/unknownuser/Awcc/bin:/usr/local/bin:/usr/bin:$PATH"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
AWCC_MODE_CMD=""
for candidate in \
    "${SCRIPT_DIR}/awcc-mode.sh" \
    "${SCRIPT_DIR}/awcc-mode" \
    "/home/unknownuser/.local/bin/awcc-mode" \
    "/usr/local/bin/awcc-mode" \
    "/usr/bin/awcc-mode"; do
    if [[ -x "$candidate" ]]; then
        AWCC_MODE_CMD="$candidate"
        break
    fi
done
if [[ -z "$AWCC_MODE_CMD" ]]; then
    AWCC_MODE_CMD="awcc-mode"
fi

STATE_FILE="/tmp/.awcc_gmode_state"

# Discover alienware_wmi hwmon
AW_HWMON=""
for d in /sys/class/hwmon/hwmon*; do
    if [[ -f "$d/name" ]] && [[ "$(cat "$d/name" 2>/dev/null)" == "alienware_wmi" ]]; then
        AW_HWMON="$d"
        break
    fi
done

# Check actual hardware state
CURRENT_BOOST=0
if [[ -n "$AW_HWMON" ]] && [[ -f "$AW_HWMON/fan1_boost" ]]; then
    CURRENT_BOOST=$(cat "$AW_HWMON/fan1_boost" 2>/dev/null || echo 0)
fi

# Determine if G-Mode is currently active
IS_GMODE_ACTIVE=0
if [[ "$CURRENT_BOOST" -ge 240 ]]; then
    IS_GMODE_ACTIVE=1
fi

IS_TTY=0
if [[ -t 1 ]] && [[ "${1:-}" != "--quiet" ]] && [[ "${1:-}" != "-q" ]]; then
    IS_TTY=1
fi

if [[ "$IS_GMODE_ACTIVE" -eq 1 ]]; then
    echo "OFF" > "$STATE_FILE"
    # Apply hardware balanced mode immediately
    "$AWCC_MODE_CMD" balance --quiet
    ( notify-send -u normal -t 3000 -i preferences-system-power \
        "❄ ALIENWARE GAME SHIFT: OFF" \
        "BALANCED MODE RESTORED\nFans: 40% Boost (~1,980 RPM)\nProfile: Balanced System" 2>/dev/null || true ) &
    ( canberra-gtk-play -i service-logout 2>/dev/null || true ) &
    if [[ "$IS_TTY" -eq 1 ]]; then
        exec "$AWCC_MODE_CMD" balance
    fi
else
    echo "ON" > "$STATE_FILE"
    # Apply hardware ultra / Game Shift mode immediately
    "$AWCC_MODE_CMD" ultra --quiet
    ( notify-send -u critical -t 3500 -i preferences-system-performance \
        "⚡ ALIENWARE GAME SHIFT: ON" \
        "G-MODE ULTRA ACTIVATED\nFans: 100% Boost (~5,000 RPM Max Speed)\nProfile: Extreme Performance" 2>/dev/null || true ) &
    ( canberra-gtk-play -i service-login 2>/dev/null || true ) &
    if [[ "$IS_TTY" -eq 1 ]]; then
        exec "$AWCC_MODE_CMD" ultra
    fi
fi
