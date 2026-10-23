#!/bin/sh
# Run only on the disposable Ubuntu CI VM after building the Debian package.
set -eu
package=$(find target/release/bundle/deb -name '*.deb' -print -quit)
[ -n "$package" ]
name=$(dpkg-deb --field "$package" Package)
sudo dpkg --install "$package"
/usr/bin/batterybridge config
systemctl is-active --quiet batterybridge.service
sudo dpkg --remove "$name"
if systemctl is-active --quiet batterybridge.service; then
    echo 'Background service survived removal' >&2
    exit 1
fi
[ ! -e /usr/bin/batterybridge-service ]
[ ! -e /usr/lib/systemd/system/batterybridge.service ]
