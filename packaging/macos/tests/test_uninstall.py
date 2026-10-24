"""Run the graphical uninstaller against disposable paths, never the host service."""
import subprocess
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parents[1]


class UninstallTests(unittest.TestCase):
    def run_uninstall(self, root, *, limit=0, bootout=0, loaded=True, owner=0, link=False, mode="755", config=0, bootstrap=0):
        trace = root / "trace"

        def command(name, body):
            path = root / name
            path.write_text(f'#!/bin/sh\necho "{name} $*" >> "{trace}"\n{body}\n')
            path.chmod(0o755)
            return str(path)

        helpers = root / "helpers"
        helpers.mkdir()
        service = helpers / "io.github.batterybridge.service"
        service.touch()
        client = helpers / "io.github.batterybridge.cli"
        client.write_text(f'#!/bin/sh\necho "client $*" >> "{trace}"\ncase "$1" in reset) exit {limit};; config) exit {config};; esac\n')
        client.chmod(0o755)
        if link:
            service.unlink()
            service.symlink_to(client)
        daemon_dir = root / "daemons"
        daemon_dir.mkdir()
        (daemon_dir / "io.github.batterybridge.plist").touch()
        app = root / "BatteryBridge.app"
        app.mkdir()
        state = root / "state"
        state.mkdir()
        (state / "config.toml").touch()
        text = (HERE / "uninstall.sh").read_text()
        replacements = {
            "/bin/launchctl": command("launchctl", f'case "$1" in print) exit {0 if loaded else 1};; bootout) exit {bootout};; bootstrap) exit {bootstrap};; esac'),
            "/usr/bin/stat": command("stat", f'case "$2" in %u) echo {owner};; *) echo {mode};; esac'),
            "/bin/sleep": command("sleep", "exit 0"),
            "/usr/sbin/pkgutil": command("pkgutil", "exit 0"),
            "/bin/rm": command("rm", '/bin/rm "$@"'),
            "/Library/PrivilegedHelperTools": str(helpers),
            "/Library/LaunchDaemons": str(daemon_dir),
            "/Library/Application Support/BatteryBridge": str(state),
            "/var/run/batterybridge": str(root / "runtime"),
            "/Applications/BatteryBridge.app": str(app),
            "/var/log/batterybridge.log": str(root / "log"),
            "/var/log/batterybridge.err": str(root / "err"),
        }
        for source, target in replacements.items():
            text = text.replace(source, target)
        assert "/Library/" not in text and "/Applications/" not in text and "/var/" not in text.replace(str(root), "")
        script = root / "uninstall"
        script.write_text(text)
        result = subprocess.run(["/bin/sh", str(script)], capture_output=True, text=True)
        return result, trace.read_text() if trace.exists() else "", app, state

    def test_restores_charging_then_unloads_and_removes_all_installed_components(self):
        for loaded in (True, False):
            with self.subTest(loaded=loaded), tempfile.TemporaryDirectory() as directory:
                result, trace, app, state = self.run_uninstall(Path(directory), loaded=loaded)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertLess(trace.index("client reset"), trace.index("launchctl bootout"))
                self.assertLess(trace.index("launchctl bootout"), trace.index("rm -f"))
                self.assertFalse(app.exists())
                self.assertFalse(state.exists())
                self.assertFalse((Path(directory) / "helpers/io.github.batterybridge.service").exists())
                self.assertIn("pkgutil --forget", trace)
                if not loaded:
                    self.assertLess(trace.index("launchctl bootstrap"), trace.index("client reset"))

    def test_restoration_or_unload_failure_preserves_installation(self):
        for failure in ({"limit": 1}, {"bootout": 1}, {"loaded": False, "config": 1}, {"loaded": False, "bootstrap": 1}):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as directory:
                result, trace, app, state = self.run_uninstall(Path(directory), **failure)
                self.assertNotEqual(result.returncode, 0)
                self.assertNotIn("rm -", trace)
                self.assertTrue(app.exists())
                self.assertTrue(state.exists())

    def test_rejects_untrusted_or_linked_helpers_before_execution(self):
        for invalid in ({"owner": 501}, {"link": True}, {"mode": "777"}):
            with self.subTest(invalid=invalid), tempfile.TemporaryDirectory() as directory:
                result, trace, app, _ = self.run_uninstall(Path(directory), **invalid)
                self.assertNotEqual(result.returncode, 0)
                self.assertNotIn("client reset", trace)
                self.assertNotIn("rm -", trace)
                self.assertTrue(app.exists())
