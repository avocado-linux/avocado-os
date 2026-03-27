#!/usr/bin/env bash

set -e

echo "============================================"
echo "Building gpio-toggle with meson"
echo "============================================"

# ---------------------------------------------------------------------------
# SDK meson cross-compilation setup
# ---------------------------------------------------------------------------
# The SDK's meson-wrapper auto-injects --cross-file and --native-file on every
# meson setup call. These files may not exist in the container SDK, so we
# generate them from the SDK environment.
MESON_CROSS_FILE="${OECORE_NATIVE_SYSROOT}/usr/share/meson/${TARGET_PREFIX}meson.cross"
MESON_NATIVE_FILE="${OECORE_NATIVE_SYSROOT}/usr/share/meson/meson.native"

if [ ! -f "$MESON_CROSS_FILE" ]; then
    echo "Generating meson cross file..."
    cat > "$MESON_CROSS_FILE" <<MESONEOF
[binaries]
c = '${CROSS_COMPILE}gcc'
cpp = '${CROSS_COMPILE}g++'
ar = '${CROSS_COMPILE}ar'
nm = '${CROSS_COMPILE}nm'
strip = '${CROSS_COMPILE}strip'
pkg-config = 'pkg-config'

[built-in options]
c_args = ['--sysroot=${OECORE_TARGET_SYSROOT}']
c_link_args = ['--sysroot=${OECORE_TARGET_SYSROOT}']
cpp_args = ['--sysroot=${OECORE_TARGET_SYSROOT}']
cpp_link_args = ['--sysroot=${OECORE_TARGET_SYSROOT}']

[properties]
needs_exe_wrapper = true
sys_root = '${OECORE_TARGET_SYSROOT}'

[host_machine]
system = 'linux'
cpu_family = 'aarch64'
cpu = 'cortex-a76'
endian = 'little'
MESONEOF
fi

if [ ! -f "$MESON_NATIVE_FILE" ]; then
    SDK_HOST_PREFIX="${AVOCADO_SDK_ARCH:-x86_64}-avocadosdk-linux"
    NATIVE_CC="${OECORE_NATIVE_SYSROOT}/usr/bin/${SDK_HOST_PREFIX}-gcc"
    NATIVE_CXX="${OECORE_NATIVE_SYSROOT}/usr/bin/${SDK_HOST_PREFIX}-g++"

    echo "Generating meson native file..."
    cat > "$MESON_NATIVE_FILE" <<NATIVEEOF
[binaries]
c = '${NATIVE_CC}'
cpp = '${NATIVE_CXX}'
ar = '${OECORE_NATIVE_SYSROOT}/usr/bin/${SDK_HOST_PREFIX}-ar'
nm = '${OECORE_NATIVE_SYSROOT}/usr/bin/${SDK_HOST_PREFIX}-nm'
strip = '${OECORE_NATIVE_SYSROOT}/usr/bin/${SDK_HOST_PREFIX}-strip'
pkg-config = 'pkg-config-native'

[properties]
sys_root = '${OECORE_NATIVE_SYSROOT}'
NATIVEEOF
fi

# ---------------------------------------------------------------------------
# Build the application
# ---------------------------------------------------------------------------
cd app/src

meson setup build --prefix=/usr
ninja -C build

echo ""
echo "Build complete: app/src/build/gpio-toggle"
file build/gpio-toggle
