#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
ROOT_DIR="$(dirname "${SCRIPT_DIR}")"
AWCCCTL="${ROOT_DIR}/target/release/awccctl"
AWCC_TRACK="${ROOT_DIR}/bin/awcc-track"

echo "=== Running AWCCCTL Extended Telemetry & Tracking CLI Tests ==="

if [[ ! -x "${AWCCCTL}" ]]; then
    echo "[-] Building release awccctl..."
    cargo build --release --bin awccctl
fi

echo "[1/7] Testing awccctl status output format..."
STATUS_OUT="$(${AWCCCTL} status)"
echo "${STATUS_OUT}" | grep -q "AWCC Linux:" || { echo "[-] Failed: Missing title in status output"; exit 1; }
echo "${STATUS_OUT}" | grep -q "SILICON COMPUTATION & PERFORMANCE" || { echo "[-] Failed: Missing silicon section"; exit 1; }
echo "${STATUS_OUT}" | grep -q "THERMAL SENSOR MATRIX" || { echo "[-] Failed: Missing thermal section"; exit 1; }
echo "${STATUS_OUT}" | grep -q "COOLING TURBINES" || { echo "[-] Failed: Missing cooling section"; exit 1; }
echo "[+] Status snapshot contains all telemetry sections."

echo "[2/7] Testing awccctl status --json..."
STATUS_JSON="$(${AWCCCTL} status --json)"
echo "${STATUS_JSON}" | grep -q '"dmi_product"' || { echo "[-] Failed: JSON missing dmi_product"; exit 1; }
echo "${STATUS_JSON}" | grep -q '"cpu"' || { echo "[-] Failed: JSON missing cpu"; exit 1; }
echo "${STATUS_JSON}" | grep -q '"gpu"' || { echo "[-] Failed: JSON missing gpu"; exit 1; }
echo "${STATUS_JSON}" | grep -q '"memory"' || { echo "[-] Failed: JSON missing memory"; exit 1; }
echo "${STATUS_JSON}" | grep -q '"power"' || { echo "[-] Failed: JSON missing power"; exit 1; }
echo "[+] Status JSON structure verified."

echo "[3/7] Testing awccctl track --once and --cli..."
TRACK_OUT="$(${AWCCCTL} track --once --cli)"
echo "${TRACK_OUT}" | grep -q "AWCC Linux:" || { echo "[-] Failed: track --once missing header"; exit 1; }
echo "[+] Track single-shot verified."

echo "[4/7] Testing awccctl track --snapshot alias..."
SNAP_OUT="$(${AWCCCTL} track --snapshot)"
echo "${SNAP_OUT}" | grep -q "SILICON COMPUTATION & PERFORMANCE" || { echo "[-] Failed: track --snapshot failed"; exit 1; }
echo "[+] Track snapshot alias verified."

echo "[5/7] Testing awccctl track --once --json..."
TRACK_JSON="$(${AWCCCTL} track --once --json)"
echo "${TRACK_JSON}" | grep -q '"thermals"' || { echo "[-] Failed: track JSON missing thermals"; exit 1; }
echo "[+] Track JSON verified."

echo "[6/7] Testing bin/awcc-track wrapper (--help, --cli --once, --json --once)..."
"${AWCC_TRACK}" --help | grep -q "awcc-track" || { echo "[-] Failed: awcc-track --help failed"; exit 1; }
WRAPPER_CLI_OUT="$("${AWCC_TRACK}" --cli --once)"
echo "${WRAPPER_CLI_OUT}" | grep -q "AWCC Linux:" || { echo "[-] Failed: awcc-track --cli --once failed"; exit 1; }
WRAPPER_JSON_OUT="$("${AWCC_TRACK}" --json --once)"
echo "${WRAPPER_JSON_OUT}" | grep -q '"dmi_product"' || { echo "[-] Failed: awcc-track --json --once failed"; exit 1; }
echo "[+] bin/awcc-track wrapper verified across CLI and JSON modes."

echo "[7/10] Testing live CPU load tracking..."
LOAD_VAL=$(echo "${STATUS_JSON}" | python3 -c "import sys, json; data=json.load(sys.stdin); print(data.get('cpu', {}).get('usage_percent'))")
if [[ "${LOAD_VAL}" != "None" ]] && [[ -n "${LOAD_VAL}" ]]; then
    python3 -c "assert 0.0 <= float('${LOAD_VAL}') <= 100.0, 'Invalid CPU load percentage'"
    echo "[+] Live CPU load tracking verified: ${LOAD_VAL}%"
else
    echo "[+] CPU load not exposed on this mock/virtual interface (graceful None)."
fi

echo "[8/10] Testing awccctl monitor alias and --once snapshot..."
MON_OUT="$(${AWCCCTL} monitor --once)"
echo "${MON_OUT}" | grep -q "AWCC Linux:" || { echo "[-] Failed: monitor --once missing header"; exit 1; }
echo "${MON_OUT}" | grep -q "THERMAL SENSOR MATRIX" || { echo "[-] Failed: monitor --once missing thermal matrix"; exit 1; }
echo "[+] awccctl monitor alias verified."

echo "[9/10] Testing bin/awcc-monitor dedicated script..."
AWCC_MON="${ROOT_DIR}/bin/awcc-monitor"
"${AWCC_MON}" --help | grep -q "awcc-monitor" || { echo "[-] Failed: awcc-monitor --help failed"; exit 1; }
MON_SCRIPT_OUT="$("${AWCC_MON}" --once)"
echo "${MON_SCRIPT_OUT}" | grep -q "AWCC Linux:" || { echo "[-] Failed: awcc-monitor --once failed"; exit 1; }
echo "[+] bin/awcc-monitor wrapper verified."

echo "[10/10] Testing continuous live monitoring and clean interrupt termination (SIGINT / Ctrl+C)..."
"${AWCCCTL}" monitor &
MON_PID=$!
sleep 1.2
kill -2 "${MON_PID}"
wait "${MON_PID}" || { echo "[-] Failed: awccctl monitor did not exit cleanly on SIGINT"; exit 1; }
echo "[+] Live monitoring terminated cleanly on SIGINT (Exit Code 0)."

echo "=== All AWCCCTL Tracking CLI Tests Passed Successfully! ==="
