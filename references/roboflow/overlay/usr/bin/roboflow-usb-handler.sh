#!/bin/sh
#
# Roboflow USB Device Handler
# This script is triggered when a USB mass storage device is inserted.
# It mounts the device, looks for device.json, copies it if found, and unmounts.
#

set -e

DEVICE="$1"
MOUNT_POINT="/tmp/roboflow-usb-$$"
TARGET_DIR="/var/lib/rfdm/config"
TARGET_FILE="${TARGET_DIR}/device.json"
SOURCE_FILE="device.json"

# Logging function
log() {
    echo "[roboflow-usb-handler] $*" | systemd-cat -t roboflow-usb-handler -p info
}

error() {
    echo "[roboflow-usb-handler] ERROR: $*" | systemd-cat -t roboflow-usb-handler -p err
}

# Cleanup function
cleanup() {
    if mountpoint -q "${MOUNT_POINT}" 2>/dev/null; then
        log "Unmounting ${DEVICE}"
        umount "${MOUNT_POINT}" || error "Failed to unmount ${DEVICE}"
    fi
    if [ -d "${MOUNT_POINT}" ]; then
        rmdir "${MOUNT_POINT}" || error "Failed to remove mount point ${MOUNT_POINT}"
    fi
}

# Set trap to ensure cleanup happens
trap cleanup EXIT

# Validate device exists
if [ ! -b "${DEVICE}" ]; then
    error "Device ${DEVICE} does not exist or is not a block device"
    exit 1
fi

log "Processing USB device: ${DEVICE}"

# Wait a moment for the device to be ready
sleep 1

# Create mount point
mkdir -p "${MOUNT_POINT}"

# Try to mount the device (try common filesystems)
if ! mount -o ro "${DEVICE}" "${MOUNT_POINT}" 2>/dev/null; then
    error "Failed to mount ${DEVICE}"
    exit 1
fi

log "Mounted ${DEVICE} at ${MOUNT_POINT}"

# Check if device.json exists on the USB device
if [ -f "${MOUNT_POINT}/${SOURCE_FILE}" ]; then
    log "Found ${SOURCE_FILE} on ${DEVICE}"
    
    # Create target directory if it doesn't exist
    mkdir -p "${TARGET_DIR}"
    
    # Copy the file
    if cp "${MOUNT_POINT}/${SOURCE_FILE}" "${TARGET_FILE}"; then
        log "Successfully copied ${SOURCE_FILE} to ${TARGET_FILE}"
        
        # Set appropriate permissions
        chmod 644 "${TARGET_FILE}"
        
        log "Configuration updated from USB device"
    else
        error "Failed to copy ${SOURCE_FILE} to ${TARGET_FILE}"
        exit 1
    fi
else
    log "No ${SOURCE_FILE} found on ${DEVICE}, skipping"
fi

# Cleanup will be handled by trap
log "Finished processing ${DEVICE}"
exit 0

