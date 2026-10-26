#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== AWCC Linux: Shell Script Syntax Inspection ==="
SCRIPTS_CHECKED=0

while IFS= read -r s; do
    # Skip backup files, python files, editor temps, compiled artifacts
    if [[ "$s" =~ \.(bak|tmp|swp|py|pyc|o|ko|mod|cmd|class)$ ]] || [[ "$s" =~ ~$ ]]; then
        continue
    fi
    # Check if executable shell script: either .sh or shebang with sh/bash
    if [[ "$s" =~ \.sh$ ]] || [[ "$(head -n 1 "$s" 2>/dev/null)" =~ ^#!.*(bash|sh) ]]; then
        bash -n "$s"
        SCRIPTS_CHECKED=$(( SCRIPTS_CHECKED + 1 ))
    fi
done < <(find "${REPO_DIR}/bin" "${REPO_DIR}/kernel/scripts" "${REPO_DIR}/tests" -maxdepth 2 -type f | sort)

echo "PASS: All ${SCRIPTS_CHECKED} shell scripts passed bash -n syntax check."
