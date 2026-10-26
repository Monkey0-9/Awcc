#!/usr/bin/env bash
set -euo pipefail

echo "=== AWCC Linux: Shell Script Syntax Inspection ==="
SCRIPTS_CHECKED=0
for s in bin/*.sh bin/awcc-* bin/rhel-*; do
    if [[ -f "$s" ]] && [[ ! "$s" =~ \.py$ ]] && [[ ! "$s" =~ \.pyc$ ]]; then
        bash -n "$s"
        SCRIPTS_CHECKED=$(( SCRIPTS_CHECKED + 1 ))
    fi
done

echo "PASS: All ${SCRIPTS_CHECKED} shell scripts passed bash -n syntax check."
