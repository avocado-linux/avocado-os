#!/usr/bin/env bash
set -e

echo "============================================"
echo "Installing v4l2loopback kernel module"
echo "============================================"

# Get kernel version from the kernel source
KDIR="${OECORE_TARGET_SYSROOT}/usr/src/kernel"

if [ -f "$KDIR/include/config/kernel.release" ]; then
    KERNEL_VERSION=$(cat "$KDIR/include/config/kernel.release")
elif [ -f "$KDIR/include/generated/utsrelease.h" ]; then
    KERNEL_VERSION=$(grep UTS_RELEASE "$KDIR/include/generated/utsrelease.h" | cut -d'"' -f2)
else
    echo "ERROR: Could not determine kernel version from $KDIR"
    exit 1
fi

echo "Kernel version: $KERNEL_VERSION"

# Install the kernel module into the extension sysroot
# AVOCADO_BUILD_EXT_SYSROOT is the sysroot of the extension being installed into
MODULE_DIR="${AVOCADO_BUILD_EXT_SYSROOT}/usr/lib/modules/${KERNEL_VERSION}/extra"
echo "Installing module to: $MODULE_DIR"

install -d "$MODULE_DIR"
install -m 0644 v4l2loopback/v4l2loopback.ko "$MODULE_DIR/"

echo "Module installed successfully!"
echo "Installed to: $MODULE_DIR/v4l2loopback.ko"
