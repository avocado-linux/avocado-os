#!/usr/bin/env bash
#
# Stage the cross-compiled kernel into $DESTDIR for RPM packaging.
#
# Called by `avocado sdk package kernel`. Environment variables:
#   DESTDIR            - staging root; set by avocado-cli
#   AVOCADO_BUILD_DIR  - out-of-tree build directory; contains arch/x86/boot/bzImage
#
set -e

KERNEL_VERSION="6.12.69"

if [ -z "${AVOCADO_BUILD_DIR}" ]; then
  echo "[ERROR] AVOCADO_BUILD_DIR is not set." >&2
  exit 1
fi
BZIMAGE="${AVOCADO_BUILD_DIR}/arch/x86/boot/bzImage"

echo "================================================================"
echo "Staging kernel ${KERNEL_VERSION} for RPM packaging"
echo "================================================================"

if [ -z "${DESTDIR}" ]; then
  echo "[ERROR] DESTDIR is not set." >&2
  exit 1
fi

if [ ! -f "${BZIMAGE}" ]; then
  echo "[ERROR] Kernel image not found at ${BZIMAGE}" >&2
  echo "  Run 'avocado sdk compile kernel' first." >&2
  exit 1
fi

mkdir -p "${DESTDIR}/boot"

echo "Copying bzImage -> ${DESTDIR}/boot/vmlinuz-${KERNEL_VERSION}"
cp -f "${BZIMAGE}" "${DESTDIR}/boot/vmlinuz-${KERNEL_VERSION}"

echo ""
echo "================================================================"
echo "Kernel staged successfully"
echo "================================================================"
