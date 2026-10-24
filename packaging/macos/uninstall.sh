#!/bin/sh
# Invoked by the desktop app through the macOS administrator authorization dialog.
set -eu

# Execute only trusted, installed helpers. Refuse linked or writable replacements.
for path in /Library/PrivilegedHelperTools /Library/LaunchDaemons \
    /Library/PrivilegedHelperTools/io.github.batterybridge.cli \
    /Library/PrivilegedHelperTools/io.github.batterybridge.service \
    /Library/PrivilegedHelperTools/io.github.batterybridge.launcher \
    /Library/LaunchDaemons/io.github.batterybridge.plist \
    '/Library/Application Support/BatteryBridge' /var/run/batterybridge; do
    if [ -L "$path" ]; then
        echo "Refusing symbolic link: $path" >&2
        exit 1
    fi
    if [ -e "$path" ]; then
        owner=$(/usr/bin/stat -f '%u' "$path")
        mode=$(/usr/bin/stat -f '%Lp' "$path")
        if [ "$owner" != 0 ] || [ "$((0$mode & 022))" -ne 0 ]; then
            echo "Refusing untrusted installation path: $path" >&2
            exit 1
        fi
    fi
done

# Restore charging before removing the only component that can control it.
# Failure leaves the installation intact for diagnosis instead of hiding a latched gate.
running=0
if /bin/launchctl print system/io.github.batterybridge >/dev/null 2>&1; then
    running=1
fi
if [ "$running" -eq 1 ] || [ -e /Library/PrivilegedHelperTools/io.github.batterybridge.service ]; then
    if [ "$running" -eq 0 ]; then
        /bin/launchctl bootstrap system /Library/LaunchDaemons/io.github.batterybridge.plist
        attempt=0
        until /Library/PrivilegedHelperTools/io.github.batterybridge.cli config >/dev/null 2>&1; do
            attempt=$((attempt + 1))
            [ "$attempt" -lt 10 ] || exit 1
            /bin/sleep 1
        done
    fi
    /Library/PrivilegedHelperTools/io.github.batterybridge.cli reset
    /bin/launchctl bootout system/io.github.batterybridge
fi

/bin/rm -f /Library/LaunchDaemons/io.github.batterybridge.plist \
    /Library/PrivilegedHelperTools/io.github.batterybridge.service \
    /Library/PrivilegedHelperTools/io.github.batterybridge.cli \
    /Library/PrivilegedHelperTools/io.github.batterybridge.launcher
/bin/rm -rf '/Library/Application Support/BatteryBridge' /var/run/batterybridge
# The log and app can be links: rm removes the link itself, not its target.
/bin/rm -f /var/log/batterybridge.log
/bin/rm -rf /Applications/BatteryBridge.app
if /usr/sbin/pkgutil --pkg-info io.github.batterybridge.installer >/dev/null 2>&1; then
    /usr/sbin/pkgutil --forget io.github.batterybridge.installer
fi
