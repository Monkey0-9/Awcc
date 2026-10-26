#!/usr/bin/env bash
set -euo pipefail

# Fail if chmod 0666 or 666 appears in any executable, script, spec, or rule
if grep -rnE "chmod (0?666|777)" bin/ packaging/ systemd/ 2>/dev/null; then
    echo "FAIL: Found world-writable chmod instructions in repository!"
    exit 1
fi
echo "PASS: No world-writable permission patterns found."
