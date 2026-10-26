#!/bin/bash
set -euo pipefail

echo "[*] Registering Ultra-Luxurious Game Shift Keybindings in GNOME Desktop..."

# Comprehensive list of hotkeys:
# F9, Super+F9, Ctrl+Alt+G, Super+G, and all vendor multimedia launch keys
KEYS=(
    "F9"
    "<Super>F9"
    "<Primary><Alt>g"
    "<Super>g"
    "XF86Launch1"
    "XF86Launch2"
    "XF86Launch3"
    "XF86Launch4"
    "XF86LaunchA"
    "XF86LaunchB"
    "XF86Tools"
    "XF86Game"
)

TARGET_CMD=""
if command -v awcc-gameshift-toggle >/dev/null 2>&1; then
    TARGET_CMD="$(command -v awcc-gameshift-toggle)"
elif [[ -x "${HOME}/.local/bin/awcc-gameshift-toggle" ]]; then
    TARGET_CMD="${HOME}/.local/bin/awcc-gameshift-toggle"
elif [[ -x "/usr/local/bin/awcc-gameshift-toggle" ]]; then
    TARGET_CMD="/usr/local/bin/awcc-gameshift-toggle"
else
    TARGET_CMD="awcc-gameshift-toggle"
fi

EXISTING=$(gsettings get org.gnome.settings-daemon.plugins.media-keys custom-keybindings 2>/dev/null || echo "[]")
EXISTING_ITEMS=()
if [[ "$EXISTING" != "@as []" && "$EXISTING" != "[]" ]]; then
    # Strip brackets and split by comma
    CLEAN="${EXISTING#[}"
    CLEAN="${CLEAN%]}"
    IFS=',' read -ra ADDR <<< "$CLEAN"
    for i in "${ADDR[@]}"; do
        TRIMMED=$(echo "$i" | xargs)
        if [[ -n "$TRIMMED" ]]; then
            EXISTING_ITEMS+=("$TRIMMED")
        fi
    done
fi

NEW_ITEMS=("${EXISTING_ITEMS[@]}")
INDEX=0

for k in "${KEYS[@]}"; do
    PATH_ID="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/awcc-gmode-${INDEX}/"
    PATH_QUOTE="'$PATH_ID'"
    
    # Only append if not already present
    if [[ ! " ${NEW_ITEMS[*]} " =~ " ${PATH_QUOTE} " ]]; then
        NEW_ITEMS+=("$PATH_QUOTE")
    fi
    
    gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$PATH_ID name "Alienware Game Shift ($k)" 2>/dev/null || true
    gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$PATH_ID command "$TARGET_CMD" 2>/dev/null || true
    gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$PATH_ID binding "$k" 2>/dev/null || true
    INDEX=$(( INDEX + 1 ))
done

ARRAY_STR="[$(IFS=,; echo "${NEW_ITEMS[*]}")]"
gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings "$ARRAY_STR" 2>/dev/null || true

echo "[+] Successfully registered Game Shift hotkeys in GNOME without overwriting prior shortcuts!"
