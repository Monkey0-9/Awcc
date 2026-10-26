#!/usr/bin/env bash
set -euo pipefail

echo "=== AWCC Linux: Removing Game Shift GNOME Keybindings ==="

if ! command -v gsettings >/dev/null 2>&1; then
    echo "[!] gsettings not found; skipping GNOME shortcut unregistration."
    exit 0
fi

EXISTING=$(gsettings get org.gnome.settings-daemon.plugins.media-keys custom-keybindings 2>/dev/null || echo "[]")
if [[ "$EXISTING" == "@as []" || "$EXISTING" == "[]" ]]; then
    echo "[+] No custom keybindings found."
    exit 0
fi

CLEAN="${EXISTING#[}"
CLEAN="${CLEAN%]}"
IFS=',' read -ra ADDR <<< "$CLEAN"

KEPT_ITEMS=()
for i in "${ADDR[@]}"; do
    TRIMMED=$(echo "$i" | xargs)
    if [[ -n "$TRIMMED" ]]; then
        # Check if it belongs to awcc
        if [[ "$TRIMMED" =~ awcc-gmode || "$TRIMMED" =~ custom-awcc-gmode ]]; then
            # Strip quotes for gsettings reset
            RAW_PATH="${TRIMMED#\'}"
            RAW_PATH="${RAW_PATH%\'}"
            gsettings reset-recursively "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:${RAW_PATH}" 2>/dev/null || true
            echo "  Removed AWCC keybinding: ${RAW_PATH}"
        else
            KEPT_ITEMS+=("$TRIMMED")
        fi
    fi
done

if [[ ${#KEPT_ITEMS[@]} -eq 0 ]]; then
    gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings "@as []" 2>/dev/null || true
else
    ARRAY_STR="[$(IFS=,; echo "${KEPT_ITEMS[*]}")]"
    gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings "$ARRAY_STR" 2>/dev/null || true
fi

echo "[+] Successfully unregistered AWCC Game Shift keybindings and restored prior user shortcuts."
