#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
TARGET="${SCRIPT_DIR}/awcc-mode"
if [[ ! -x "${TARGET}" ]]; then
    TARGET="awcc-mode"
fi

exec "${TARGET}" "$@"
