#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
ROOT_DIR="$(dirname "${SCRIPT_DIR}")"
AWCCCTL="${ROOT_DIR}/target/release/awccctl"

echo "=== Running AWCCCTL Extended Telemetry & Tracking CLI Tests ==="

if [[ ! -x "${AWCCCTL}" ]]; then
    echo "[-] Building release awccctl..."
    cargo build --release --bin awccctl
fi

echo "[1/4] Testing awccctl status output format..."
STATUS_OUT="$(${AWCCCTL} status)"
echo "${STATUS_OUT}" | grep -q "AWCC Linux:" || { echo "[-] Failed: Missing title in status output"; exit 1; }
echo "${STATUS_OUT}" | grep -q "SILICON COMPUTATION & PERFORMANCE" || { echo "[-] Failed: Missing silicon section"; exit 1; }
echo "${STATUS_OUT}" | grep -q "THERMAL SENSOR MATRIX" || { echo "[-] Failed: Missing thermal section"; exit 1; }
echo "${STATUS_OUT}" | grep -q "COOLING TURBINES" || { echo "[-] Failed: Missing cooling section"; exit 1; }
echo "[+] Status snapshot contains all telemetry sections."

echo "[2/4] Testing awccctl status --json..."
STATUS_JSON="$(${AWCCCTL} status --json)"
echo "${STATUS_JSON}" | grep -q '"dmi_product"' || { echo "[-] Failed: JSON missing dmi_product"; exit 1; }
echo "${STATUS_JSON}" | grep -q '"cpu"' || { echo "[-] Failed: JSON missing cpu"; exit 1; }
echo "${STATUS_JSON}" | grep -q '"gpu"' || { echo "[-] Failed: JSON missing gpu"; exit 1; }
echo "${STATUS_JSON}" | grep -q '"memory"' || { echo "[-] Failed: JSON missing memory"; exit 1; }
echo "${STATUS_JSON}" | grep -q '"power"' || { echo "[-] Failed: JSON missing power"; exit 1; }
echo "[+] Status JSON structure verified."

echo "[3/4] Testing awccctl track --once..."
TRACK_OUT="$(${AWCCCTL} track --once)"
echo "${TRACK_OUT}" | grep -q "AWCC Linux:" || { echo "[-] Failed: track --once missing header"; exit 1; }
echo "[+] Track single-shot verified."

echo "[4/4] Testing awccctl track --once --json..."
TRACK_JSON="$(${AWCCCTL} track --once --json)"
echo "${TRACK_JSON}" | grep -q '"thermals"' || { echo "[-] Failed: track JSON missing thermals"; exit 1; }
echo "[+] Track JSON verified."

echo "=== All AWCCCTL Tracking CLI Tests Passed Successfully! ==="
