#!/usr/bin/env bash
# ==============================================================================
# tests/gui_smoke_test.sh
# Smoke test harness for the AWCC Linux modern GTK4 / Libadwaita desktop GUI.
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
GUI_BIN="${REPO_DIR}/bin/awcc-gui"
DESKTOP_FILE="${REPO_DIR}/packaging/awcc-gui.desktop"
ICON_FILE="${REPO_DIR}/packaging/icons/awcc-gui.svg"

echo "=== Running Desktop GUI Smoke Test ==="

# 1. Binary existence and executable permission
if [[ ! -x "$GUI_BIN" ]]; then
    echo "[FAIL] GUI binary ${GUI_BIN} does not exist or is not executable"
    exit 1
fi
echo "[PASS] GUI binary exists and is executable"

# 2. Version check
VER_OUTPUT=$("$GUI_BIN" --version)
if [[ "$VER_OUTPUT" =~ awcc-gui\ [0-9]+\.[0-9]+ ]]; then
    echo "[PASS] --version output: ${VER_OUTPUT}"
else
    echo "[FAIL] Unexpected --version output: ${VER_OUTPUT}"
    exit 1
fi

# 3. Help check
HELP_OUTPUT=$("$GUI_BIN" --help)
if echo "$HELP_OUTPUT" | grep -q "Usage: awcc-gui"; then
    echo "[PASS] --help output verified"
else
    echo "[FAIL] Unexpected --help output"
    exit 1
fi

# 4. Headless test mode initialization
if python3 -c "import gi; gi.require_version('Gtk', '4.0'); gi.require_version('Adw', '1')" >/dev/null 2>&1; then
    TEST_OUTPUT=$("$GUI_BIN" --test 2>&1)
    if echo "$TEST_OUTPUT" | grep -q "Headless GUI verification test passed"; then
        echo "[PASS] Headless UI initialization and widget binding verified"
    else
        echo "[FAIL] Headless test did not succeed: ${TEST_OUTPUT}"
        exit 1
    fi
else
    echo "[SKIP] GTK4 / Libadwaita not installed in this environment; skipping active widget test"
fi

# 5. Desktop file verification
if [[ -f "$DESKTOP_FILE" ]]; then
    if grep -q "Name=AWCC Hardware Control" "$DESKTOP_FILE" && grep -q "Exec=awcc-gui" "$DESKTOP_FILE"; then
        echo "[PASS] XDG Desktop entry valid: ${DESKTOP_FILE}"
    else
        echo "[FAIL] Desktop entry missing required fields"
        exit 1
    fi
else
    echo "[FAIL] Desktop file ${DESKTOP_FILE} missing"
    exit 1
fi

# 6. SVG Icon verification
if [[ -f "$ICON_FILE" ]]; then
    if grep -q "<svg" "$ICON_FILE" && grep -q "</svg>" "$ICON_FILE"; then
        echo "[PASS] FreeDesktop SVG icon verified: ${ICON_FILE}"
    else
        echo "[FAIL] SVG icon does not appear to be valid XML/SVG"
        exit 1
    fi
else
    echo "[FAIL] Icon file ${ICON_FILE} missing"
    exit 1
fi

echo "=== All Desktop GUI Tests Passed ==="
