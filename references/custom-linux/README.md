# Custom Kernel Compilation with Avocado

This guide covers how to compile and include your own Linux kernel in an Avocado
runtime, replacing the default kernel provided by the `avocado-runtime`
meta-package.

## Overview

By default, Avocado runtimes get their kernel from the `avocado-img-bootfiles`
package (pulled in by the `avocado-runtime` meta-package). The `kernel`
configuration option lets you override this with either:

- **A different package** -- install a specific kernel RPM from the package feed.
- **A from-source build** -- cross-compile the kernel inside the SDK container
  and install the resulting image into the runtime.

These two modes are mutually exclusive. If the `kernel` section is omitted
entirely, the default behavior is preserved.

## Configuration

Three sections of `avocado.yaml` are involved when compiling a kernel from
source.

### Runtime `kernel` section

Add a `kernel` block to any runtime that should use the custom kernel:

```yaml
runtimes:
  dev:
    # ... extensions, packages, etc.

    kernel:
      compile: kernel             # Name of the sdk.compile section
      install: kernel-install.sh  # Script that copies artifacts into the runtime
```

- `compile` -- References an `sdk.compile` section by name. That section's
  script does the actual kernel build.
- `install` -- A script executed after compilation. It receives
  `AVOCADO_RUNTIME_BUILD_DIR` (pointing to
  `$AVOCADO_PREFIX/runtimes/<runtime>/`) and is responsible for copying the
  kernel image there.

> **Package mode.** If you have a pre-built kernel RPM in your package feed,
> use `package` instead:
>
> ```yaml
> kernel:
>   package: kernel-image
>   version: '6.12.69'
> ```
>
> `package` and `compile` are mutually exclusive.

### SDK `compile` section

Define the compile section that the runtime references:

```yaml
sdk:
  image: docker.io/avocadolinux/sdk:{{ avocado.distro.channel }}

  compile:
    kernel:
      compile: kernel-compile.sh
      packages:           # Host packages installed before the script runs
        nativesdk-bc: '*'
        nativesdk-libelf-dev: '*'

  packages:
    avocado-sdk-toolchain: '{{ avocado.distro.version }}'
```

The `packages` field inside `sdk.compile.kernel` declares host-side
dependencies that are installed into the SDK sysroot before the compile script
runs. Common ones for kernel builds include `bc` (for `timeconst.h` generation)
and `libelf` (for `objtool`). Check your SDK package feed for the correct
names.

## Writing the Compile Script

The compile script runs inside the SDK container where the OpenEmbedded
cross-compilation environment is already sourced. Variables like
`CROSS_COMPILE`, `ARCH`, and `OECORE_TARGET_SYSROOT` are set before your script
executes.

### Dealing with SDK Environment Conflicts

The OE SDK sets `CC`, `CFLAGS`, `LDFLAGS`, and other variables with
`--sysroot` flags and tuning options designed for userspace cross-compilation.
The kernel build system manages its own flags and derives `CC` from
`CROSS_COMPILE` internally, so these SDK variables **conflict** with it.

Save the variables you need, unset the rest, then restore:

```bash
# Save what the kernel build needs
_CROSS_COMPILE="${CROSS_COMPILE}"
_ARCH="${ARCH}"
_TARGET_SYSROOT="${OECORE_TARGET_SYSROOT}"

# Clear everything the SDK sets that conflicts with the kernel build system
unset CC CXX CPP LD AR AS NM STRIP OBJCOPY OBJDUMP READELF RANLIB
unset CFLAGS CXXFLAGS CPPFLAGS LDFLAGS
unset KCFLAGS

# Restore
export CROSS_COMPILE="${_CROSS_COMPILE}"
export ARCH="${_ARCH}"
```

### Setting Up Host Tool Compilation

The kernel builds host-side tools (`fixdep`, `objtool`, `genksyms`, etc.) that
run on the build machine during compilation. Three issues arise in the SDK
environment:

1. **No bare `gcc`/`ld`/`ar`.** The SDK container only ships cross-prefixed
   tools (e.g., `x86_64-avocado-linux-gcc`).

2. **Make assignment semantics.** The kernel Makefile uses `HOSTCC = gcc`
   (unconditional assignment). In GNU make, environment variables do **not**
   override unconditional assignments -- you must pass them on the **command
   line**.

3. **No default sysroot.** The bare cross-compiler has no built-in sysroot, so
   standard headers like `sys/types.h` are not found. Point it at the target
   sysroot with `--sysroot`.

```bash
HOSTCC="${CROSS_COMPILE}gcc --sysroot=${_TARGET_SYSROOT}"
HOSTCXX="${CROSS_COMPILE}g++ --sysroot=${_TARGET_SYSROOT}"
HOSTLD="${CROSS_COMPILE}ld"
HOSTAR="${CROSS_COMPILE}ar"

MAKE_ARGS=(
  HOSTCC="${HOSTCC}"
  HOSTCXX="${HOSTCXX}"
  HOSTLD="${HOSTLD}"
  HOSTAR="${HOSTAR}"
)

make "${MAKE_ARGS[@]}" defconfig       # or your_defconfig
make "${MAKE_ARGS[@]}" olddefconfig
make "${MAKE_ARGS[@]}" -j"$(nproc)" bzImage  # or Image, zImage, etc.
```

> **Cross-architecture note.** Using the cross-compiler as `HOSTCC` only works
> when host and target share the same architecture (e.g., x86_64 SDK building
> an x86_64 kernel). For true cross-architecture builds (e.g., building an
> ARM64 kernel on x86_64), the cross-compiler's output won't run on the host.
> In that case, install `nativesdk-gcc` in the SDK to get a native host
> compiler.

### Required Kernel Configuration

Beyond the upstream defconfig, Avocado requires specific kernel options. The
distro layer ships config fragments you can reference:

```
distro/<meta-layer>/recipes-kernel/linux/files/
├── avocado-core.cfg      # Core requirements (filesystems, systemd, etc.)
├── avocado-extra.cfg     # Optional drivers
├── mmc.cfg               # MMC/SDHCI support (if root is on mmcblk)
├── tpm.cfg               # TPM support
└── <machine>/defconfig   # Machine-specific base config
```

At minimum, your kernel must have:

**Root device driver** -- Whatever storage controller presents the root
filesystem. Without this, the kernel will hang at `Waiting for root device...`.
Check what `/dev/` node the bootloader passes as `root=` and ensure the
corresponding driver is built in (not as a module).

**Filesystem support** -- Avocado uses overlayfs and squashfs for
system-extensions, btrfs for the var partition, and optionally erofs. These
should be built-in (`=y`), not modules.

**Systemd** -- The init system requires cgroups, namespaces, devtmpfs,
tmpfs with POSIX ACL, fhandle, autofs, inotify, signalfd, timerfd, and epoll.

Use `scripts/config` to enable options after running the base defconfig, then
run `make olddefconfig` to resolve dependencies before building.

## Writing the Install Script

The install script runs inside the SDK container after compilation. Its job is
to copy the kernel image to `$AVOCADO_RUNTIME_BUILD_DIR`:

```bash
#!/usr/bin/env bash
set -e

BZIMAGE="path/to/arch/<arch>/boot/<image>"

if [ -z "${AVOCADO_RUNTIME_BUILD_DIR}" ]; then
  echo "[ERROR] AVOCADO_RUNTIME_BUILD_DIR is not set." >&2
  exit 1
fi

mkdir -p "${AVOCADO_RUNTIME_BUILD_DIR}"
cp -f "${BZIMAGE}" "${AVOCADO_RUNTIME_BUILD_DIR}/bzImage"
```

`AVOCADO_RUNTIME_BUILD_DIR` is set automatically by `avocado-cli` and points to
`$AVOCADO_PREFIX/runtimes/<runtime_name>/`. This is the same location where the
package-based kernel would be placed, so the downstream build hook picks it up
without changes.

## Build Workflow

```bash
# Install the SDK and toolchain
avocado sdk install

# Compile the kernel (optional -- avocado build does this automatically)
avocado sdk compile kernel

# Build everything (triggers compile + install + runtime assembly)
avocado build
```

When `avocado build` encounters a runtime with a `kernel.compile` section, it:

1. Executes the referenced `sdk.compile` section (runs your compile script).
2. Runs the `kernel.install` script with `AVOCADO_RUNTIME_BUILD_DIR` set.
3. Proceeds with normal runtime assembly (extension installation, var part
   creation, image creation).

The build stamp system includes the `kernel` configuration in its hash, so
changes to the `kernel` section automatically trigger a rebuild.

## Adapting for Your Target

When targeting a different board or architecture, adjust:

| What | Why |
|---|---|
| `ARCH` and defconfig | Each architecture has its own defconfig and `ARCH` value (`arm64`, `arm`, `x86`, `riscv`, etc.). |
| Kernel image name | ARM64 produces `Image`, ARM produces `zImage`, x86 produces `bzImage`. Update both scripts. |
| Root device driver | Match the storage controller on your hardware (eMMC, NVMe, SATA, virtio-blk, etc.). |
| `HOSTCC` strategy | For same-arch builds, the cross-compiler works as HOSTCC. For cross-arch, install `nativesdk-gcc`. |
| Board-specific options | Device tree, pinmux, clock, regulator, and peripheral drivers for your hardware. |
| SDK packages | The package names for `bc`, `libelf`, etc. may vary across SDK package feeds. |

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `gcc: command not found` | SDK has no bare `gcc` | Set `HOSTCC` to the cross-prefixed compiler and pass it on the `make` command line |
| `ld: command not found` | SDK has no bare `ld` | Set `HOSTLD` to `${CROSS_COMPILE}ld` and pass on the `make` command line |
| `sys/types.h: No such file` | Cross-compiler has no sysroot | Add `--sysroot=${OECORE_TARGET_SYSROOT}` to `HOSTCC`/`HOSTCXX` |
| `bc: command not found` | Missing host utility | Add `nativesdk-bc` (or equivalent) to SDK packages |
| `libelf` errors from objtool | Missing development library | Add `nativesdk-libelf-dev` (or equivalent) to SDK packages |
| `Waiting for root device...` | Missing storage driver | Enable the driver for your root device (MMC, NVMe, virtio-blk, etc.) |
| Config option silently ignored | Dependency not met | Run `make olddefconfig` after `scripts/config` calls to resolve dependencies |
