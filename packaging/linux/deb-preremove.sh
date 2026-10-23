#!/bin/sh
set -eu
if [ "${1:-}" = remove ] || [ "${1:-}" = deconfigure ]; then
    if systemctl is-active --quiet batterybridge.service; then
        systemctl stop batterybridge.service
        # The daemon exits nonzero if it cannot restore normal charging.
        [ "$(systemctl show --property=ExecMainStatus --value batterybridge.service)" = 0 ] || {
            echo 'Normal charging could not be restored; removal stopped.' >&2
            exit 1
        }
    fi
    systemctl disable batterybridge.service
fi
