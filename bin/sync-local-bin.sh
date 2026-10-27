#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
ROOT_DIR="$(dirname "${SCRIPT_DIR}")"
LOCAL_BIN="${HOME}/.local/bin"

echo "=== AWCC Linux: Synchronizing Binaries & Scripts to ${LOCAL_BIN} ==="
mkdir -p "${LOCAL_BIN}"

# 1. Build release binaries
echo "[1/4] Compiling optimized release binaries (awccctl, awccd)..."
(cd "${ROOT_DIR}" && cargo build --release)

# 2. Sync compiled Rust binaries
echo "[2/4] Installing awccctl and awccd to ${LOCAL_BIN}..."
cp -f "${ROOT_DIR}/target/release/awccctl" "${LOCAL_BIN}/awccctl"
cp -f "${ROOT_DIR}/target/release/awccd" "${LOCAL_BIN}/awccd"
chmod +x "${LOCAL_BIN}/awccctl" "${LOCAL_BIN}/awccd"

# 3. Sync dashboard, GUI, & mode scripts
echo "[3/4] Installing dashboard, GUI, and mode actuation scripts..."
cp -f "${ROOT_DIR}/bin/awcc-lux-dashboard.py" "${LOCAL_BIN}/awcc-lux-dashboard.py"
cp -f "${ROOT_DIR}/bin/awcc-gui" "${LOCAL_BIN}/awcc-gui"
cp -f "${ROOT_DIR}/bin/awcc-mode" "${LOCAL_BIN}/awcc-mode"
cp -f "${ROOT_DIR}/bin/awcc-mode.sh" "${LOCAL_BIN}/awcc-mode.sh"
cp -f "${ROOT_DIR}/bin/awcc-track" "${LOCAL_BIN}/awcc-track"
cp -f "${ROOT_DIR}/bin/awcc-monitor" "${LOCAL_BIN}/awcc-monitor"
cp -f "${ROOT_DIR}/bin/awcc-gameshift-toggle.sh" "${LOCAL_BIN}/awcc-gameshift-toggle"
ln -sf "${LOCAL_BIN}/awcc-gameshift-toggle" "${LOCAL_BIN}/gmode"
chmod +x \
    "${LOCAL_BIN}/awcc-lux-dashboard.py" \
    "${LOCAL_BIN}/awcc-gui" \
    "${LOCAL_BIN}/awcc-mode" \
    "${LOCAL_BIN}/awcc-mode.sh" \
    "${LOCAL_BIN}/awcc-track" \
    "${LOCAL_BIN}/awcc-monitor" \
    "${LOCAL_BIN}/awcc-gameshift-toggle"

# Install desktop icon, entry, shell completions, and man pages
mkdir -p "${HOME}/.local/share/applications" "${HOME}/.local/share/icons/hicolor/scalable/apps"
if [[ -f "${ROOT_DIR}/packaging/awcc-gui.desktop" ]]; then
    cp -f "${ROOT_DIR}/packaging/awcc-gui.desktop" "${HOME}/.local/share/applications/awcc-gui.desktop"
fi
if [[ -f "${ROOT_DIR}/packaging/icons/awcc-gui.svg" ]]; then
    cp -f "${ROOT_DIR}/packaging/icons/awcc-gui.svg" "${HOME}/.local/share/icons/hicolor/scalable/apps/awcc-gui.svg"
fi

mkdir -p "${HOME}/.local/share/bash-completion/completions"
if [[ -f "${ROOT_DIR}/completions/bash/awccctl" ]]; then
    cp -f "${ROOT_DIR}/completions/bash/awccctl" "${HOME}/.local/share/bash-completion/completions/awccctl"
fi

mkdir -p "${HOME}/.local/share/man/man1"
for mp in awccctl.1 awcc-lux-dashboard.1 awcc-mode.1; do
    if [[ -f "${ROOT_DIR}/man/man1/${mp}" ]]; then
        cp -f "${ROOT_DIR}/man/man1/${mp}" "${HOME}/.local/share/man/man1/${mp}"
    fi
done

# 4. Sync profile actuation shortcuts
echo "[4/4] Synchronizing mode shortcuts (awcc-battery, awcc-quiet, awcc-balanced, awcc-performance, awcc-ultra)..."
for profile in battery quiet balanced balance performance mid high ultra; do
    SHORTCUT="${LOCAL_BIN}/awcc-${profile}"
    cat << 'EOF' > "${SHORTCUT}"
#!/usr/bin/env bash
set -euo pipefail
PROFILE="$(basename "$0" | sed 's/^awcc-//')"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
if [[ -x "${SCRIPT_DIR}/awcc-mode" ]]; then
    exec "${SCRIPT_DIR}/awcc-mode" "${PROFILE}" "$@"
elif command -v awcc-mode >/dev/null 2>&1; then
    exec awcc-mode "${PROFILE}" "$@"
else
    echo "[-] Error: awcc-mode not found in PATH or ~/.local/bin." >&2
    exit 1
fi
EOF
    chmod +x "${SHORTCUT}"
    ln -sf "${SHORTCUT}" "${LOCAL_BIN}/${profile}"
done

echo "=========================================================================="
echo "[+] All AWCC Linux binaries and scripts synchronized successfully to ${LOCAL_BIN}!"
echo "    - awccctl status         (Comprehensive telemetry matrix)"
echo "    - awccctl track          (Real-time CLI monitoring)"
echo "    - awcc-lux-dashboard.py  (Cybernetic TrueColor cockpit)"
echo "    - awcc-quiet [--track]   (Silent mode actuation + live tracking)"
echo "    - awcc-track             (Dedicated live telemetry tracking)"
echo "=========================================================================="
