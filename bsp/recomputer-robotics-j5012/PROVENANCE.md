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

Third member, used only by the DTB build (task 2.4) and not named by the
recipe's `TEGRA_SRC_SUBARCHIVE`: the kernel source tarball, from which only
`kernel/kernel-jammy-src/include/dt-bindings` is extracted. Seeed's DTS includes
`dt-bindings/net/ti-dp83867.h`, which `kernel_oot_modules_src.tbz2` does not
carry.

- Member: `Linux_for_Tegra/source/kernel_src.tbz2`
- sha256: `2a26015d13a2c4551c1266c8ec7503fc27b487d1cf74ccf3965d0633dacced1d`

```sh
tar -xjOf public_sources.tbz2 Linux_for_Tegra/source/kernel_src.tbz2 > kernel_src.tbz2
sha256sum kernel_src.tbz2
```

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

Every file under `stone/carrier-bsp/`, with where it came from. Seeed paths are
in `Seeed-Studio/Linux_for_Tegra` at commit
`f9a68317fbe3d276efc25b14b8b72339a1cc5d5c`.

### Seeed-derived BCT files (byte-identical copies)

Each is committed unchanged. The upstream sha256 and the committed sha256 are
equal for all three.

| Committed file | Seeed path | sha256 (upstream and committed) |
|---|---|---|
| `recomputer-robotics-agx-orin-j501x-pinmux.dtsi` | `bootloader/generic/BCT/recomputer-robotics-agx-orin-j501x-pinmux.dtsi` | `e02edfe55e57c250b66674134083628229f7317c361a52c8da4ea7f83125cfb9` |
| `recomputer-robotics-agx-orin-j501x-gpio-default.dtsi` | `bootloader/generic/BCT/recomputer-robotics-agx-orin-j501x-gpio-default.dtsi` | `d1c65885d57af7881d3c114710e078ec0d00772163de694c7056d6bb9203a69e` |
| `recomputer-robotics-agx-orin-j501x-padvoltage-default.dtsi` | `bootloader/generic/BCT/recomputer-robotics-agx-orin-j501x-padvoltage-default.dtsi` | `d19202e0b2ccc7debb72427207abd977d27e6268a01c11dbc7e49a9db6f56064` |

```sh
gh api -H 'Accept: application/vnd.github.raw' 'repos/Seeed-Studio/Linux_for_Tegra/contents/bootloader/generic/BCT/recomputer-robotics-agx-orin-j501x-pinmux.dtsi?ref=f9a68317fbe3d276efc25b14b8b72339a1cc5d5c' | sha256sum
gh api -H 'Accept: application/vnd.github.raw' 'repos/Seeed-Studio/Linux_for_Tegra/contents/bootloader/generic/BCT/recomputer-robotics-agx-orin-j501x-gpio-default.dtsi?ref=f9a68317fbe3d276efc25b14b8b72339a1cc5d5c' | sha256sum
gh api -H 'Accept: application/vnd.github.raw' 'repos/Seeed-Studio/Linux_for_Tegra/contents/bootloader/generic/BCT/recomputer-robotics-agx-orin-j501x-padvoltage-default.dtsi?ref=f9a68317fbe3d276efc25b14b8b72339a1cc5d5c' | sha256sum
sha256sum stone/carrier-bsp/recomputer-robotics-agx-orin-j501x-pinmux.dtsi stone/carrier-bsp/recomputer-robotics-agx-orin-j501x-gpio-default.dtsi stone/carrier-bsp/recomputer-robotics-agx-orin-j501x-padvoltage-default.dtsi
```

### MB2 BCT misc override (derived from NVIDIA stock, one delta)

- Committed file: `tegra234-mb2-bct-misc-p3701-0000-seeed.dts`
- sha256: `e19569a43952ec2adad7fd6bf0e88fcba262163517dfdf2ed3bd302cabe146d4`
- Authored in this extension. It has no upstream copy under this name.
- Builds on two stock files from the NVIDIA BSP archive
  (`Jetson_Linux_R36.5.2_aarch64.tbz2`, sha256 recorded above):
  `Linux_for_Tegra/bootloader/generic/BCT/tegra234-mb2-bct-misc-p3701-0000.dts`
  (the file it replaces, which only includes the common dtsi) and
  `Linux_for_Tegra/bootloader/tegra234-mb2-bct-common.dtsi` (the file it
  includes).
- The one delta: `cvb_eeprom_read_size = <0x0>;`, replacing the stock
  `<0x100>`, so MB2 skips the carrier EEPROM read. cvm_eeprom_* is unchanged.
- Evidence for the delta: Seeed's own `bootloader/tegra234-mb2-bct-common.dtsi`
  at the pinned commit differs from the stock common dtsi in exactly this one
  line (stock `<0x100>`, Seeed `<0>`). The diff is kept in the change's
  `evidence/mb2-common.diff`.
- No SCR file ships. Task 2.3 concluded the stock SCR config is used unchanged
  (verdict stock, user decision 2026-09-29). The diff Seeed's SCR would have
  added is recorded in the change's `evidence/mb2-scr.md` and is not shipped.

```sh
sha256sum stone/carrier-bsp/tegra234-mb2-bct-misc-p3701-0000-seeed.dts
tar -xjOf Jetson_Linux_R36.5.2_aarch64.tbz2 Linux_for_Tegra/bootloader/tegra234-mb2-bct-common.dtsi > stock-common.dtsi
gh api -H 'Accept: application/vnd.github.raw' 'repos/Seeed-Studio/Linux_for_Tegra/contents/bootloader/tegra234-mb2-bct-common.dtsi?ref=f9a68317fbe3d276efc25b14b8b72339a1cc5d5c' | diff stock-common.dtsi -
```

### Kernel DTB

- Committed file: `tegra234-j501x-0000+p3701-0005-recomputer-robo.dtb`
- sha256: `f3f7eb9ecacf18d7f7be03002eaf2954c3bd8d74e5627c1af68268334109b44b`,
  the value `dtb/build-dtb.sh --verify` enforces (it rebuilds and compares).
- Built from Seeed commit `f9a68317fbe3d276efc25b14b8b72339a1cc5d5c`. Four
  Seeed files are overlaid onto NVIDIA's `hardware/nvidia/t23x/nv-public`, all
  under `source/hardware/nvidia/t23x/nv-public/` in the Seeed repo, each checked
  against its git blob sha:

| Path under nv-public | git blob sha |
|---|---|
| `nv-platform/tegra234-j501x-0000+p3701-0000-recomputer-robo.dts` | `33f9b31e2d4ab642f6cff1db6ee69a60969456d5` |
| `nv-platform/tegra234-j501x-0000+p3701-0005-recomputer-robo.dts` | `2dc696746cee3872e6ff0a3432d0ad8d48d82365` |
| `nv-platform/tegra234-p3737-0000+p3701-xxxx-nv-common.dtsi` | `72289c86ac0bb14350a7a3126f1077aa11b6f134` |
| `tegra234-j501x-0000+p3701-0000.dts` | `b4013dab7f6cd34b948cf162eea677990bce5d72` |

- NVIDIA inputs: `public_sources.tbz2` and its members
  `kernel_oot_modules_src.tbz2` (device-tree tree) and `kernel_src.tbz2`
  (dt-bindings headers only); hashes are in the sections above.
- Compiler: `DTC v1.8.1`. Preprocessor flags:
  `-nostdinc -undef -D__DTS__ -DLINUX_VERSION=600 -DTEGRA_HOST1X_DT_VERSION=2`
  with `-I` for `hardware/nvidia/tegra/nv-public`, `nv-public/include/kernel`,
  `nv-public/include/nvidia-oot`, `nv-public/include/platforms`, `nv-public`
  and the kernel `include/` directory holding `dt-bindings`, run with
  `-x assembler-with-cpp`. dtc flags: `-b 0 -@` plus the `-Wno-*` suppressions
  the installed dtc accepts, and the same directories as `-i`. The script is
  the authoritative source for the exact invocation.

```sh
sha256sum stone/carrier-bsp/tegra234-j501x-0000+p3701-0005-recomputer-robo.dtb
bash bsp/recomputer-robotics-j5012/dtb/build-dtb.sh --verify
```

### carrier.env

- Committed file: `carrier.env`
- sha256: `8c8e4f5746680c8793da3607c82d0882d1201ddd00f297bd1e9692cbd9fc764d`
- Authored in this extension; there is no upstream copy. Its values are taken
  from Seeed's `recomputer-robo-agx-orin-j501x.conf` at the pinned commit (git
  blob `b642384ac4a2b1477724f22179d41018d719b541`): the `DTB_FILE`,
  `PINMUX_CONFIG`, `PMC_CONFIG` and `ODMDATA` lines. `MB2BCT_CFG` points at the
  override above and the two `CHECK_*` pins repeat the stock devkit values;
  neither comes from the conf.

```sh
gh api "repos/Seeed-Studio/Linux_for_Tegra/contents/recomputer-robo-agx-orin-j501x.conf?ref=f9a68317fbe3d276efc25b14b8b72339a1cc5d5c" --jq .sha
sha256sum stone/carrier-bsp/carrier.env
```
