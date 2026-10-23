#!/bin/sh
set -eu
getent group wheel >/dev/null || { echo "Local administrator group wheel is required." >&2; exit 1; }
test -d /run/systemd/system || { echo "BatteryBridge requires a running systemd system." >&2; exit 1; }
