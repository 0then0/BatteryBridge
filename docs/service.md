# Service installation and recovery

Installation is deliberately manual and elevated. The GUI is always an ordinary application. Build first; no build/test command installs a service or writes battery controls. Install the CLI and daemon into directories ordinary users cannot replace.

## Linux / systemd

Run these commands deliberately as an administrator, from the repository root after `cargo build --release --locked`. Replace the username when granting access.

```sh
sudo groupadd --system batterybridge
sudo install -m 0755 target/release/batterybridge target/release/batterybridge-service /usr/local/bin/
sudo install -m 0644 packaging/systemd/batterybridge.service /etc/systemd/system/
sudo usermod -aG batterybridge YOUR_USER
sudo systemctl daemon-reload
sudo systemctl enable --now batterybridge
```

If the group already exists, skip group creation. Sign out and in for new group membership. The unit creates `/run/batterybridge` (root:batterybridge 0750) and `/var/lib/batterybridge` (0700). Socket: `/run/batterybridge/control.sock`; config: `/var/lib/batterybridge/config.toml`. The sysfs controls resolve into `/sys/devices`, so the preview unit grants access to that subtree. It does not grant a GUI filesystem write access. Inspect `journalctl -u batterybridge` when policy fails.

On resume, native thresholds are rechecked within 30 seconds. There is no Linux force-discharge path in the generic threshold adapter. Before removing the unit, run `batterybridge limit off` and check normal charging, then `sudo systemctl disable --now batterybridge`.

## macOS / launchd

Apple Silicon charge control uses root-only private SMC operations. The `admin` group can send validated commands over the socket. Installation is not part of the GUI bundle.

```sh
sudo install -d -o root -g wheel -m 0755 /usr/local/bin
sudo install -m 0755 target/release/batterybridge target/release/batterybridge-service /usr/local/bin/
sudo install -d -o root -g admin -m 0750 /var/run/batterybridge
sudo install -d -o root -g wheel -m 0700 '/Library/Application Support/BatteryBridge'
sudo install -d -o root -g wheel -m 0755 /usr/local/libexec
sudo install -m 0755 packaging/launchd/batterybridge-launcher /usr/local/libexec/
sudo install -m 0644 packaging/launchd/io.github.batterybridge.plist /Library/LaunchDaemons/
sudo launchctl bootstrap system /Library/LaunchDaemons/io.github.batterybridge.plist
```

The shipped administrator-owned launcher recreates `/var/run/batterybridge` after reboot, then uses `exec` to start the daemon. Its path is fixed in the plist. The launcher and daemon must remain owned by root and unwritable by ordinary users.

Socket: `/var/run/batterybridge/control.sock`; config: `/Library/Application Support/BatteryBridge/config.toml`. On sleep, the daemon restores adapter power and inhibits charging when a limit is active, then acknowledges IOKit. On wake it reapplies policy. Sleep/shutdown firmware resets may still override SMC keys. The daemon refuses to start when its IOKit power notification registration fails.

The plist uses launchd restart with throttling. A force kill may leave a gate latched until restart. `/var/log/batterybridge.log` receives stderr; normal logs are quiet, but administrators should include it in their local log rotation policy. Before uninstalling, run `batterybridge limit off`, verify normal charging, then `sudo launchctl bootout system /Library/LaunchDaemons/io.github.batterybridge.plist`.

## Windows / Task Scheduler

Use an elevated PowerShell to execute `packaging/windows/install-task.ps1` from an extracted release archive. It places the daemon in `C:\Program Files\BatteryBridge`, rejects reparse points or trees owned by other users and replaces complete ACLs to restrict writes to Administrators/SYSTEM, creates protected state under `C:\ProgramData\BatteryBridge`, and registers an elevated SYSTEM task at startup/logon. The installer and daemon honor the system `ProgramFiles` / `ProgramData` environment locations; `C:` paths here describe their usual defaults. The task has no execution-time limit, allows battery operation, prevents concurrent instances and retries failures. This is a Task Scheduler mechanism; the daemon does not pretend to implement the SCM service entrypoint.

Pipe: `\\.\pipe\BatteryBridge.v1`; config: `C:\ProgramData\BatteryBridge\config.toml`. Interactive local users may send validated commands; remote/network users are denied. Run `batterybridge status` as the ordinary user after installation. WMI provider/driver availability remains device-specific. The daemon reasserts experimental conservation mode within 30 seconds of resume; the known getter is not used as a false confirmation.

The installer locates the daemon two levels above its own directory, matching the staged release archive. When using source builds, stage that archive layout or copy the built executable alongside the intended archive root first.

To remove the task, first run `batterybridge limit off` and confirm normal charging, then invoke `uninstall-task.ps1` elevated. Task Scheduler termination is abrupt. Configuration replacement uses a synced sibling file and standard-library rename. If an interrupted write leaves `config.toml.new`, stop the task and inspect that file before removing it; the last committed `config.toml` remains authoritative. No ephemeral mode is restored from configuration.

## Recovery

Invalid configuration is a startup error, not a silent reset to defaults. Fix the TOML as administrator and restart the service. An unsupported saved policy remains visible as an enforcement error; no emulated hardware state is reported.

On Unix, an OS file lock allows only one daemon per endpoint. Startup reclaims a stale socket only under that lock and after a connection attempt reports that no listener exists; non-socket files and live listeners are never replaced. The lock is released by the kernel on process death, enabling service-manager crash recovery. Do not remove a lock file while a daemon is running. Reboot normally clears the runtime directory. For supervised development, `batterybridge-service --config PATH --socket PATH` selects disposable resources; it still accesses real hardware, so do not start it merely to test IPC. Unit tests use fake controllers and temporary files.
