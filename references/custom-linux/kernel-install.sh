#!/usr/bin/env bash
#
# Install the cross-compiled kernel into the runtime build directory.
#
# AVOCADO_RUNTIME_BUILD_DIR: set by avocado-cli, points to
#   $AVOCADO_PREFIX/runtimes/<runtime_name>/
#
# The avocado-build hook expects the kernel image (bzImage) to be available
# in the runtime build directory.
#
set -e

KERNEL_SRC="linux-6.12.69"
BZIMAGE="${KERNEL_SRC}/arch/x86/boot/bzImage"

echo "================================================================"
echo "Installing kernel into runtime build directory"
echo "================================================================"

if [ -z "${AVOCADO_RUNTIME_BUILD_DIR}" ]; then
  echo "[ERROR] AVOCADO_RUNTIME_BUILD_DIR is not set."
  exit 1
fi

if [ ! -f "${BZIMAGE}" ]; then
  echo "[ERROR] Kernel image not found at ${BZIMAGE}"
  echo "  Run kernel-compile.sh first."
  exit 1
fi

mkdir -p "${AVOCADO_RUNTIME_BUILD_DIR}"

echo "Copying bzImage to ${AVOCADO_RUNTIME_BUILD_DIR}/"
cp -f "${BZIMAGE}" "${AVOCADO_RUNTIME_BUILD_DIR}/bzImage"

echo ""
echo "Kernel version:"
file "${BZIMAGE}"

echo ""
echo "================================================================"
echo "Kernel installed successfully"
echo "================================================================"
