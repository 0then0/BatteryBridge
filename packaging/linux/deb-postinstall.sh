#!/bin/sh
set -eu
systemctl daemon-reload
systemctl enable batterybridge.service
systemctl restart batterybridge.service
# Wait for IPC, not just a launch registration. Configuration has no offline fallback.
attempt=0
until /usr/bin/batterybridge config >/dev/null 2>&1; do
    attempt=$((attempt + 1))
    [ "$attempt" -lt 10 ] || { echo 'BatteryBridge did not start.' >&2; exit 1; }
    sleep 1
done
