# Provenance: recomputer-robotics-j5012

Every pinned input this extension is built from, each followed by the command
that produced the value. Re-run a command and compare its output with the value
recorded here; a different result means the input has drifted. All values were
resolved on 2026-09-29. Pinned sources are read through the GitHub API at a
fixed commit, never from a local checkout on another branch.

This file is a build record. It is not listed in `package_files` and does not
ship in the extension package.

## Seeed carrier sources

- Repository: <https://github.com/Seeed-Studio/Linux_for_Tegra>
- Branch: `r36.5.0`
- Commit: `f9a68317fbe3d276efc25b14b8b72339a1cc5d5c`

```sh
gh api repos/Seeed-Studio/Linux_for_Tegra/commits/r36.5.0 --jq .sha
```

The branch moves; the commit is the pin. Later tasks fetch every Seeed file at
`?ref=f9a68317fbe3d276efc25b14b8b72339a1cc5d5c`, never at `r36.5.0`.

## meta-avocado (2024 feed)

- Repository: <https://github.com/avocado-linux/meta-avocado>
- Branch: `scarthgap`
- Commit: `e5b750c61c09b16deea4cd21cb01586e1d39d874` (committer date
  2026-09-29T14:21:16Z; the `scarthgap` head when resolved)

```sh
gh api repos/avocado-linux/meta-avocado/commits/scarthgap --jq .sha
gh api repos/avocado-linux/meta-avocado/commits/e5b750c61c09b16deea4cd21cb01586e1d39d874 --jq '.sha + " " + .commit.committer.date'
```

Provisioning script whose carrier block the override check runs:
`meta-avocado-nvidia/stone/tegra/stone-provision-tegraflash.sh`, git blob
`821bc7ce648fa7b0aca66f8f674145b8d8c62c38`, 26430 bytes.

```sh
gh api "repos/avocado-linux/meta-avocado/contents/meta-avocado-nvidia/stone/tegra/stone-provision-tegraflash.sh?ref=e5b750c61c09b16deea4cd21cb01586e1d39d874" --jq '.path + " blob=" + .sha + " size=" + (.size|tostring)'
```

## meta-tegra (2024 feed pin)

- Repository: <https://github.com/avocado-linux/vendor-meta-tegra>
- Branch: `scarthgap`
- Commit: `053a4e97d356499ab028a59c17a5ee36e2c5b8c2` (committer date
  2026-08-30T15:29:54Z)

Named by `kas/vendor/nvidia.yml` at the meta-avocado commit above, key
`repos.meta-tegra.commit`:

```sh
gh api "repos/avocado-linux/meta-avocado/contents/kas/vendor/nvidia.yml?ref=e5b750c61c09b16deea4cd21cb01586e1d39d874" --jq .content | base64 -d
```

### L4T version

`L4T_VERSION ?= "36.5.2"` in `classes/l4t_version.bbclass`:

```sh
gh api "repos/avocado-linux/vendor-meta-tegra/contents/classes/l4t_version.bbclass?ref=053a4e97d356499ab028a59c17a5ee36e2c5b8c2" --jq .content | base64 -d
```

### URL variables

From `classes-recipe/l4t_bsp.bbclass` at the pin:

- `L4T_BSP_NAME ??= "releases"`
- `L4T_SRCS_NAME ??= "sources"`
- `L4T_BSP_PREFIX ??= "Jetson"`
- `L4T_URI_BASE ?= "https://developer.download.nvidia.com/embedded/L4T/${@l4t_release_dir(d)}/${L4T_BSP_NAME}"`
- `l4t_release_dir()` turns `36.5.2` into `r36_Release_v5.2`

```sh
gh api "repos/avocado-linux/vendor-meta-tegra/contents/classes-recipe/l4t_bsp.bbclass?ref=053a4e97d356499ab028a59c17a5ee36e2c5b8c2" --jq .content | base64 -d | head -40
```

Neither `conf/layer.conf` nor `conf/machine/include/tegra-common.inc` at the
pin overrides `L4T_URI_BASE`, `L4T_BSP_PREFIX`, `L4T_BSP_NAME` or
`L4T_SRCS_NAME`.

### NVIDIA BSP archive

`recipes-bsp/tegra-binaries/tegra-binaries-36.5.2.inc` fetches
`${L4T_URI_BASE}/${L4T_BSP_PREFIX}_Linux_R${L4T_VERSION}_aarch64.tbz2` with
`L4T_BSP_NAME` left at `releases`, which resolves to:

- URL: <https://developer.download.nvidia.com/embedded/L4T/r36_Release_v5.2/releases/Jetson_Linux_R36.5.2_aarch64.tbz2>
- sha256: `752326264c5e16826d3044a78e59ae06109467d37705143b94e664c91a471f47`
- Size: 749742254 bytes (HTTP `content-length`)

```sh
gh api "repos/avocado-linux/vendor-meta-tegra/contents/recipes-bsp/tegra-binaries/tegra-binaries-36.5.2.inc?ref=053a4e97d356499ab028a59c17a5ee36e2c5b8c2" --jq .content | base64 -d
curl -sIL https://developer.download.nvidia.com/embedded/L4T/r36_Release_v5.2/releases/Jetson_Linux_R36.5.2_aarch64.tbz2
```

The sha256 is the recipe's `SRC_URI[sha256sum]`. It was not re-hashed from a
download here; task 2.2 verifies it before extracting.

### NVIDIA device-tree sources

`recipes-kernel/nvidia-kernel-oot/nvidia-kernel-oot_36.5.2.bb` sets
`TEGRA_SRC_SUBARCHIVE` to
`Linux_for_Tegra/source/kernel_oot_modules_src.tbz2` and
`Linux_for_Tegra/source/nvidia_kernel_display_driver_source.tbz2`, then
requires `recipes-bsp/tegra-sources/tegra-sources-36.5.2.inc`. That include sets
`L4T_BSP_NAME = "${L4T_SRCS_NAME}"` (`sources`) and fetches
`${L4T_URI_BASE}/public_sources.tbz2`, extracting each subarchive from it.

```sh
gh api "repos/avocado-linux/vendor-meta-tegra/contents/recipes-kernel/nvidia-kernel-oot/nvidia-kernel-oot_36.5.2.bb?ref=053a4e97d356499ab028a59c17a5ee36e2c5b8c2" --jq .content | base64 -d
gh api "repos/avocado-linux/vendor-meta-tegra/contents/recipes-bsp/tegra-sources/tegra-sources-36.5.2.inc?ref=053a4e97d356499ab028a59c17a5ee36e2c5b8c2" --jq .content | base64 -d
```

Outer archive, the unit the recipe fetches and checksums:

- URL: <https://developer.download.nvidia.com/embedded/L4T/r36_Release_v5.2/sources/public_sources.tbz2>
- sha256: `4347a718e828edebee0d776d2110870d02a1d5766665d06c0abd3f325b1801d5`
  (recipe `SRC_URI[sha256sum]`, and the sha256 of the downloaded file matched it)
- Size: 232402561 bytes

```sh
curl -sSfL -o public_sources.tbz2 https://developer.download.nvidia.com/embedded/L4T/r36_Release_v5.2/sources/public_sources.tbz2
sha256sum public_sources.tbz2
```

Device-tree subarchive, the `TEGRA_SRC_SUBARCHIVE` entry that carries the
device tree. It has no URL of its own; it is a member of the outer archive:

- Member: `Linux_for_Tegra/source/kernel_oot_modules_src.tbz2`
- sha256: `d5f334212b2c3bc4bb3047a4d2df9c339e0378730fa3d6c87d5050304aaf2812`

```sh
tar -xjOf public_sources.tbz2 Linux_for_Tegra/source/kernel_oot_modules_src.tbz2 > kernel_oot_modules_src.tbz2
sha256sum kernel_oot_modules_src.tbz2
```

The second subarchive, `nvidia_kernel_display_driver_source.tbz2`, holds the
display driver and no device tree.

Device-tree directory: `hardware/nvidia/` inside the subarchive, unpacked into
the recipe's `${S}`. `nvidia-kernel-oot.inc` installs it as the device-tree
source tree with `cp -R ${S}/hardware/nvidia/ ${D}/usr/src/device-tree`. The
SOM and carrier sources live under `hardware/nvidia/t23x/nv-public/`, for
example `nv-platform/tegra234-p3768-0000+p3767-0000-nv.dts`.

```sh
gh api "repos/avocado-linux/vendor-meta-tegra/contents/recipes-kernel/nvidia-kernel-oot/nvidia-kernel-oot.inc?ref=053a4e97d356499ab028a59c17a5ee36e2c5b8c2" --jq .content | base64 -d | grep -n 'hardware/nvidia'
tar -tjf kernel_oot_modules_src.tbz2 | grep -c '^hardware/nvidia/'
```

## Device-tree compiler

- Version: `DTC v1.8.1`

```sh
dtc --version
```

## Carrier files

Filled by task 5.4 once every carrier file exists.
