# QEMU Heartbeat Reference Runtime

A tutorial runtime that demonstrates how to build a custom Avocado OS extension. The running example is a **device heartbeat service** — a systemd service that collects system vitals (uptime, memory, load) and logs them as structured JSON to the journal.

This is the "hello world" for embedded extension development on Avocado OS.

## Prerequisites

- Docker Desktop running
- `avocado` CLI installed

## Build and Run

```bash
cd runtimes-dev/qemu-heartbeat-ref

# Install SDK, extensions, and runtime dependencies (one-time setup)
avocado install -f

# Build extensions and assemble the runtime image
avocado build

# Provision the runtime (create the bootable disk image)
avocado provision -f -r dev

# Boot QEMU interactively inside the SDK container
avocado sdk run -iE vm dev
```

The `-f` flag skips interactive confirmation prompts. You can also run the install steps individually if needed:

```bash
avocado sdk install -f        # Install SDK toolchain
avocado ext install -f        # Install extension dependencies
avocado runtime install -f -r dev  # Install runtime dependencies
```

To SSH in from another terminal instead of using the QEMU console:

```bash
avocado sdk run -iE vm dev --host-fwd "2222-:22"

# Then from another terminal:
ssh -o StrictHostKeyChecking=no -p 2222 root@localhost
```

> Boot takes ~70 seconds on macOS (no KVM acceleration). `avocado sdk run` launches QEMU inside the SDK Docker container — no local QEMU install needed. `-i` = interactive (attach terminal), `-E` = pass environment variables.

## Observe the Heartbeat

Login as `root` with an empty password (set by the `config` extension). The heartbeat service starts automatically on boot via `enable_services`.

```bash
# Check the service is running
systemctl status heartbeat

# Watch JSON vitals streaming in real time
journalctl -u heartbeat -f

# Stop and start the service
systemctl stop heartbeat
systemctl start heartbeat
```

You'll see output like:

```json
{"uptime":142,"mem_free_kb":412356,"mem_total_kb":524288,"load_1m":"0.03","ts":1740700000}
```

## Project Structure

```
qemu-heartbeat-ref/
├── README.md            # This file
├── avocado.yaml         # Runtime config — defines extensions, packages, SDK
└── heartbeat/           # The heartbeat extension
    └── overlay/         # Files merged into the root filesystem
        ├── etc/
        │   └── heartbeat.conf           # Configuration (sample interval)
        └── usr/
            ├── local/bin/
            │   └── heartbeat.sh         # The heartbeat script
            └── lib/systemd/system/
                └── heartbeat.service    # Systemd unit file
```

## How the Extension Works

The `heartbeat` extension in `avocado.yaml` declares:

| Field | Purpose |
|-------|---------|
| `types: [sysext, confext]` | Built as a systemd system extension and config extension (confext required for service linking) |
| `overlay: heartbeat/overlay` | Files to merge into the root filesystem |
| `enable_services` | Enables `heartbeat.service` at boot |
| `on_merge` | Restarts the service when the extension is applied |
| `on_unmerge` | Stops the service when the extension is removed |

The script reads from `/proc` for system vitals and uses `printf` to produce structured JSON. Output goes to stdout, which systemd captures into the journal — the idiomatic approach for logging on systemd-based systems.

## Customization

Edit `heartbeat/overlay/etc/heartbeat.conf` to change the sample interval:

```sh
INTERVAL=10  # Sample every 10 seconds instead of 30
```

After any change to overlay files, rebuild and reprovision:

```bash
avocado build && avocado provision -f -r dev
```

All config changes happen on the host in the overlay directory — never edit files directly on the device.

## What's Next

This heartbeat service logs vitals locally. Natural next steps:
- Add configuration via `/etc/heartbeat.conf` (interval, log format)
- Extend the stats collected (disk usage, network traffic, temperature)
