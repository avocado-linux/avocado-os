# Avocado React.js Reference Application

A simple React.js application that displays live system statistics. This reference demonstrates how to build and deploy a Node.js/React application using the Avocado nativesdk.

## Features

- **Live System Stats**: CPU, Memory, Disk, Load Average, Uptime, Temperature
- **Network Monitoring**: Per-interface RX/TX statistics
- **Auto-refresh**: Stats update every 2 seconds
- **Responsive Design**: Works on various screen sizes
- **Dark Theme**: Modern dark UI with Tailwind CSS

## Architecture

- **Frontend**: React.js with Vite build tool and Tailwind CSS
- **Backend**: Express.js server serving static files and `/api/stats` endpoint
- **Data Sources**: Standard Linux proc filesystem (`/proc/stat`, `/proc/meminfo`, etc.)

## Development

To run locally for development:

```bash
# Install dependencies
npm install

# Start the API server (in one terminal)
npm run server

# Start the Vite dev server (in another terminal)
npm run dev
```

## Building

The application is built using the Avocado SDK with `nativesdk-nodejs`:

```bash
npm install
npm run build
```

This creates a `dist/` directory with the optimized React build.

## API Endpoints

- `GET /api/stats` - Returns JSON with current system statistics

## System Stats Sources

All stats are gathered from standard Linux sources that work across various targets:

| Stat | Source |
|------|--------|
| CPU | `/proc/stat` |
| Memory | `/proc/meminfo` |
| Load Average | `/proc/loadavg` |
| Uptime | `/proc/uptime` |
| Network | `/proc/net/dev` |
| Disk | `df` command |
| Temperature | `/sys/class/thermal/thermal_zone0/temp` |
