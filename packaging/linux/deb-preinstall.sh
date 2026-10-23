#!/bin/sh
set -eu
if [ -e /etc/systemd/system/batterybridge.service ]; then
    echo 'A manually installed BatteryBridge unit overrides this package. Remove the manual unit before installing the native package.' >&2
    exit 1
fi
getent group sudo >/dev/null || { echo "Local administrator group sudo is required." >&2; exit 1; }
test -d /run/systemd/system || { echo "BatteryBridge requires a running systemd system." >&2; exit 1; }
