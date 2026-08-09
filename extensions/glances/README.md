# avocado-ext-glances

Glances as **the** Avocado device-telemetry collector. Ships the `glances` package
plus a user-editable config so operators choose exactly what to collect and where to
send it — no firmware change required.

## How it works (default: Milestone 2)

```
glances --export restful ──POST full JSON──► avocado-glances-bridge ──newline JSON──►
    /run/avocado-conn/publish.sock ──► avocado-conn ──MQTT event/{id}──► Peridio
```

- **glances** collects everything enabled in `/etc/glances/glances.conf` and POSTs it
  to the local bridge every `-t` seconds.
- **avocado-glances-bridge** (stdlib Python, `/usr/libexec/avocado-glances-bridge`)
  wraps each POST as `{"type":"metrics","source":"glances","stats":…}` and writes it to
  `avocado-conn`'s publish-ingest socket.
- **avocado-conn** publishes it on `event/{id}` over its single authenticated MQTT
  session. One cloud connection, per-device auth, cardinality control at the bridge.

## Files it installs

| Path | Purpose |
|---|---|
| `python3-glances` (+ `python3-requests`) | collector + RESTful exporter (from feed) |
| `/etc/glances/glances.conf` | **what** to collect (plugins/thresholds) + **where** (`[restful]`) |
| `/etc/glances/avocado-glances.env` | exporter + **transmit interval** (`-t`) |
| `/etc/glances/avocado-glances-bridge.env` | bridge host/port, conn socket, `INCLUDE_PLUGINS` |
| `avocado-glances.service` / `avocado-glances-bridge.service` | the two units |

Recipes live in `meta-avocado`:
`recipes-devtools/python/python3-{glances,shtab,pyinstrument}_*.bb`.

## Build

```bash
avocado-repo sysext install python3-glances -y   # glances + deps into the sysext sysroot
avocado ext build glances                         # -> glances.raw
```

## Test

**Default path (Milestone 2 — through avocado-conn):**
```bash
# Needs avocado-ext-connect (avocado-conn) running with its publish socket.
systemctl status avocado-glances-bridge avocado-glances
# Watch it arrive on the cloud side (event topic) with your MQTT client, or locally:
socat -u UNIX-LISTEN:/run/avocado-conn/publish.sock,fork -   # if testing the bridge alone
```

**Quick path (Milestone 1 — straight to a broker, testing only):**
```bash
# edit avocado-glances.env:  GLANCES_ARGS="-q --export mqtt -t 60"
# edit glances.conf: uncomment [mqtt], set host=<broker> tls=false
systemctl restart avocado-glances
docker run --rm --network host eclipse-mosquitto mosquitto_sub -t 'avocado/glances/#' -v
```

## Configure what you collect

Everything is in `/etc/glances/glances.conf` (plugins on by default; disable the ones
you don't want, set thresholds). High-cardinality plugins (`processlist`,
`connections`) are disabled by default. Interval is `-t` in `avocado-glances.env`.
`INCLUDE_PLUGINS` in the bridge env is an extra allowlist before egress.

See `avocado-cli/docs/features/observability-metrics-extension.md` (ENG-2477) for the
full design.
