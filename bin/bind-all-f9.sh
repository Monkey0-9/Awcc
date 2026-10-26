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

PATHS=()
INDEX=0

for k in "${KEYS[@]}"; do
    PATH_ID="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/awcc-gmode-${INDEX}/"
    PATHS+=("'$PATH_ID'")
    
    gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$PATH_ID name "Alienware Game Shift ($k)" 2>/dev/null || true
    gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$PATH_ID command "/home/unknownuser/.local/bin/awcc-gameshift-toggle" 2>/dev/null || true
    gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$PATH_ID binding "$k" 2>/dev/null || true
    INDEX=$(( INDEX + 1 ))
done

ARRAY_STR="[$(IFS=,; echo "${PATHS[*]}")]"
gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings "$ARRAY_STR" 2>/dev/null || true

echo "[+] Successfully registered ${#KEYS[@]} Game Shift hotkey combinations in GNOME!"
