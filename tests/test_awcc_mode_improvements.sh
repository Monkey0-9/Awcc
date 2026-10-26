#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
ROOT_DIR="$(dirname "${SCRIPT_DIR}")"

echo "=== Running AWCC Mode & Dashboard Improvement Tests ==="

echo "[1/4] Checking python syntax on bin/awcc-lux-dashboard.py..."
python3 -m py_compile "${ROOT_DIR}/bin/awcc-lux-dashboard.py"
echo "[+] Python syntax check passed."

echo "[2/4] Testing bin/awcc-lux-dashboard.py --snapshot across all 6 themes..."
for th in cryo cyberpunk darkcore solar matrix vaporwave; do
    echo "  -> Testing theme '${th}'..."
    OUT="$(python3 "${ROOT_DIR}/bin/awcc-lux-dashboard.py" --snapshot --theme "${th}")"
    echo "${OUT}" | grep -q "ALIENWARE COMMAND CENTER" || { echo "[-] Failed: Missing header in theme ${th}"; exit 1; }
    echo "${OUT}" | grep -q "COOLING TURBINES" || { echo "[-] Failed: Missing turbines in theme ${th}"; exit 1; }
    echo "${OUT}" | grep -q "SILICON COMPUTATION" || { echo "[-] Failed: Missing silicon in theme ${th}"; exit 1; }
done
echo "[+] All 6 color grading themes rendered snapshot cleanly without errors."

echo "[3/4] Testing bin/awcc-mode dynamic machine detection..."
# Verify that awcc-mode does not contain raw hardcoded echo " Target Machine: Dell G15 5530"
if grep -q 'echo " Target Machine: Dell G15 5530"' "${ROOT_DIR}/bin/awcc-mode"; then
    echo "[-] Failed: Hardcoded Dell G15 5530 string still present in awcc-mode!"
    exit 1
fi
echo "[+] Dynamic DMI identification in awcc-mode verified."

echo "[4/4] Testing bin/awcc-track syntax and existence..."
bash -n "${ROOT_DIR}/bin/awcc-track"
bash -n "${ROOT_DIR}/bin/sync-local-bin.sh"
bash -n "${ROOT_DIR}/bin/setup-sudoers.sh"
echo "[+] Helper scripts syntax verified."

echo "=== All AWCC Mode & Dashboard Improvement Tests Passed! ==="
