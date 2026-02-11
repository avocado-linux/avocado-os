#!/usr/bin/env bash
#
# Cross-compile Linux kernel 6.12.69 for qemux86-64 using the Avocado SDK.
#
# The SDK entrypoint sources the OE environment, providing:
#   TARGET_PREFIX, ARCH, OECORE_NATIVE_SYSROOT, etc.
#
# Output: linux-6.12.69/arch/x86/boot/bzImage
#
set -e

KERNEL_SRC="linux-6.12.69"

echo "================================================================"
echo "Compiling Linux kernel 6.12.69 for qemux86-64"
echo "================================================================"

if [ ! -d "${KERNEL_SRC}" ]; then
  echo "[ERROR] Kernel source directory '${KERNEL_SRC}' not found."
  exit 1
fi

cd "${KERNEL_SRC}"

# ---------------------------------------------------------------------------
# Set up cross-compile variables from the SDK environment
# ---------------------------------------------------------------------------
# The SDK sources the OE environment before this script runs, so
# CROSS_COMPILE, ARCH, and OECORE_NATIVE_SYSROOT are already set.
# However the SDK also sets CC, CFLAGS, LDFLAGS, etc. with --sysroot
# and tuning flags that conflict with the kernel's internal build system
# (which derives CC from CROSS_COMPILE on its own). Save what we need,
# then unset the rest.
_CROSS_COMPILE="${CROSS_COMPILE}"
_ARCH="${ARCH}"
_TARGET_SYSROOT="${OECORE_TARGET_SYSROOT}"

unset CC CXX CPP LD AR AS NM STRIP OBJCOPY OBJDUMP READELF RANLIB
unset CFLAGS CXXFLAGS CPPFLAGS LDFLAGS
unset KCFLAGS

export CROSS_COMPILE="${_CROSS_COMPILE}"
export ARCH="${_ARCH}"

# The kernel needs a native (host) compiler for build-time tools like
# fixdep, genksyms, etc.  The kernel Makefile hardcodes "HOSTCC = gcc",
# and GNU make ignores environment variables for unconditional assignments,
# so we MUST pass HOSTCC/HOSTCXX on the make command line.
#
# Since this is x86_64 → x86_64 the cross-compiler produces host-runnable
# binaries and can serve as HOSTCC.  For true cross-arch builds you would
# need nativesdk-gcc in the SDK instead.
#
# The bare cross-compiler has no default sysroot so we must supply one.
# The native sysroot is a minimal toolchain tree without libc headers,
# but the target sysroot has full development headers.  Since host and
# target are both x86_64 the target headers are binary-compatible.
HOSTCC="${CROSS_COMPILE}gcc --sysroot=${_TARGET_SYSROOT}"
HOSTCXX="${CROSS_COMPILE}g++ --sysroot=${_TARGET_SYSROOT}"
HOSTLD="${CROSS_COMPILE}ld"
HOSTAR="${CROSS_COMPILE}ar"

# Common make arguments used for every invocation.
# - HOSTCC / HOSTCXX: top-level Makefile uses "=" so env vars are ignored;
#   must be overridden on the command line.
# - HOSTLD / HOSTAR: tools/scripts/Makefile.include uses "?=" so env vars
#   would work, but passing on the command line is more robust.  objtool's
#   build passes these through to libsubcmd as LD= and AR=.
MAKE_ARGS=(
  HOSTCC="${HOSTCC}"
  HOSTCXX="${HOSTCXX}"
  HOSTLD="${HOSTLD}"
  HOSTAR="${HOSTAR}"
)

echo "ARCH=${ARCH}"
echo "CROSS_COMPILE=${CROSS_COMPILE}"
echo "HOSTCC=${HOSTCC}"

# ---------------------------------------------------------------------------
# Configure the kernel
# ---------------------------------------------------------------------------
echo "Configuring kernel with x86_64_defconfig..."
make "${MAKE_ARGS[@]}" x86_64_defconfig

# =====================================================================
# Avocado core config (from avocado-core.cfg / defconfig)
# =====================================================================

# --- Filesystem support ---
scripts/config --enable CONFIG_OVERLAY_FS
scripts/config --enable CONFIG_EROFS_FS
scripts/config --enable CONFIG_EXT4_FS
scripts/config --enable CONFIG_BTRFS_FS
scripts/config --enable CONFIG_PARTITION_ADVANCED
scripts/config --enable CONFIG_EFI_PARTITION
scripts/config --enable CONFIG_BLK_DEV_LOOP

# SquashFS (systemd-sysext uses squashfs images)
scripts/config --enable CONFIG_SQUASHFS
scripts/config --enable CONFIG_SQUASHFS_FILE_CACHE
scripts/config --enable CONFIG_SQUASHFS_DECOMP_MULTI
scripts/config --enable CONFIG_SQUASHFS_ZLIB
scripts/config --enable CONFIG_SQUASHFS_ZSTD
scripts/config --set-val CONFIG_SQUASHFS_FRAGMENT_CACHE_SIZE 3
scripts/config --enable CONFIG_CRYPTO_LZ4
scripts/config --enable CONFIG_ZRAM

# --- MMC / SDHCI (root device is /dev/mmcblk0p5) ---
scripts/config --enable CONFIG_MMC
scripts/config --enable CONFIG_MMC_SDHCI
scripts/config --enable CONFIG_MMC_SDHCI_PCI

# --- Storage ---
scripts/config --enable CONFIG_BLK_DEV_SD
scripts/config --enable CONFIG_SCSI
scripts/config --enable CONFIG_ATA
scripts/config --enable CONFIG_SATA_AHCI
scripts/config --enable CONFIG_SCSI_VIRTIO

# --- Virtio drivers (QEMU) ---
scripts/config --enable CONFIG_VIRTIO
scripts/config --enable CONFIG_VIRTIO_PCI
scripts/config --enable CONFIG_VIRTIO_BLK
scripts/config --enable CONFIG_VIRTIO_MMIO
scripts/config --enable CONFIG_VIRTIO_NET
scripts/config --enable CONFIG_VIRTIO_CONSOLE
scripts/config --enable CONFIG_VIRTIO_BALLOON
scripts/config --enable CONFIG_HW_RANDOM_VIRTIO
scripts/config --enable CONFIG_DRM_VIRTIO_GPU
scripts/config --enable CONFIG_VIRTIO_INPUT
scripts/config --enable CONFIG_CRYPTO_DEV_VIRTIO

# --- 9P filesystem (host/guest file sharing) ---
scripts/config --enable CONFIG_NET_9P
scripts/config --enable CONFIG_NET_9P_VIRTIO
scripts/config --enable CONFIG_9P_FS

# --- Boot / EFI ---
scripts/config --enable CONFIG_EFI
scripts/config --enable CONFIG_EFI_STUB
scripts/config --enable CONFIG_BLK_DEV_INITRD
scripts/config --enable CONFIG_RD_ZSTD

# --- Systemd requirements ---
scripts/config --enable CONFIG_CGROUPS
scripts/config --enable CONFIG_CGROUP_FREEZER
scripts/config --enable CONFIG_CGROUP_DEVICE
scripts/config --enable CONFIG_CGROUP_SCHED
scripts/config --enable CONFIG_CGROUP_CPUACCT
scripts/config --enable CONFIG_CGROUP_PIDS
scripts/config --enable CONFIG_CGROUP_PERF
scripts/config --enable CONFIG_BLK_CGROUP
scripts/config --enable CONFIG_USER_NS
scripts/config --enable CONFIG_NAMESPACES
scripts/config --enable CONFIG_INOTIFY_USER
scripts/config --enable CONFIG_SIGNALFD
scripts/config --enable CONFIG_TIMERFD
scripts/config --enable CONFIG_EPOLL
scripts/config --enable CONFIG_TMPFS
scripts/config --enable CONFIG_TMPFS_POSIX_ACL
scripts/config --enable CONFIG_DEVTMPFS
scripts/config --enable CONFIG_DEVTMPFS_MOUNT
scripts/config --enable CONFIG_FHANDLE
scripts/config --enable CONFIG_AUTOFS_FS
scripts/config --enable CONFIG_BINFMT_MISC

# --- TPM (qemux86-64) ---
scripts/config --enable CONFIG_TCG_TPM
scripts/config --enable CONFIG_TCG_TIS
scripts/config --enable CONFIG_TCG_TIS_CORE
scripts/config --enable CONFIG_SECURITYFS
scripts/config --enable CONFIG_HW_RANDOM_TPM

# --- Serial / Console ---
scripts/config --enable CONFIG_SERIAL_8250
scripts/config --enable CONFIG_SERIAL_8250_CONSOLE

# --- Input ---
scripts/config --enable CONFIG_INPUT
scripts/config --enable CONFIG_INPUT_EVDEV

# --- USB ---
scripts/config --enable CONFIG_USB
scripts/config --enable CONFIG_USB_HID
scripts/config --enable CONFIG_USB_XHCI_HCD
scripts/config --enable CONFIG_USB_EHCI_HCD
scripts/config --enable CONFIG_USB_STORAGE

# --- RTC ---
scripts/config --enable CONFIG_RTC_CLASS
scripts/config --enable CONFIG_RTC_HCTOSYS

# --- Security ---
scripts/config --enable CONFIG_SECURITY

# Ensure the config is consistent
make "${MAKE_ARGS[@]}" olddefconfig

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------
echo "Building kernel..."
make "${MAKE_ARGS[@]}" -j"$(nproc)" bzImage

echo ""
echo "================================================================"
echo "Kernel build complete: arch/x86/boot/bzImage"
echo "================================================================"
