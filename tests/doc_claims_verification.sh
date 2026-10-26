#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== AWCC Linux: Documentation & Safety Claims Verification ==="
CHECKS_PASSED=0

# 1. Verify no world-writable recommendations in docs
echo "[Check 1] Asserting no world-writable permissions recommended in docs..."
if grep -rnE "^\s*(\$|sudo\s+)?chmod\s+(0?666|777)" "${REPO_DIR}/docs" "${REPO_DIR}/README.md" 2>/dev/null; then
    echo "FAIL: Found world-writable chmod instructions in documentation!" >&2
    exit 1
fi
echo "  PASS: Documentation contains no insecure world-writable instructions."
CHECKS_PASSED=$(( CHECKS_PASSED + 1 ))

# 2. Verify no hardcoded user home paths in docs or specs
echo "[Check 2] Asserting no hardcoded user home paths in docs or specs..."
if grep -rn "/home/unknownuser" "${REPO_DIR}/docs"/*.md "${REPO_DIR}/README.md" "${REPO_DIR}/packaging" "${REPO_DIR}/systemd" 2>/dev/null; then
    echo "FAIL: Found hardcoded user paths in docs or deployment files!" >&2
    exit 1
fi
echo "  PASS: Documentation and specs use portable dynamic paths."
CHECKS_PASSED=$(( CHECKS_PASSED + 1 ))

# 3. Verify rhel-performance boost 255 claim in README.md
echo "[Check 3] Asserting README.md documents 255 boost for performance / ultra..."
if ! grep -q "255/255" "${REPO_DIR}/README.md"; then
    echo "FAIL: README.md does not document 255/255 maximum fan boost!" >&2
    exit 1
fi
echo "  PASS: README.md correctly documents 255/255 maximum boost."
CHECKS_PASSED=$(( CHECKS_PASSED + 1 ))

# 4. Verify root LICENSE exists and specifies dual licensing
echo "[Check 4] Asserting root LICENSE existence and dual licensing terms..."
if [[ ! -f "${REPO_DIR}/LICENSE" ]]; then
    echo "FAIL: Root LICENSE file is missing!" >&2
    exit 1
fi
if ! grep -q "GPL-2.0-or-later" "${REPO_DIR}/LICENSE" || ! grep -q "MIT" "${REPO_DIR}/LICENSE"; then
    echo "FAIL: Root LICENSE does not document GPL-2.0-or-later and MIT licenses!" >&2
    exit 1
fi
echo "  PASS: Root LICENSE accurately documents dual license structure."
CHECKS_PASSED=$(( CHECKS_PASSED + 1 ))

# 5. Verify packaging spec matches dual license
echo "[Check 5] Asserting RPM spec specifies GPL-2.0-or-later AND MIT..."
if ! grep -q "License:        GPL-2.0-or-later AND MIT" "${REPO_DIR}/packaging/rhel/awcc-linux.spec"; then
    echo "FAIL: RPM spec license does not match dual license structure!" >&2
    exit 1
fi
echo "  PASS: RPM spec matches project license metadata."
CHECKS_PASSED=$(( CHECKS_PASSED + 1 ))

# 6. Verify provenance document exists
echo "[Check 6] Asserting docs/provenance-and-licenses.md existence..."
if [[ ! -f "${REPO_DIR}/docs/provenance-and-licenses.md" ]]; then
    echo "FAIL: docs/provenance-and-licenses.md is missing!" >&2
    exit 1
fi
echo "  PASS: Provenance documentation verified."
CHECKS_PASSED=$(( CHECKS_PASSED + 1 ))

echo "================================================================"
echo "PASS: All ${CHECKS_PASSED} documentation and provenance assertions passed!"
