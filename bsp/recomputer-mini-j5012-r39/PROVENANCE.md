# Provenance: recomputer-mini-j5012-r39

Every pinned input this extension is built from, each followed by the command
that produced the value. Re-run a command and compare its output with the value
recorded here; a different result means the input has drifted. The JetPack 7.2
(L4T R39.2.0) inputs were resolved on 2026-10-06. Pinned sources are read
through the GitHub API at a fixed commit, never from a local checkout on another
branch.

This extension is a copy of `bsp/recomputer-mini-j5012` (JetPack 6.x, L4T
R36.5.2), retargeted to the 2026 line. Sections under "R39.2.0 inputs" are
current. Sections under "Superseded R36.5.x inputs" are kept only where a file
in this extension still depends on them, and each says so.

This file is a build record. It is not listed in `package_files` and does not
ship in the extension package.

## R39.2.0 inputs

### Seeed carrier sources (r39.2.0)

- Repository: <https://github.com/Seeed-Studio/Linux_for_Tegra>
- Branch: `r39.2.0`
- Commit: `df17ed28201645fcaeb6c8e6fb82a0b4592faefb` (2026-09-14, "Add readme.md for GitHub")

```sh
gh api repos/Seeed-Studio/Linux_for_Tegra/commits/r39.2.0 --jq .sha
gh api repos/Seeed-Studio/Linux_for_Tegra/branches --paginate --jq '.[].name' | grep -x 'r39.2.0'
```

The branch moves; the commit is the pin. Every Seeed file below is fetched at
`?ref=df17ed28201645fcaeb6c8e6fb82a0b4592faefb`, never at `r39.2.0`.

Board configuration this extension follows:

- File: `recomputer-mini-agx-orin-j501x.conf` at the commit above
- sha256: `40b47214a673746b723aa044a04f4067606755c5bf88fd1db6fdceb1bfa61f7e`
- Against the r36.5.0 conf (`a45a0642...fa506`) the only differences are the
  `source` line (`p3701.conf.common` instead of `p3737-0000-p3701-0000.conf.common`)
  and 49 added lines (`EMMC_CFG` selection and `update_flash_args_common`).
  `ODMDATA`, `PINMUX_CONFIG`, `PMC_CONFIG` and the SKU 0005 `DTB_FILE` are
  unchanged; their line numbers moved by +49, which `carrier.env` reflects.

```sh
gh api -H 'Accept: application/vnd.github.raw' 'repos/Seeed-Studio/Linux_for_Tegra/contents/recomputer-mini-agx-orin-j501x.conf?ref=df17ed28201645fcaeb6c8e6fb82a0b4592faefb' | sha256sum
gh api -H 'Accept: application/vnd.github.raw' 'repos/Seeed-Studio/Linux_for_Tegra/contents/recomputer-mini-agx-orin-j501x.conf?ref=f9a68317fbe3d276efc25b14b8b72339a1cc5d5c' | diff - <(gh api -H 'Accept: application/vnd.github.raw' 'repos/Seeed-Studio/Linux_for_Tegra/contents/recomputer-mini-agx-orin-j501x.conf?ref=df17ed28201645fcaeb6c8e6fb82a0b4592faefb')
```

Carrier DTS present on the same branch (content not yet read; task 2.2 ports
the DTB build): `source/hardware/nvidia/t23x/nv-public/nv-platform/tegra234-j501x-0000+p3701-0005-recomputer-mini.dts`.

```sh
gh api 'repos/Seeed-Studio/Linux_for_Tegra/git/trees/df17ed28201645fcaeb6c8e6fb82a0b4592faefb?recursive=1' --jq '.tree[].path' | grep 'recomputer-mini'
```

The three Seeed-copied dtsi files in `stone/carrier-bsp/` are byte-identical at
r39.2.0 to the r36.5.0 copies (sha256 values in the table below):

```sh
for f in gpio-default padvoltage-default pinmux; do
  gh api -H 'Accept: application/vnd.github.raw' "repos/Seeed-Studio/Linux_for_Tegra/contents/bootloader/generic/BCT/recomputer-mini-agx-orin-j501x-$f.dtsi?ref=df17ed28201645fcaeb6c8e6fb82a0b4592faefb" | sha256sum
done
```

### meta-tegra pin (2026 line)

- Repository: <https://github.com/avocado-linux/vendor-meta-tegra>
- Branch: `wrynose`
- Commit: `727633deca9b72643112d1aebadafb736dad60c0` (committer date
  2026-08-10T04:49:40-07:00)
- `L4T_VERSION ?= "39.2.0"` in `classes/l4t_version.bbclass`

Named by `kas/vendor/nvidia.yml` on meta-avocado `wrynose` (head
`86202be527224f2b5da7549ba3cbe66a0c4b4189`, committer date
2026-10-02T07:23:25-06:00 when read), key `repos.meta-tegra.commit`:

```sh
gh api 'repos/avocado-linux/meta-avocado/contents/kas/vendor/nvidia.yml?ref=wrynose' --jq .content | base64 -d | grep -A4 'meta-tegra:'
git -C ~/repos/work/peridio/meta-tegra log -1 --format='%H %cI' 727633de
git -C ~/repos/work/peridio/meta-tegra show 727633de:classes/l4t_version.bbclass | grep -n L4T_VERSION
```

`TEGRA_FLASHVAR_ODMDATA` is the BitBake variable that bakes the ODMDATA default
(`conf/machine/include/agx-orin.inc:9` at the pin); `TEGRA_FLASHVARS` still
lists the flashvars key `ODMDATA`. See `stone/carrier-bsp/carrier.env` for why
the carrier knob keeps the name `CARRIER_ENV_ODMDATA`.

```sh
git -C ~/repos/work/peridio/meta-tegra grep -n 'ODMDATA' 727633de -- conf/machine/include/agx-orin.inc conf/machine/include/tegra-common.inc
```

### 2026/next feed (jetson-agx-orin)

- Target repo: <https://repo.avocadolinux.org/2026/next/target/jetson-agx-orin>
  (25710 packages, repomd revision `1789555949`)
- Ext repo: <https://repo.avocadolinux.org/2026/next/target/jetson-agx-orin-ext>
  (15 packages)
- Snapshot pointer: `{"id": "311", "created": "2026-09-18T04:44:45Z"}`; the
  snapshot's repomd revision is the same `1789555949`.
- `2026/edge/target/jetson-agx-orin` answers 404, so the channel is `next`.
- SDK image: `docker.io/avocadolinux/sdk:2026`, digest
  `sha256:d21da5f658d97be3d06a2452e31d68396a755f2735d27ff2d088c0b3b5e79196`
  (`2026-edge` is the same digest; there is no `2026-next` tag, so `avocado.yaml`
  names `sdk:2026`).

```sh
curl -s https://repo.avocadolinux.org/2026/next/target/jetson-agx-orin/snapshots-latest.json
curl -s https://repo.avocadolinux.org/2026/next/snapshots/311/target/jetson-agx-orin/repodata/repomd.xml | grep -o '<revision>[0-9]*</revision>'
curl -sI -o /dev/null -w '%{http_code}\n' https://repo.avocadolinux.org/2026/edge/target/jetson-agx-orin/repodata/repomd.xml
docker pull docker.io/avocadolinux/sdk:2026 | grep -i digest
```

Versions below were read from the snapshot-311 primary metadata:

```sh
B=https://repo.avocadolinux.org/2026/next/snapshots/311/target/jetson-agx-orin
P=$(curl -s $B/repodata/repomd.xml | grep -o 'href="[^"]*primary.xml.gz"' | cut -d'"' -f2)
curl -s $B/$P | zcat | grep -o '<name>[^<]*</name>' | sort -u > /var/tmp/claude-code/peridio/2026-10-06-jp72/snapshot311-names.txt
```

- Kernels: `kernel-6.18.35-yocto-standard` (linux-yocto 6.18.35, the default) and
  `kernel-6.8.12-l4t-r39.2.0-1021.21` (linux-noble-nvidia-tegra 6.8.12).
- Out-of-tree modules at version 39.2.0, each present for both kernels:
  `nv-kernel-module-nvethernet`, `nv-kernel-module-mttcan`,
  `nv-kernel-module-i2c-nvvrs11` (the nvvrs11 package), `nv-kernel-module-nvpps`.
- `l4t-launcher` 39.2.0; `systemd-jetson-masks` 1.0 (the unit that replaces the
  removed `getty@getty` mask).
- Ext repo: `avocado-bsp-jetson-agx-orin` 0.2.0, `avocado-ext-dev` 0.2.0,
  `avocado-ext-sshd-dev` 0.1.0.

```sh
grep -x -e '<name>kernel-6.18.35-yocto-standard</name>' -e '<name>kernel-6.8.12-l4t-r39.2.0-1021.21</name>' -e '<name>l4t-launcher</name>' -e '<name>systemd-jetson-masks</name>' /var/tmp/claude-code/peridio/2026-10-06-jp72/snapshot311-names.txt
grep -c 'nv-kernel-module-\(nvethernet\|mttcan\|i2c-nvvrs11\|nvpps\)-6.18.35-yocto-standard</name>' /var/tmp/claude-code/peridio/2026-10-06-jp72/snapshot311-names.txt
```

The probe note with the full inventory is `jp72-probe.md` in the devspec
change `jetson-agx-emmc-window-prep` (evidence directory).

### Module list changes against the R36.5.2 extension

Each entry that was dropped or renamed, and the observation behind it. Package
existence was checked against the primary metadata of the 2026/next feed above
(an entry counts as resolvable when a `kernel-module-<name>-6.18.35-yocto-standard`
package or an RPM Provides of the unqualified name exists).

| Entry (R36.5.2 extension) | Change | Reason |
|---|---|---|
| `kernel-6.6.*` stanza | now `kernel-6.18.*` | the 2026 default kernel is linux-yocto 6.18 (`linux-yocto_6.18.bbappend` on meta-avocado wrynose); the Mini keeps the same `drm-display-helper` module, which exists for 6.18 |
| `nv-kernel-module-snd-soc-tegra186-asrc`, `-tegra186-dspk`, `-tegra210-admaif`, `-adx`, `-ahub`, `-amx`, `-dmic`, `-i2s`, `-mixer`, `-mvc`, `-ope`, `-sfc` | renamed to `kernel-module-snd-soc-<same>` | on 2026/next these drivers are built in-tree: the feed has `kernel-module-snd-soc-tegra210-i2s-6.18.35-yocto-standard` etc. and no `nv-kernel-module-` package or Provides under the old names |
| `nv-kernel-module-snd-soc-tegra-machine-driver` | dropped | no package or Provides of that name exists for either kernel |
| `nv-kernel-module-snd-soc-tegra210-afc` | kept | still an out-of-tree `nv-kernel-module-` package for both kernels |
| `kernel-module-snd-soc-tegra186-arad`, `kernel-module-snd-soc-tegra-utils` | kept | resolve through the Provides of the `nv-kernel-module-` packages |
| `kernel-module-crct10dif-ce` | dropped | the feed has it only for 6.8 (`kernel-module-crct10dif-ce-6.8.12-...`); the unqualified name would pull the 6.8 package next to the 6.18 kernel |
| `tegra-nvphs`, `tegra-nvstartup` | dropped | neither is in the feed, and meta-tegra at the pin has no recipe for either (`git ls-tree` finds only `tegra-nvpower`) |
| `nv-kernel-module-nvethernet`, `-mttcan`, `-i2c-nvvrs11`, `-nvpps` | kept | the out-of-tree modules, version 39.2.0, present for 6.18 |
| every other `kernel-module-*` entry | kept | each has a 6.18 package or Provides in the snapshot |

The statement that the tegra210 audio drivers are in-tree on R39 comes from the
feed (above), not from meta-tegra release notes: meta-tegra at the pin carries
no notes about it (`git grep -i 'audio\|snd' 727633de -- conf recipes-kernel`
prints nothing).

```sh
git -C ~/repos/work/peridio/meta-tegra grep -n -i 'audio\|snd' 727633de -- conf recipes-kernel
git -C ~/repos/work/peridio/meta-tegra ls-tree -r --name-only 727633de | grep -i 'nvphs\|nvstartup'
grep -c 'kernel-module-snd-soc-tegra210-i2s-6.18.35-yocto-standard' /var/tmp/claude-code/peridio/2026-10-06-jp72/snapshot311-names.txt
grep -c 'nv-kernel-module-snd-soc-tegra210-i2s' /var/tmp/claude-code/peridio/2026-10-06-jp72/snapshot311-names.txt
```

The last command prints `0`, the one before it a non-zero count.

### Kernel choice

6.18 (linux-yocto 6.18.35), not 6.8. The 2026 wrynose branch builds
`linux-yocto_6.18.bbappend` with the Tegra module packagegroups and the OOT
packagegroup pulled in, and the 6.8 `linux-noble-nvidia-tegra` kernel is built
alongside it.

```sh
git -C ~/repos/work/.worktrees/meta-avocado/jetson-orin-nano-ethernet-modules-wrynose show origin/wrynose:meta-avocado-nvidia/recipes-kernel/linux/linux-yocto_6.18.bbappend | head -20
```

### Provisioning script (2026 line)

`meta-avocado-nvidia/stone/tegra/stone-provision-tegraflash.sh` on meta-avocado
`wrynose`, git blob `d698009f3309d98064516ae8ffe176d85f58f129` at the head
recorded above. It applies `CARRIER_FV_*` / `CARRIER_ENV_*` knobs and warns
`target not found` for a knob naming a key the file does not carry.

```sh
git -C ~/repos/work/.worktrees/meta-avocado/jetson-orin-nano-ethernet-modules-wrynose rev-parse origin/wrynose:meta-avocado-nvidia/stone/tegra/stone-provision-tegraflash.sh
```

### Overlay files

- `overlay/etc/hostname` and `overlay/etc/profile.d/10-sbin-path.sh`, both mode
  0644 on disk, carried over unchanged from the R36.5.2 extension.
- The masked `getty@getty.service` symlink of the R36.5.2 extension is not
  carried: `systemd-jetson-masks` is in the 2026/next feed (above), which fixes
  the bare `getty@` instance at its source. The console is `serial-getty@ttyTCU0`
  from `systemd-getty-generator`.

```sh
find bsp/recomputer-mini-j5012-r39/overlay -printf '%y %m %p\n'
```

## Carrier files

Every file in `stone/carrier-bsp/`, with its origin. All Seeed paths are read at
the commit of the section it is listed under. For the three Seeed-copied files the
upstream and committed sha256 are equal (the copy is byte-exact), at both
r36.5.0 (`f9a68317fbe3d276efc25b14b8b72339a1cc5d5c`) and r39.2.0.

### Seeed-copied files

| Committed file (`stone/carrier-bsp/`) | Seeed path | Upstream sha256 (r36.5.0 and r39.2.0) | Committed sha256 |
|---|---|---|---|
| `recomputer-mini-agx-orin-j501x-gpio-default.dtsi` | `bootloader/generic/BCT/recomputer-mini-agx-orin-j501x-gpio-default.dtsi` | `5efec361e3c01d0c185c4489ba7f1964fd16a8a6f625dedf01de4a8dfc7b0cdb` | `5efec361e3c01d0c185c4489ba7f1964fd16a8a6f625dedf01de4a8dfc7b0cdb` |
| `recomputer-mini-agx-orin-j501x-padvoltage-default.dtsi` | `bootloader/generic/BCT/recomputer-mini-agx-orin-j501x-padvoltage-default.dtsi` | `aede16fea255ea7b87669eeece54fe5d2ee7e28388812597c199922609017ed5` | `aede16fea255ea7b87669eeece54fe5d2ee7e28388812597c199922609017ed5` |
| `recomputer-mini-agx-orin-j501x-pinmux.dtsi` | `bootloader/generic/BCT/recomputer-mini-agx-orin-j501x-pinmux.dtsi` | `18115da2682e03a50b893756958e3fb6c399c751bb8d91ab09f328977dbc7698` | `18115da2682e03a50b893756958e3fb6c399c751bb8d91ab09f328977dbc7698` |

```sh
# upstream sha256, per Seeed path in the table
gh api -H 'Accept: application/vnd.github.raw' 'repos/Seeed-Studio/Linux_for_Tegra/contents/<Seeed path>?ref=df17ed28201645fcaeb6c8e6fb82a0b4592faefb' | sha256sum
# committed sha256
sha256sum bsp/recomputer-mini-j5012-r39/stone/carrier-bsp/<committed file>
```

### MB2 BCT misc override (authored, carried over unchanged)

- Committed file: `tegra234-mb2-bct-misc-p3701-0000-seeed.dts`
- Committed sha256: `5611b92a6bceda9af9494611155287151972d474bb3c7809c3e5b5aa2c0727d0`
- Builds on the stock file `Linux_for_Tegra/bootloader/generic/BCT/tegra234-mb2-bct-misc-p3701-0000.dts`
  and its include `Linux_for_Tegra/bootloader/tegra234-mb2-bct-common.dtsi`. It was
  authored against the R36.5.2 stock files and was not re-diffed against the
  R39.2.0 stock files; the single delta it carries (`cvb_eeprom_read_size`
  from `0x100` to `0`) is still present in Seeed's `bootloader/tegra234-mb2-bct-common.dtsi`
  at r39.2.0.

```sh
sha256sum bsp/recomputer-mini-j5012-r39/stone/carrier-bsp/tegra234-mb2-bct-misc-p3701-0000-seeed.dts
gh api -H 'Accept: application/vnd.github.raw' 'repos/Seeed-Studio/Linux_for_Tegra/contents/bootloader/tegra234-mb2-bct-common.dtsi?ref=df17ed28201645fcaeb6c8e6fb82a0b4592faefb' | grep -n cvb_eeprom_read_size
```

### carrier.env (authored, not copied from Seeed)

- Encodes Seeed `recomputer-mini-agx-orin-j501x.conf` at r39.2.0 (sha256
  `40b47214a673746b723aa044a04f4067606755c5bf88fd1db6fdceb1bfa61f7e`).
- The ODMDATA knob keeps the name `CARRIER_ENV_ODMDATA`; the 2026 rename is of the
  BitBake variable `TEGRA_FLASHVAR_ODMDATA`, not of the `.env.initrd-flash` key
  (reasoning in the file's comment).

| Knob | Seeed conf line (r39.2.0) |
|---|---|
| `CARRIER_ENV_DTBFILE` | 93 (`DTB_FILE`, SKU 0005 branch, lines 90-93) |
| `CARRIER_FV_PINMUX_CONFIG` | 110 (`PINMUX_CONFIG`) |
| `CARRIER_FV_PMC_CONFIG` | 111 (`PMC_CONFIG`) |
| `CARRIER_ENV_ODMDATA` | 109 (`ODMDATA`) |
| `CARRIER_FV_CHECK_BOARDID`, `CARRIER_FV_CHECK_BOARDSKU` | 86-97 (SKU branches; also in the stock flashvars) |
| `CARRIER_FV_MB2BCT_CFG` | none; names the authored MB2 file above |
| `TBCDTB_FILE` (deliberately unset) | 99 |

```sh
gh api -H 'Accept: application/vnd.github.raw' 'repos/Seeed-Studio/Linux_for_Tegra/contents/recomputer-mini-agx-orin-j501x.conf?ref=df17ed28201645fcaeb6c8e6fb82a0b4592faefb' | cat -n | sed -n 83,115p
```

### Kernel DTB (not yet ported)

`stone/carrier-bsp/tegra234-j501x-0000+p3701-0005-recomputer-mini.dtb` and
`dtb/build-dtb.sh` are copied unchanged from the R36.5.2 extension. The DTB was
built from R36.5.2 public sources (see the superseded sections below) and the
script still pins them. The R39.2.0 port is a separate step; until it lands this
DTB is an R36.5.2 artifact and must not be read as the R39 one.

```sh
sha256sum bsp/recomputer-mini-j5012-r39/stone/carrier-bsp/tegra234-j501x-0000+p3701-0005-recomputer-mini.dtb
grep -n 'R36\|36.5' bsp/recomputer-mini-j5012-r39/dtb/build-dtb.sh | head
```

## Superseded R36.5.x inputs

Inherited from `bsp/recomputer-mini-j5012`. Nothing below is an input to the
R39.2.0 extension except where a line says the unported DTB build still reads it.

### Seeed carrier sources (r36.5.0), superseded

- Branch `r36.5.0`, commit `f9a68317fbe3d276efc25b14b8b72339a1cc5d5c`, conf
  sha256 `a45a06424c320019cbbaa70b2f01c9f5f49fc0d0cb4048183a00ce5e131fa506`.
  Still read by the unported DTB build for its four device-tree inputs
  (git blob shas, paths under `source/hardware/nvidia/t23x/nv-public/`):

| Blob sha | Path |
|---|---|
| `ca325af7ef26132460504d59aa1b031f7f6eddd3` | `nv-platform/tegra234-j501x-0000+p3701-0000-recomputer-mini.dts` |
| `e01b6915d8cbf758fd6a9f01296b3c70b08f6eeb` | `nv-platform/tegra234-j501x-0000+p3701-0005-recomputer-mini.dts` |
| `72289c86ac0bb14350a7a3126f1077aa11b6f134` | `nv-platform/tegra234-p3737-0000+p3701-xxxx-nv-common.dtsi` |
| `b4013dab7f6cd34b948cf162eea677990bce5d72` | `tegra234-j501x-0000+p3701-0000.dts` |

```sh
gh api repos/Seeed-Studio/Linux_for_Tegra/commits/r36.5.0 --jq .sha
grep -n 'SEEED_FILES=' -A4 bsp/recomputer-mini-j5012-r39/dtb/build-dtb.sh
```

### The shipped board image (context, not a pin)

The board runs Seeed's own image: Seeed branch `r36.4.4`, L4T 36.4.4, JetPack
6.2.1. Its QSPI firmware stays; this extension's R39 kernel on that firmware is
an unsupported mix and may not boot.

### meta-avocado 2024 feed and meta-tegra scarthgap, superseded

- meta-avocado `scarthgap` commit `e5b750c61c09b16deea4cd21cb01586e1d39d874`
  (provisioning script blob `821bc7ce648fa7b0aca66f8f674145b8d8c62c38`).
- vendor-meta-tegra `scarthgap` commit `053a4e97d356499ab028a59c17a5ee36e2c5b8c2`,
  `L4T_VERSION ?= "36.5.2"`.
- Replaced by the wrynose pins above.

```sh
gh api repos/avocado-linux/meta-avocado/commits/e5b750c61c09b16deea4cd21cb01586e1d39d874 --jq '.sha + " " + .commit.committer.date'
gh api repos/avocado-linux/vendor-meta-tegra/commits/053a4e97d356499ab028a59c17a5ee36e2c5b8c2 --jq '.sha + " " + .commit.committer.date'
```

### NVIDIA R36.5.2 archives, still read by the unported DTB build

- `Jetson_Linux_R36.5.2_aarch64.tbz2`, sha256
  `752326264c5e16826d3044a78e59ae06109467d37705143b94e664c91a471f47`,
  <https://developer.download.nvidia.com/embedded/L4T/r36_Release_v5.2/releases/Jetson_Linux_R36.5.2_aarch64.tbz2>
- `public_sources.tbz2`, sha256
  `4347a718e828edebee0d776d2110870d02a1d5766665d06c0abd3f325b1801d5`,
  <https://developer.download.nvidia.com/embedded/L4T/r36_Release_v5.2/sources/public_sources.tbz2>
  - member `Linux_for_Tegra/source/kernel_oot_modules_src.tbz2`, sha256
    `d5f334212b2c3bc4bb3047a4d2df9c339e0378730fa3d6c87d5050304aaf2812`
    (carries `hardware/nvidia/`, the device-tree sources)
  - member `Linux_for_Tegra/source/kernel_src.tbz2`, sha256
    `2a26015d13a2c4551c1266c8ec7503fc27b487d1cf74ccf3965d0633dacced1d`
    (source of `kernel/kernel-jammy-src/include/dt-bindings`)

```sh
curl -sSfL -o public_sources.tbz2 https://developer.download.nvidia.com/embedded/L4T/r36_Release_v5.2/sources/public_sources.tbz2
sha256sum public_sources.tbz2
tar -xjOf public_sources.tbz2 Linux_for_Tegra/source/kernel_oot_modules_src.tbz2 | sha256sum
tar -xjOf public_sources.tbz2 Linux_for_Tegra/source/kernel_src.tbz2 | sha256sum
```

### Device-tree compiler and the R36.5.2 DTB

- `DTC v1.8.1` (`dtc --version`).
- R36.5.2 DTB output sha256 (enforced by the unported `dtb/build-dtb.sh --verify`):
  `07d830c48c1a697d7e0baa05e8f9ea9d32de64dc7b56027721f4474bc800953d`.

```sh
dtc --version
sha256sum bsp/recomputer-mini-j5012-r39/stone/carrier-bsp/tegra234-j501x-0000+p3701-0005-recomputer-mini.dtb
```
