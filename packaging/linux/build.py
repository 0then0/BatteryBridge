#!/usr/bin/env python3
"""Build native Debian and RPM desktop packages from existing release binaries."""
import argparse
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tempfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from app import REPO, VERSION, licenses

HERE = Path(__file__).resolve().parent
DEB_DEPENDS = ['systemd', 'libc6 (>= 2.39)', 'libgl1', 'libegl1', 'libxkbcommon0', 'libxkbcommon-x11-0',
               'libwayland-client0', 'libwayland-egl1', 'libx11-6', 'libxcursor1', 'libxi6', 'libxrandr2', 'libxcb1']
RPM_DEPENDS = ['systemd', '/usr/bin/getent', 'libGL.so.1()(64bit)', 'libEGL.so.1()(64bit)',
               'libxkbcommon.so.0()(64bit)', 'libxkbcommon-x11.so.0()(64bit)', 'libwayland-client.so.0()(64bit)', 'libwayland-egl.so.1()(64bit)', 'libX11.so.6()(64bit)',
               'libXcursor.so.1()(64bit)', 'libXi.so.6()(64bit)', 'libXrandr.so.2()(64bit)', 'libxcb.so.1()(64bit)']


def stage(root, build, package):
    binaries = root / 'usr/bin'
    binaries.mkdir(parents=True)
    for name in ('batterybridge', 'batterybridge-service', 'batterybridge-gui'):
        target = binaries / name
        shutil.copy2(build / name, target)
        target.chmod(0o755)
    unit = root / 'usr/lib/systemd/system/batterybridge.service'
    unit.parent.mkdir(parents=True)
    shutil.copy2(HERE / f'{package}.service', unit)
    unit.chmod(0o644)
    desktop = root / 'usr/share/applications/BatteryBridge.desktop'
    desktop.parent.mkdir(parents=True)
    desktop.write_text('[Desktop Entry]\nType=Application\nName=BatteryBridge\n'
                       'Comment=Device-aware battery charge control\nExec=batterybridge-gui\n'
                       'Icon=batterybridge-gui\nTerminal=false\nCategories=Utility;\nStartupWMClass=io.github.batterybridge\n')
    for size in (32, 128):
        icon = root / f'usr/share/icons/hicolor/{size}x{size}/apps/batterybridge-gui.png'
        icon.parent.mkdir(parents=True)
        shutil.copy2(REPO / f'apps/gui/icons/{size}x{size}.png', icon)
    licenses(root / 'usr/share/licenses/batterybridge')


def build_deb(build, output):
    with tempfile.TemporaryDirectory(prefix='batterybridge-deb-') as temporary:
        root = Path(temporary) / 'package'
        stage(root, build, 'deb')
        control = root / 'DEBIAN'
        control.mkdir()
        installed = (sum(path.stat().st_size for path in root.rglob('*') if path.is_file()) + 1023) // 1024
        (control / 'control').write_text(
            f'Package: battery-bridge\nVersion: {VERSION}\nArchitecture: amd64\n'
            f'Maintainer: BatteryBridge contributors <noreply@github.com>\nInstalled-Size: {installed}\n'
            f'Depends: {", ".join(DEB_DEPENDS)}\nRecommends: fonts-noto-cjk\n'
            'Section: utils\nPriority: optional\nHomepage: https://github.com/0then0/BatteryBridge\n'
            'Description: Device-aware battery charge control\n Native desktop app, CLI and charging policy service.\n')
        for source, target in [('preinstall', 'preinst'), ('postinstall', 'postinst'), ('preremove', 'prerm'), ('postremove', 'postrm')]:
            shutil.copy2(HERE / f'deb-{source}.sh', control / target)
            (control / target).chmod(0o755)
        subprocess.run(['dpkg-deb', '--root-owner-group', '--build', str(root), str(output)], check=True)


def build_rpm(build, output):
    with tempfile.TemporaryDirectory(prefix='batterybridge-rpm-') as temporary:
        work = Path(temporary)
        payload = work / 'payload'
        stage(payload, build, 'rpm')
        spec = [f'Name: BatteryBridge\nVersion: {VERSION}\nRelease: 1\nSummary: Device-aware battery charge control\n'
                'License: Apache-2.0 AND MIT AND OFL-1.1 AND BSD-3-Clause\nURL: https://github.com/0then0/BatteryBridge\n'
                'BuildArch: x86_64\nRequires: ' + ', '.join(RPM_DEPENDS) + '\nRecommends: google-noto-sans-cjk-jp-fonts\n'
                '%description\nNative desktop app, CLI and charging policy service.\n'
                f'%install\nmkdir -p "%{{buildroot}}"\ncp -a {shlex.quote(str(payload))}/. "%{{buildroot}}/"\n']
        for section, hook in [('pre', 'preinstall'), ('post', 'postinstall'), ('preun', 'preremove'), ('postun', 'postremove')]:
            spec.append(f'%{section}\n' + (HERE / f'rpm-{hook}.sh').read_text().replace('#!/bin/sh\n', '', 1))
        spec.append('%files\n%defattr(-,root,root,-)\n/usr/bin/batterybridge\n/usr/bin/batterybridge-gui\n'
                    '/usr/bin/batterybridge-service\n/usr/lib/systemd/system/batterybridge.service\n'
                    '/usr/share/applications/BatteryBridge.desktop\n/usr/share/icons/hicolor/*/apps/batterybridge-gui.png\n'
                    '/usr/share/licenses/batterybridge\n')
        path = work / 'BatteryBridge.spec'
        path.write_text('\n'.join(spec))
        subprocess.run(['rpmbuild', '-bb', '--define', f'_topdir {work}', str(path)], check=True)
        candidates = list((work / 'RPMS').rglob('*.rpm'))
        if len(candidates) != 1:
            raise RuntimeError('Expected one RPM output')
        shutil.copy2(candidates[0], output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('format', choices=('deb', 'rpm'))
    parser.add_argument('--build-dir', type=Path, default=REPO / 'target/release')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    output = (args.output or REPO / f'target/release/bundle/{args.format}/BatteryBridge-linux-x86_64.{args.format}').resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    (build_deb if args.format == 'deb' else build_rpm)(args.build_dir.resolve(), output)
    print(output)


if __name__ == '__main__':
    main()
