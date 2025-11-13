#!/bin/sh

set -e

# Directories
RFDM_VAR_DIR="/var/lib/rfdm/bin"
RFDM_CONFIG_DIR="/var/lib/rfdm/config"
RFDM_BINARY="${RFDM_VAR_DIR}/rfdm"
RFDM_SOURCE="/opt/roboflow/rfdm"
RFDM_TEMP="${RFDM_VAR_DIR}/rfdm.new"
RFCONFIG_VAR="${RFDM_CONFIG_DIR}/rfconfig.json"
RFCONFIG_SOURCE="/opt/roboflow/config/rfconfig.json"

# Create directories if they don't exist
mkdir -p "${RFDM_VAR_DIR}"
mkdir -p "${RFDM_CONFIG_DIR}"

# If rfdm doesn't exist in /var/lib/rfdm/bin, copy it from /opt/roboflow
if [ ! -f "${RFDM_BINARY}" ]; then
    echo "rfdm not found at ${RFDM_BINARY}, copying from ${RFDM_SOURCE}"
    cp "${RFDM_SOURCE}" "${RFDM_BINARY}"
    chmod +x "${RFDM_BINARY}"
fi

# If rfconfig.json doesn't exist in /var/lib/rfdm/config, copy it from /opt/roboflow
if [ ! -f "${RFCONFIG_VAR}" ] && [ -f "${RFCONFIG_SOURCE}" ]; then
    echo "rfconfig.json not found at ${RFCONFIG_VAR}, copying from ${RFCONFIG_SOURCE}"
    cp "${RFCONFIG_SOURCE}" "${RFCONFIG_VAR}"
fi

# Check for updated version
echo "Checking for rfdm updates..."

# Get the latest version
VERSION=$(curl -fsSL https://repo.roboflow.com/rfdm/latest/LATEST_VERSION.txt 2>/dev/null || echo "")

if [ -z "$VERSION" ]; then
    echo "Warning: Failed to retrieve latest version, using existing binary"
    exit 0
fi

echo "Latest version available: $VERSION"

# Download the new binary to a temporary location
if curl -fsSL -o "${RFDM_TEMP}" "https://repo.roboflow.com/rfdm/${VERSION}/rfdm/linux-arm64" 2>/dev/null; then
    chmod +x "${RFDM_TEMP}"
    
    # Compare the binaries to see if they're different
    if ! cmp -s "${RFDM_BINARY}" "${RFDM_TEMP}"; then
        echo "New version detected, updating rfdm binary"
        mv "${RFDM_TEMP}" "${RFDM_BINARY}"
    else
        echo "rfdm binary is already up to date"
        rm -f "${RFDM_TEMP}"
    fi
else
    echo "Warning: Failed to download updated version, using existing binary"
    rm -f "${RFDM_TEMP}"
fi

echo "rfdm initialization complete"

