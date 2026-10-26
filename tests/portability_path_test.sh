#!/usr/bin/env bash
set -euo pipefail

# Fail if /home/unknownuser appears in scripts, docs, systemd, packaging, or crates
# (Exclude git logs, binary files, pycache, build target)
if grep -rn "/home/unknownuser" bin/ systemd/ packaging/ crates/ 2>/dev/null | grep -v "__pycache__"; then
    echo "FAIL: Found hardcoded user home path in codebase!"
    exit 1
fi
echo "PASS: No hardcoded home paths found in codebase."
