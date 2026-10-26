#!/bin/bash
set -euo pipefail

echo "=== Dell G15 5530 Game Shift (F9) Keybind Setup ==="

# Check desktop environment
if command -v gsettings >/dev/null 2>&1; then
    echo "[*] Detected GNOME / Desktop environment. Registering custom F9 keybinding..."

    KEYPATH="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom-awcc-gmode/"
    
    # Add to list of custom keybindings if not present
    EXISTING=$(gsettings get org.gnome.settings-daemon.plugins.media-keys custom-keybindings 2>/dev/null || echo "[]")
    
    if [[ "$EXISTING" != *"$KEYPATH"* ]]; then
        if [[ "$EXISTING" == "@as []" || "$EXISTING" == "[]" ]]; then
            NEW_BINDINGS="['$KEYPATH']"
        else
            NEW_BINDINGS="${EXISTING%]*}, '$KEYPATH']"
        fi
        gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings "$NEW_BINDINGS" 2>/dev/null || true
    fi

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

    gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KEYPATH name "Dell AWCC Game Shift (F9)" 2>/dev/null || true
    gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KEYPATH command "$TARGET_CMD" 2>/dev/null || true
    gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KEYPATH binding "F9" 2>/dev/null || true

    echo "[+] Successfully bound F9 to ${TARGET_CMD}!"
fi

echo "[*] You can test toggling Game Shift anytime by running: 'awcc-gmode' or pressing F9."
