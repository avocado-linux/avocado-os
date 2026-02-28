# React Reference Runtime (Experimental)

A reference runtime that demonstrates how to build and deploy a React.js system monitoring dashboard on Avocado OS. The app is a live dashboard — an Express server reads from `/proc` and serves a React frontend with real-time CPU, memory, disk, network, and process stats.

This showcases a full-stack web application on embedded Linux: React + Vite frontend compiled in the SDK, Express.js backend reading Linux system stats, served by Node.js and managed by systemd.

## Prerequisites

- Docker Desktop running
- `avocado` CLI installed

## Build and Run

```bash
cd os/references/react-experimental

# Install SDK (includes Node.js toolchain), extensions, and runtime
avocado install -f -t qemux86-64

# Build the React app (compiles via npm inside the SDK container) and assemble the image
avocado build -t qemux86-64

# Provision the bootable disk image
avocado provision -f -r dev -t qemux86-64

# Boot QEMU with port forwarding (SSH on 2222, dashboard on 4000)
avocado sdk run -iE -t qemux86-64 vm dev --host-fwd "2222-:22" --host-fwd "4000-:4000"
```

To SSH in from another terminal:

```bash
ssh -o StrictHostKeyChecking=no -p 2222 root@localhost
```

> Boot takes ~70 seconds on macOS (no KVM acceleration). Login as `root` with an empty password.

## View the Dashboard

Open `http://localhost:4000` in your browser. The dashboard shows:

- CPU usage with per-core breakdown and history chart
- Memory usage with cached/total breakdown
- Disk usage
- Load average (1/5/15 min)
- System uptime
- Temperature (if sensor available)
- Network interface stats (RX/TX)
- Top processes by memory usage

## Observe the Service

```bash
# Check the service is running
systemctl status ref-reactjs

# Watch server logs
journalctl -u ref-reactjs -f

# Stop and start the service
systemctl stop ref-reactjs
systemctl start ref-reactjs
```

## Make a Change and Redeploy

All edits happen on the host — never on the device.

1. Edit files in `ref-reactjs/src/` on your host machine
2. Rebuild and reprovision:

```bash
avocado build -t qemux86-64 -e example-reactjs && avocado provision -f -r dev -t qemux86-64
```

3. Boot the new image and open `http://localhost:4000` to see the changes

## Project Structure

```
react-experimental/
├── README.md
├── avocado.yaml              # Runtime, extension, and SDK config
├── reactjs-compile.sh        # Builds React app via npm inside SDK container
├── reactjs-install.sh        # Installs dist/, node_modules/, server.js to extension sysroot
├── reactjs-clean.sh          # Cleans build artifacts
├── overlay/
│   └── usr/lib/systemd/system/
│       └── ref-reactjs.service   # Systemd unit file
└── ref-reactjs/              # React + Express source code (compiled in SDK)
    ├── package.json
    ├── server.js             # Express backend (reads /proc, serves API + static files)
    ├── vite.config.js
    ├── tailwind.config.js
    ├── index.html
    ├── public/
    │   └── avocado.svg
    └── src/
        ├── App.jsx           # Main dashboard component
        ├── main.jsx
        ├── index.css
        ├── components/       # StatCard, ProgressBar, MiniChart, NetworkCard, ProcessList, Tabs
        └── hooks/            # useLocalStorage
```

## How It Works

### Build Pipeline

1. **`avocado build`** invokes `reactjs-compile.sh` inside the SDK Docker container
2. The SDK has Node.js and npm: `nativesdk-nodejs`, `nativesdk-nodejs-npm`
3. The compile script runs `npm install` then `npm run build` (Vite produces `dist/`)
4. Build verification checks that `dist/` and `dist/index.html` exist
5. `reactjs-install.sh` copies `dist/`, `node_modules/`, `package.json`, and `server.js` to the extension sysroot at `/usr/lib/ref-reactjs/`
6. The extension image is assembled with the app files and the systemd service unit

### API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/stats` | GET | CPU, memory, disk, load, uptime, network, temperature |
| `/api/processes` | GET | Top processes by memory (configurable limit) |
| `/*` | GET | Static files from `dist/` with SPA fallback |

All stats are read directly from `/proc` and `/sys` — no external dependencies on the device.

### Extension Config (in avocado.yaml)

| Field | Purpose |
|-------|---------|
| `overlay: overlay` | Systemd service unit merged into the root filesystem |
| `enable_services` | Enables `ref-reactjs.service` at boot |
| `on_merge` / `on_unmerge` | Restarts/stops the service on extension apply/remove |
| `packages.example-reactjs-app` | Declares the compiled package with compile and install scripts |
| `packages.nodejs` | Node.js runtime installed on the device |
| `sdk.packages` | Node.js and npm installed in the SDK container for building |

## What's Next

This reference produces a working web dashboard. Natural next steps:
- Add WebSocket for push-based updates instead of polling
- Add MQTT publishing to send vitals to Avocado Connect
- Add device configuration UI (hostname, network, services)
