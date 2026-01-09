#!/usr/bin/env bash
set -e

echo "============================================"
echo "Building v4l2loopback kernel module"
echo "============================================"

# Clone v4l2loopback if not already present
if [ ! -d "v4l2loopback" ]; then
    echo "Cloning v4l2loopback repository..."
    git clone https://github.com/v4l2loopback/v4l2loopback.git
    cd v4l2loopback
    # Use a stable release version
    git checkout v0.13.2
else
    echo "Using existing v4l2loopback directory"
    cd v4l2loopback
fi

# Find the kernel version
KERNEL_VERSION=""

if [ -f "${OECORE_TARGET_SYSROOT}/usr/src/kernel/include/config/kernel.release" ]; then
    KERNEL_VERSION=$(cat "${OECORE_TARGET_SYSROOT}/usr/src/kernel/include/config/kernel.release")
fi

if [ -z "$KERNEL_VERSION" ]; then
    KERNEL_VERSION=$(ls "${OECORE_TARGET_SYSROOT}/lib/modules/" 2>/dev/null | head -n1 || echo "")
fi

if [ -z "$KERNEL_VERSION" ]; then
    echo "ERROR: Could not determine kernel version"
    exit 1
fi

echo "Kernel version: $KERNEL_VERSION"

# The kernel build directory
KDIR="${OECORE_TARGET_SYSROOT}/usr/lib/modules/${KERNEL_VERSION}/build"

if [ ! -d "$KDIR" ]; then
    KDIR="${OECORE_TARGET_SYSROOT}/usr/src/kernel"
fi

echo "Kernel build dir: $KDIR"

if [ ! -f "$KDIR/Makefile" ]; then
    echo "ERROR: No Makefile found in $KDIR"
    echo "Make sure kernel-devsrc package is installed"
    exit 1
fi

echo "Cross compiler prefix: $CROSS_COMPILE"
echo "Architecture: $ARCH"
echo "Working directory: $(pwd)"

# The kernel-devsrc package includes script sources but not compiled host binaries.
# We need to build them for the SDK host using 'modules_prepare'.
# These are HOST binaries (run on SDK host), so we use HOSTCC (not CROSS_COMPILE).
# Check if modules_prepare has been run (modpost is the key tool for modules)
if [ ! -x "$KDIR/scripts/mod/modpost" ]; then
    echo "Preparing kernel for module compilation (modules_prepare)..."
    
    # Find the host compiler - nativesdk-gcc may be prefixed (${SDK_ARCH}-avocadosdk-linux-gcc)
    HOST_GCC=""
    HOST_GXX=""
    SDK_HOST_PREFIX="${AVOCADO_SDK_ARCH:-x86_64}-avocadosdk-linux"
    
    if [ -n "$OECORE_NATIVE_SYSROOT" ]; then
        # Try prefixed version first (typical for nativesdk)
        if [ -x "$OECORE_NATIVE_SYSROOT/usr/bin/${SDK_HOST_PREFIX}-gcc" ]; then
            HOST_GCC="$OECORE_NATIVE_SYSROOT/usr/bin/${SDK_HOST_PREFIX}-gcc"
            HOST_GXX="$OECORE_NATIVE_SYSROOT/usr/bin/${SDK_HOST_PREFIX}-g++"
        elif [ -x "$OECORE_NATIVE_SYSROOT/usr/bin/gcc" ]; then
            HOST_GCC="$OECORE_NATIVE_SYSROOT/usr/bin/gcc"
            HOST_GXX="$OECORE_NATIVE_SYSROOT/usr/bin/g++"
        fi
    fi
    
    # Fallback to finding gcc in PATH
    if [ -z "$HOST_GCC" ]; then
        if command -v ${SDK_HOST_PREFIX}-gcc &>/dev/null; then
            HOST_GCC="${SDK_HOST_PREFIX}-gcc"
            HOST_GXX="${SDK_HOST_PREFIX}-g++"
        elif command -v gcc &>/dev/null; then
            HOST_GCC="gcc"
            HOST_GXX="g++"
        else
            echo "ERROR: No host gcc found. Need nativesdk-gcc or system gcc."
            echo "OECORE_NATIVE_SYSROOT=$OECORE_NATIVE_SYSROOT"
            echo "SDK_HOST_PREFIX=$SDK_HOST_PREFIX"
            exit 1
        fi
    fi
    
    echo "Using host compiler: $HOST_GCC"
    
    # Use modules_prepare instead of just scripts - it builds everything needed
    # for out-of-tree module compilation including modpost, recordmcount, etc.
    make -C ${KDIR} \
        ARCH=${ARCH} \
        CROSS_COMPILE=${CROSS_COMPILE} \
        HOSTCC="${HOSTCC:-$HOST_GCC}" \
        HOSTCXX="${HOSTCXX:-$HOST_GXX}" \
        modules_prepare
fi

# Build the kernel module
echo "Building v4l2loopback module..."
make -C ${KDIR} \
    M=$(pwd) \
    ARCH=${ARCH} \
    CROSS_COMPILE=${CROSS_COMPILE} \
    modules

echo "Build complete!"
echo "Module location: $(pwd)/v4l2loopback.ko"
ls -lh v4l2loopback.ko
