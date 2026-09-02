#!/usr/bin/env python3
"""Tap Avocado test telemetry off MQTT: pretty-print both extensions' streams.

  avocado-ext-glances -> metrics   (topic/value)
  avocado-ext-log     -> log lines  (parsed journald JSON)

Testing helper only. Needs paho-mqtt (`pip install paho-mqtt`).

  ./mqtt-tap.py --broker 10.0.2.2            # both, default topic avocado/#
  ./mqtt-tap.py --topic 'avocado/logs/#'     # just logs
"""
import argparse, json, datetime as dt
import paho.mqtt.client as mqtt

PRIO = {"0":"EMERG","1":"ALERT","2":"CRIT","3":"ERR","4":"WARN","5":"NOTICE","6":"INFO","7":"DEBUG"}


def on_connect(c, u, flags, rc, props=None):
    c.subscribe(u["topic"])
    print(f"connected, subscribed {u['topic']}")


def on_message(c, u, m):
    if "/logs/" in m.topic:
        try:
            j = json.loads(m.payload)
        except ValueError:
            print("LOG", m.payload.decode("utf-8", "replace")); return
        msg = j.get("MESSAGE", "")
        if isinstance(msg, list):  # journald byte-array for non-utf8
            msg = bytes(msg).decode("utf-8", "replace")
        ts = j.get("__REALTIME_TIMESTAMP")
        clk = dt.datetime.fromtimestamp(int(ts) / 1e6).strftime("%H:%M:%S") if ts else "--:--:--"
        pr = PRIO.get(str(j.get("PRIORITY", "6")), "?")
        ident = j.get("SYSLOG_IDENTIFIER") or j.get("_COMM") or "?"
        print(f"{clk} {pr:6} {ident}: {msg}")
    else:  # glances metric: topic value
        print(f"{m.topic}  {m.payload.decode('utf-8', 'replace')}")


ap = argparse.ArgumentParser()
ap.add_argument("--broker", default="127.0.0.1")
ap.add_argument("--port", type=int, default=1883)
ap.add_argument("--topic", default="avocado/#")
ap.add_argument("--user"); ap.add_argument("--password")
a = ap.parse_args()

c = mqtt.Client(mqtt.CallbackAPIVersion.VERSION2, userdata={"topic": a.topic})
if a.user:
    c.username_pw_set(a.user, a.password)
c.on_connect, c.on_message = on_connect, on_message
c.connect(a.broker, a.port, 60)
print(f"tapping mqtt://{a.broker}:{a.port}  (Ctrl-C to stop)")
c.loop_forever()
