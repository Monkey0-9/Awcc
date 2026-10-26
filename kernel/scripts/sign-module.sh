#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KERNEL_DIR="$(dirname "$SCRIPT_DIR")"
KVER="$(uname -r)"

echo "=== AWCC Linux Module Signer ==="
echo "Target Kernel: ${KVER}"

KEY_DIR="${KERNEL_DIR}/certs"
mkdir -p "${KEY_DIR}"
chmod 700 "${KEY_DIR}"

PRIV_KEY="${KEY_DIR}/mok.priv"
PUB_KEY="${KEY_DIR}/mok.der"
CONF_FILE="${KEY_DIR}/x509.conf"

if [[ ! -f "${PRIV_KEY}" || ! -f "${PUB_KEY}" ]]; then
    echo "[*] Generating self-signed MOK key pair..."
    cat > "${CONF_FILE}" <<EOF
[ req ]
default_bits = 4096
distinguished_name = req_distinguished_name
prompt = no
x509_extensions = myexts

[ req_distinguished_name ]
CN = AWCC Linux Driver Module Key

[ myexts ]
basicConstraints=critical,CA:FALSE
keyUsage=digitalSignature
subjectKeyIdentifier=hash
authorityKeyIdentifier=keyid
EOF

    openssl req -x509 -new -nodes -utf8 -sha256 -days 36500 -batch \
        -config "${CONF_FILE}" -outform DER -out "${PUB_KEY}" \
        -keyout "${PRIV_KEY}"
    chmod 600 "${PRIV_KEY}"
    echo "[+] Generated ${PUB_KEY} and ${PRIV_KEY}"
fi

SIGN_FILE="/usr/src/kernels/${KVER}/scripts/sign-file"
if [[ ! -x "${SIGN_FILE}" ]]; then
    echo "[-] Error: ${SIGN_FILE} not found or not executable." >&2
    exit 1
fi

MODULE="${KERNEL_DIR}/alienware-wmi.ko"
if [[ ! -f "${MODULE}" ]]; then
    echo "[-] Error: ${MODULE} not found. Build it first with 'make'." >&2
    exit 1
fi

echo "[*] Signing ${MODULE} with sha256..."
"${SIGN_FILE}" sha256 "${PRIV_KEY}" "${PUB_KEY}" "${MODULE}"

echo "[+] Module signature complete:"
modinfo "${MODULE}" | grep -E "signer|sig_key|sig_hashalgo|signature" || true

echo "=== Success ==="
