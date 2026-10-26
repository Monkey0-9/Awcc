#!/usr/bin/env bash
set -euo pipefail

# Verify bin/rhel-performance passes boost 255, not 204
if grep -q "204" bin/rhel-performance; then
    echo "FAIL: bin/rhel-performance still passes 204 (80%) instead of 255 (100%)!"
    exit 1
fi

if ! grep -q "255" bin/rhel-performance; then
    echo "FAIL: bin/rhel-performance does not pass 255 boost!"
    exit 1
fi

echo "PASS: rhel-performance correctly specifies 255 (100% max fan boost)."
