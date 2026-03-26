#!/usr/bin/env python3

import sys
sys.path.insert(0, "/usr/lib/app/packages")

import json
import time
import os
import requests
import paho.mqtt.client as mqtt

BROKER = "broker.emqx.io"
PORT = 1883
MQTT_INTERVAL = 10
HTTP_INTERVAL = 45
HTTP_ENDPOINT = "https://httpbin.org/get"
DEVICE_ID = os.uname().nodename
TOPIC = f"avocado/{DEVICE_ID}/telemetry"

def read_uptime():
    with open("/proc/uptime") as f:
        return int(float(f.read().split()[0]))

def read_memory():
    info = {}
    with open("/proc/meminfo") as f:
        for line in f:
            if line.startswith(("MemTotal:", "MemFree:", "MemAvailable:")):
                parts = line.split()
                info[parts[0].rstrip(":")] = int(parts[1])
    return info

def read_load():
    with open("/proc/loadavg") as f:
        parts = f.read().split()
        return {"1m": float(parts[0]), "5m": float(parts[1]), "15m": float(parts[2])}

def read_temperature():
    try:
        for zone in sorted(os.listdir("/sys/class/thermal")):
            temp_path = f"/sys/class/thermal/{zone}/temp"
            if os.path.exists(temp_path):
                with open(temp_path) as f:
                    return int(f.read().strip()) / 1000.0
    except (OSError, ValueError):
        pass
    return None

def read_disk_usage():
    try:
        st = os.statvfs("/var")
        total = st.f_blocks * st.f_frsize
        free = st.f_bfree * st.f_frsize
        return {"total_mb": total // (1024 * 1024), "free_mb": free // (1024 * 1024)}
    except OSError:
        return None

def read_net_stats():
    try:
        with open("/proc/net/dev") as f:
            for line in f:
                parts = line.split()
                iface = parts[0].rstrip(":")
                if iface in ("eth0", "enp0s1", "end0"):
                    return {"interface": iface, "rx_bytes": int(parts[1]), "tx_bytes": int(parts[9])}
    except (OSError, IndexError, ValueError):
        pass
    return None

def read_process_count():
    try:
        return len([d for d in os.listdir("/proc") if d.isdigit()])
    except OSError:
        return None

def collect_telemetry():
    mem = read_memory()
    load = read_load()
    temp = read_temperature()
    disk = read_disk_usage()
    net = read_net_stats()
    procs = read_process_count()

    payload = {
        "device": DEVICE_ID,
        "kernel": os.uname().release,
        "timestamp": int(time.time()),
        "uptime_secs": read_uptime(),
        "mem_total_kb": mem.get("MemTotal", 0),
        "mem_free_kb": mem.get("MemFree", 0),
        "mem_available_kb": mem.get("MemAvailable", 0),
        "load": load,
    }

    if temp is not None:
        payload["cpu_temp_c"] = temp
    if disk is not None:
        payload["disk_var"] = disk
    if net is not None:
        payload["net"] = net
    if procs is not None:
        payload["process_count"] = procs

    return payload

def http_check():
    try:
        response = requests.get(HTTP_ENDPOINT, timeout=10)
        print(f"[http] status={response.status_code} elapsed={response.elapsed.total_seconds():.2f}s", flush=True)
    except requests.RequestException as e:
        print(f"[http] error: {e}", flush=True)

def on_connect(client, userdata, flags, reason_code, properties):
    print(f"Connected to {BROKER}:{PORT} (rc={reason_code})", flush=True)

def on_disconnect(client, userdata, flags, reason_code, properties):
    print(f"Disconnected (rc={reason_code}), will reconnect...", flush=True)

def main():
    print(f"app starting", flush=True)
    print(f"  mqtt: {BROKER}:{PORT}, topic={TOPIC}, interval={MQTT_INTERVAL}s", flush=True)
    print(f"  http: {HTTP_ENDPOINT}, interval={HTTP_INTERVAL}s", flush=True)

    client = mqtt.Client(mqtt.CallbackAPIVersion.VERSION2, client_id=f"avocado-{DEVICE_ID}")
    client.on_connect = on_connect
    client.on_disconnect = on_disconnect

    client.connect_async(BROKER, PORT, keepalive=60)
    client.loop_start()

    last_http = 0

    while True:
        now = time.time()

        # MQTT telemetry
        telemetry = collect_telemetry()
        payload = json.dumps(telemetry)
        result = client.publish(TOPIC, payload, qos=1)
        if result.rc == mqtt.MQTT_ERR_SUCCESS:
            print(f"[mqtt] published: {payload}", flush=True)
        else:
            print(f"[mqtt] publish failed: rc={result.rc}", flush=True)

        # HTTP check on its own interval
        if now - last_http >= HTTP_INTERVAL:
            http_check()
            last_http = now

        time.sleep(MQTT_INTERVAL)

if __name__ == "__main__":
    main()
