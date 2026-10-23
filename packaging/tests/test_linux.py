"""Test package lifecycle hooks with fake systemd and IPC, never the host daemon."""
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

HERE = Path(__file__).resolve().parents[1] / 'linux'


class LinuxPackageTests(unittest.TestCase):
    def run_hook(self, package, hook, argument, *, cli=0, stop=0, status=0):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            trace = root / 'trace'
            def command(name, body):
                path = root / name
                path.write_text(f'#!/bin/sh\necho "{name} $*" >> "{trace}"\n{body}\n')
                path.chmod(0o755)
                return str(path)
            replacements = {
                'systemctl': command('systemctl', f'case "$1" in stop) exit {stop};; show) echo {status};; esac'),
                '/usr/bin/batterybridge': command('client', f'exit {cli}'),
                'sleep 1': command('sleep', 'exit 0'),
                'getent': command('getent', 'exit 0'),
                '/run/systemd/system': str(root),
            }
            text = (HERE / f'{package}-{hook}.sh').read_text()
            for source, target in replacements.items():
                text = text.replace(source, target)
            script = root / 'hook'
            script.write_text(text)
            result = subprocess.run(['/bin/sh', str(script), argument], capture_output=True, text=True)
            return result, trace.read_text() if trace.exists() else ''

    def test_install_starts_and_checks_the_daemon(self):
        for package in ('deb', 'rpm'):
            result, trace = self.run_hook(package, 'postinstall', 'configure')
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertLess(trace.index('daemon-reload'), trace.index('restart'))
            self.assertLess(trace.index('restart'), trace.index('client config'))
            result, trace = self.run_hook(package, 'postinstall', 'configure', cli=1)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(trace.count('client config'), 10)

    def test_remove_releases_hardware_but_upgrade_keeps_service_registration(self):
        for package, remove, upgrade in [('deb', 'remove', 'upgrade'), ('rpm', '0', '1')]:
            result, trace = self.run_hook(package, 'preremove', remove)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertLess(trace.index('stop'), trace.index('disable'))
            result, trace = self.run_hook(package, 'preremove', upgrade)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertNotIn('disable', trace)
            for failure in ({'stop': 1}, {'status': 1}):
                result, trace = self.run_hook(package, 'preremove', remove, **failure)
                self.assertNotEqual(result.returncode, 0)
                self.assertNotIn('disable', trace)

    def test_package_contents_use_system_paths_and_administrator_groups(self):
        for package, group in [('deb', 'sudo'), ('rpm', 'wheel')]:
            config = json.loads((HERE / f'tauri.{package}.conf.json').read_text())['bundle']['linux'][package]
            self.assertIn('/usr/bin/batterybridge-service', config['files'])
            unit = (HERE / f'{package}.service').read_text()
            self.assertIn(f'Group={group}', unit)
            self.assertIn('ExecStart=/usr/bin/batterybridge-service', unit)
            self.assertIn('RuntimeDirectoryMode=0750', unit)
            self.assertIn('ProtectSystem=strict', unit)
