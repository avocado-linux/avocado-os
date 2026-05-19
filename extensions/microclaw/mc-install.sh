#!/bin/bash
set -euo pipefail

BINARY_PATH="${AVOCADO_BUILD_DIR}/microclaw/microclaw"

if [ ! -f "${BINARY_PATH}" ]; then
    echo "Error: Binary not found at ${BINARY_PATH}" >&2
    exit 1
fi

install -D -m 0755 "${BINARY_PATH}" "${AVOCADO_BUILD_EXT_SYSROOT}/usr/bin/microclaw"
echo "microclaw installed to ${AVOCADO_BUILD_EXT_SYSROOT}/usr/bin/microclaw"
