# Python Telemetry App Reference Runtime

A reference runtime that demonstrates how to build a Python application with pip dependencies as an Avocado OS extension. The app collects device telemetry (uptime, memory, CPU load, temperature) and publishes it over MQTT to a public broker where you can view it in real time.

This shows how to bundle pip packages (`requests`, `paho-mqtt`) into an extension when they aren't available as RPMs in the Avocado package repository.


## Build and Run

```bash

# Install SDK, extensions, and runtime dependencies
avocado install -f

# Build extensions (compiles pip packages in SDK container)
avocado build

# Provision the runtime (create the bootable disk image)
avocado provision -r dev

# Boot QEMU interactively inside the SDK container
avocado sdk run -iE vm dev

```


## Observe Telemetry

Login as `root` with an empty password. The app service starts automatically on boot.

```bash
# Check the service is running
systemctl status app

# Watch telemetry logs in real time
journalctl -u app -f
```

You'll see output like:

```
app starting
  mqtt: broker.emqx.io:1883, topic=avocado/avocado-qemuarm64/telemetry, interval=10s
  http: https://httpbin.org/get, interval=45s
Connected to broker.emqx.io:1883 (rc=Success)
[mqtt] published: {"device": "avocado-qemuarm64", "timestamp": 1711234567, "uptime_secs": 42, ...}
[http] status=200 elapsed=0.45s
[mqtt] published: {"device": "avocado-qemuarm64", "timestamp": 1711234577, "uptime_secs": 52, ...}
```

The app runs two tasks on different intervals:
- **MQTT telemetry** every 10 seconds — publishes device stats to the broker
- **HTTP health check** every 45 seconds — makes a GET request to httpbin.org and logs the result

### View Messages Online

The app publishes to the free public EMQX broker. To view your messages:

1. Go to [mqttx.app](https://mqttx.app/) and open the web client, or use the MQTTX desktop app
2. Connect to `broker.emqx.io` on port `1883`
3. Subscribe to `avocado/+/telemetry` (all devices) or `avocado/avocado-qemuarm64/telemetry` (this device)
4. You'll see JSON telemetry messages arriving every 10 seconds

### Telemetry Payload

```json
{
  "device": "avocado-qemuarm64",
  "kernel": "6.12.69-avocado",
  "timestamp": 1711234567,
  "uptime_secs": 3642,
  "mem_total_kb": 524288,
  "mem_free_kb": 412356,
  "mem_available_kb": 468000,
  "load": {"1m": 0.03, "5m": 0.01, "15m": 0.0},
  "disk_var": {"total_mb": 2048, "free_mb": 1856},
  "net": {"interface": "eth0", "rx_bytes": 123456, "tx_bytes": 78901},
  "process_count": 47
}
```

Optional fields included when available on the target:
- `cpu_temp_c` — CPU temperature (physical hardware only, absent on QEMU)
- `disk_var` — `/var` partition usage
- `net` — primary network interface byte counters
- `process_count` — number of running processes

To shut down the VM:

```bash
poweroff
```

## Project Structure

```
qemuarm64/
├── avocado.yaml                        # Runtime config
├── build.sh                            # Convenience build script
├── app-compile.sh                      # SDK compile step: uv pip install requests paho-mqtt
├── app-install.sh                      # Install step: copy packages to sysroot
├── app-clean.sh                        # Clean pip build artifacts
└── app/
    ├── overlay/                        # Files merged into the root filesystem
    │   └── usr/
    │       ├── local/bin/
    │       │   └── app.py              # The Python application
    │       └── lib/systemd/system/
    │           └── app.service         # Systemd unit file
    └── packages/                   # Build artifact (created by app-compile.sh)
        ├── requests/
        ├── paho/                       # paho-mqtt client
        ├── urllib3/
        ├── certifi/
        ├── charset_normalizer/
        └── idna/
```

## How It Works

### The pip dependency problem

Python's `requests` and `paho-mqtt` libraries are not available as RPMs in the Avocado package repository.  

### The compile/install pattern

Avocado solves this the same way the React, Rust, Java, and Elixir references handle non-RPM dependencies: a **compile/install script pipeline** that runs inside the SDK container.

| Script | When it runs | What it does |
|--------|-------------|-------------|
| `app-compile.sh` | `avocado build` | Runs `uv pip install --target app/packages requests paho-mqtt` inside the SDK container |
| `app-install.sh` | `avocado build` | Copies `app/packages/*` into the extension sysroot at `/usr/lib/app/packages/` |
| `app-clean.sh` | `avocado clean` | Removes `app/packages/` |

The compile step requires `uv` in the SDK container. This is provided by the `nativesdk-uv` SDK package declared under the extension's `sdk.packages`.

### Python path setup

Since the pip packages are installed to a custom path (`/usr/lib/app/packages/`) rather than the system site-packages, the app adds this path before importing:

```python
import sys
sys.path.insert(0, "/usr/lib/app/packages")

import paho.mqtt.client as mqtt  # now found
import requests                   # now found
```

### MQTT topic structure

Messages are published to `avocado/{device_hostname}/telemetry`. The device hostname is read from `os.uname().nodename` at startup (e.g., `avocado-qemuarm64`). This makes it easy to subscribe to all devices with `avocado/+/telemetry`.

### Extension configuration in avocado.yaml

The `app` extension declares:

| Field | Purpose |
|-------|---------|
| `types: [sysext, confext]` | Built as a systemd system extension and config extension |
| `overlay: app/overlay` | Files to merge into the root filesystem (the .py script and .service file) |
| `packages.app.compile` | Links to the SDK compile section that runs `app-compile.sh` |
| `packages.app.install` | Runs `app-install.sh` to copy built artifacts into the sysroot |
| `packages.python3` | Installs Python 3 from the Avocado RPM repo |
| `packages.curl` | Installs curl for manual connectivity testing |
| `sdk.packages.nativesdk-uv` | Installs uv into the SDK container so `app-compile.sh` can run |
| `enable_services` | Enables `app.service` at boot |
| `on_merge` / `on_unmerge` | Restarts/stops the service on extension refresh |

## Customization

Edit `app/overlay/usr/local/bin/app.py` to change the broker, topic, or interval:

```python
BROKER = "broker.emqx.io"       # or your own broker
PORT = 1883
MQTT_INTERVAL = 10               # publish telemetry every 10 seconds
HTTP_INTERVAL = 45               # HTTP check every 45 seconds
HTTP_ENDPOINT = "https://httpbin.org/get"  # or your API endpoint
```

To add more pip dependencies, edit `app-compile.sh`:

```bash
uv pip install --target app/packages requests paho-mqtt psutil
```

After any change, rebuild and reprovision:

```bash
avocado build && avocado provision -r dev
```


