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

  json="{"
  json="${json}\"uptime\":${uptime_secs},"
  json="${json}\"mem_free_kb\":${mem_free_kb},"
  json="${json}\"mem_total_kb\":${mem_total_kb},"
  json="${json}\"load_1m\":\"${load_1m}\","
  json="${json}\"ts\":${ts}"
  json="${json}}"
  echo "$json"

  sleep "$INTERVAL"
done
