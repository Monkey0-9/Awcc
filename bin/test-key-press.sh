#!/bin/bash
set -euo pipefail

echo "================================================================="
echo " Dell G15 5530: Key Event Diagnostic & Listener"
echo "================================================================="
echo "Press your F9 (or Fn+F9) key on the keyboard right now..."
echo "(Will listen for 5 seconds for any keyboard activity)"
echo "-----------------------------------------------------------------"

# Use libinput record or debug-events if available
if command -v libinput >/dev/null 2>&1; then
    timeout 5 libinput debug-events --show-keycodes 2>&1 || true
fi
