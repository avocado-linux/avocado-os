#!/bin/sh
set -eu

# Source configuration
INTERVAL=30
[ -f /etc/heartbeat.conf ] && . /etc/heartbeat.conf

while true; do
  uptime_secs=$(awk '{print int($1)}' /proc/uptime)
  mem_free_kb=$(awk '/MemFree:/ {print $2}' /proc/meminfo)
  mem_total_kb=$(awk '/MemTotal:/ {print $2}' /proc/meminfo)
  load_1m=$(awk '{print $1}' /proc/loadavg)
  ts=$(date +%s)

  printf '{"uptime":%d,"mem_free_kb":%d,"mem_total_kb":%d,"load_1m":"%s","ts":%d}\n' \
    "$uptime_secs" "$mem_free_kb" "$mem_total_kb" "$load_1m" "$ts"

  sleep "$INTERVAL"
done
