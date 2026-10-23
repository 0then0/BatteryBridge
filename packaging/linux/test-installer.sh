#!/bin/sh
# Run only on the disposable Ubuntu CI VM after building the Debian package.
set -eu
package=$(find target/release/bundle/deb -name '*.deb' -print -quit)
[ -n "$package" ]
name=$(dpkg-deb --field "$package" Package)
depends=$(dpkg-deb --field "$package" Depends)
for required in libegl1 libgl1 libxkbcommon0; do
    case "$depends" in *"$required"*) ;; *) echo "Missing GUI dependency: $required" >&2; exit 1;; esac
done
case "$depends" in *webkit*|*libgtk*) echo 'Unexpected WebView dependency' >&2; exit 1;; esac
sudo dpkg --install "$package"
# Hosted runners can receive sudo through sudoers without membership in the
# distribution's administrator group. Exercise the package's actual group policy.
sudo useradd --system --no-create-home --groups sudo bb-test-admin
sudo useradd --system --no-create-home bb-test-viewer
trap 'sudo userdel bb-test-admin; sudo userdel bb-test-viewer' EXIT
sudo stat -c '%U:%G %a %n' /run/batterybridge /run/batterybridge/control.sock
[ "$(sudo stat -c '%G' /run/batterybridge/control.sock)" = sudo ]
[ "$(sudo stat -c '%a' /run/batterybridge/control.sock)" = 660 ]
sudo -u bb-test-admin /usr/bin/batterybridge config
if sudo -u bb-test-viewer /usr/bin/batterybridge config; then
    echo 'Non-administrator unexpectedly accessed the charging service' >&2
    exit 1
fi
systemctl is-active --quiet batterybridge.service
sudo dpkg --remove "$name"
if systemctl is-active --quiet batterybridge.service; then
    echo 'Background service survived removal' >&2
    exit 1
fi
[ ! -e /usr/bin/batterybridge-service ]
[ ! -e /usr/lib/systemd/system/batterybridge.service ]
