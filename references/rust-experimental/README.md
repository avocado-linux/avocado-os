# Rust Reference Runtime (Experimental)

A reference runtime that demonstrates how to cross-compile and deploy a Rust application on Avocado OS. The app is a system vitals reporter — a single static binary that reads from `/proc` and logs structured JSON to the journal.

This showcases Rust's advantage on embedded: no runtime, no interpreter, no dependencies on the device. Just a compiled binary started by systemd.

## Prerequisites

- Docker Desktop running
- `avocado` CLI installed

## Build and Run

```bash
cd os/references/rust-experimental

# Install SDK (includes Rust toolchain), extensions, and runtime
avocado install -f -t qemux86-64

# Build the Rust app (compiles via cargo inside the SDK container) and assemble the image
avocado build -t qemux86-64

# Provision the bootable disk image
avocado provision -f -r dev -t qemux86-64

# Boot QEMU interactively
avocado sdk run -iE -t qemux86-64 vm dev
```

To SSH in from another terminal:

```bash
avocado sdk run -iE -t qemux86-64 vm dev --host-fwd "2222-:22"

# Then from another terminal:
ssh -o StrictHostKeyChecking=no -p 2222 root@localhost
```

> Boot takes ~70 seconds on macOS (no KVM acceleration). Login as `root` with an empty password.

## Observe the Rust Service

```bash
# Check the service is running
systemctl status ref-rust

# Watch JSON output streaming in real time
journalctl -u ref-rust -f

# Stop and start the service
systemctl stop ref-rust
systemctl start ref-rust
```

You'll see output like:

```json
{"hostname":"avocado-qemux86-64","uptime":42,"mem_total_kb":977972,"mem_free_kb":821488,"load_1m":"0.12"}
```

## Make a Change and Redeploy

All edits happen on the host — never on the device.

1. Edit `ref-rust/src/main.rs` on your host machine
2. Rebuild and reprovision:

```bash
avocado build -t qemux86-64 -e example-rust && avocado provision -f -r dev -t qemux86-64
```

3. Boot the new image and observe the changes via `journalctl -u ref-rust -f`

## Project Structure

```
rust-experimental/
├── README.md
├── avocado.yaml            # Runtime, extension, and SDK config
├── rust-compile.sh         # Cross-compiles via cargo inside SDK container
├── rust-install.sh         # Installs target-specific binary to extension sysroot
├── overlay/
│   └── usr/lib/systemd/system/
│       └── ref-rust.service    # Systemd unit file
└── ref-rust/               # Rust source code (cross-compiled in SDK)
    ├── Cargo.toml
    └── src/
        └── main.rs
```

## How It Works

### Cross-Compilation Pipeline

1. **`avocado build`** invokes `rust-compile.sh` inside the SDK Docker container
2. The SDK has the full Rust toolchain: `nativesdk-rust`, `nativesdk-cargo`, and `packagegroup-rust-cross-canadian-avocado-{{ target }}`
3. The compile script discovers the Rust target triple from `RUST_TARGET_PATH` (e.g. `x86_64-avocado-linux-gnu` for qemux86-64)
4. SDK-injected `RUSTFLAGS` and `CARGO_TARGET_*_RUSTFLAGS` are cleared to avoid conflicts
5. A `.cargo/config.toml` is generated with the correct `--sysroot` and linker flags for the target
6. `cargo build --release --target $RUST_TARGET` produces a cross-compiled binary at `ref-rust/target/$RUST_TARGET/release/ref_rust`
7. `rust-install.sh` discovers the same target triple, locates the binary under `target/$RUST_TARGET/release/`, and installs it to the extension sysroot at `/usr/bin/ref_rust`
8. The extension image is assembled with the binary and the systemd service unit

This approach works on any host architecture (including Apple Silicon) building for any target — true cross-compilation rather than QEMU emulation of the SDK.

### SDK Compile Packages

The `sdk.compile.example-rust-app.packages` section adds `libstd-rs` and `libstd-rs-dev` to the SDK container. These provide the pre-built Rust standard library for the target architecture, which is required for `cargo build --target` to succeed.

### Extension Config (in avocado.yaml)

| Field | Purpose |
|-------|---------|
| `overlay: overlay` | Systemd service unit merged into the root filesystem |
| `enable_services` | Enables `ref-rust.service` at boot |
| `on_merge` / `on_unmerge` | Restarts/stops the service on extension apply/remove |
| `packages.example-rust-app` | Declares the compiled package with compile and install scripts |
| `sdk.packages` | Cross-compilation toolchain packages installed in the SDK container |

### Rust vs React on Embedded

| Aspect | Rust | React |
|--------|------|-------|
| Device dependencies | None — single static binary | Node.js runtime, node_modules |
| Install script | 2 lines (copy binary) | Copy dist/, node_modules/, server.js |
| Binary size | ~1 MB | ~50 MB+ (with Node.js) |
| Runtime overhead | Minimal | V8 engine + garbage collector |

## What's Next

This reference produces a standalone binary that logs to the journal. Natural next steps:
- Use `serde_json` for structured serialization
- Add configuration via `/etc/ref-rust.conf`
- Extend the stats collected (disk usage, network traffic, temperature)
