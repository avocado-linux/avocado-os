#!/usr/bin/env bash
#
# Clean the Linux kernel out-of-tree build directory.
#
# Removes $AVOCADO_BUILD_DIR, which holds all kernel build artifacts from
# the out-of-tree build (O=$AVOCADO_BUILD_DIR). The kernel source tree
# itself is left untouched.
#
set -e

if [ -z "${AVOCADO_BUILD_DIR}" ]; then
  echo "[ERROR] AVOCADO_BUILD_DIR is not set." >&2
  exit 1
fi

if [ ! -d "${AVOCADO_BUILD_DIR}" ]; then
  echo "Build directory ${AVOCADO_BUILD_DIR} does not exist, nothing to clean."
  exit 0
fi

echo "================================================================"
echo "Cleaning kernel build directory: ${AVOCADO_BUILD_DIR}"
echo "================================================================"

rm -rf "${AVOCADO_BUILD_DIR}"

echo "Kernel build directory cleaned."
