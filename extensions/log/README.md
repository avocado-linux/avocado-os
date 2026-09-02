# avocado-ext-log

Ships device logs to an MQTT broker. journald is the collector (already running
on Avocado OS); this extension only (1) persists the journal to disk with a
rolling window and (2) streams it to a broker.

## What it does

- **Persist** — `10-avocado-log.conf` flips journald to `Storage=persistent` on
  `/var` with a 256M / 7d rolling window. Boot logs survive; nothing is RAM-only.
- **Stream** — `avocado-log.service` runs `journalctl <filters> -o json` piped to
  `mosquitto_pub -l`. One JSON log entry per MQTT message.

## Configure

`/etc/avocado-log/avocado-log.env`:

- `LOG_BROKER` / `LOG_PORT` — your MQTT broker (default `127.0.0.1:1883`).
- `LOG_TOPIC` — default `avocado/logs/{host}` (`{host}` → hostname).
- `JOURNAL_ARGS` — journalctl filter flags. Default `-b -n all -f -o json`
  (whole current boot, then follow). Narrow with `-p err`, `-u foo.service`,
  `--grep RE`. This is the WHAT-to-ship knob.

Then: `systemctl restart avocado-log`.

Watch on the broker: `mosquitto_sub -t 'avocado/logs/#' -v`.

## Milestone 1 scope (and its ceilings)

This is the direct-broker path, matching glances M1:

- **No TLS, no redaction.** Logs carry secrets/PII. Point this only at a broker
  you trust on a trusted network. Redaction-before-egress and TLS are the job of
  `avocado-logd` (Phase 2, see the logging RFC).
- **No backpressure / cursor.** On broker drop the pipe restarts and `-b -n all`
  replays the boot (duplicate lines). No rate limit; a chatty device can flood.
- **No backend gating.** Streaming is always-on while the service runs, not
  leased/capped by a backend. That gating is also `avocado-logd`.

Upgrade path when any ceiling bites: `avocado-logd` — a Rust sysext that taps
journald, applies filters + redaction + rate-limit, tracks a journal cursor, and
rides `avocado-conn`'s single authenticated MQTT session instead of a second
connection. Design: `avocado-cli/docs/features/logging-extension.md` (ENG-2476).
