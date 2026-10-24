## BatteryBridge 0.3.0

### Native Rust desktop app

The GUI now uses egui and eframe. Selected fastframe modules provide the tray,
window lifecycle, font fallback and system language detection; macOS also uses
its Dock visibility helper. System light/dark appearance, all seven languages,
read-only fallback, charging settings, compatibility reports and graphical macOS
uninstall remain available.

The app no longer requires Node.js, WebView2 or WebKitGTK. The former local glib
backport is removed with that dependency graph. The source uses released egui and
pins fastframe to v0.14.1. GUI builds require Rust 1.98+.

### Battery care and compatibility

This release adds temporary charge targets, hold/pause/resume controls, automatic
discharge, battery-temperature protection, calibration, calendar schedules,
MagSafe light settings and an idle-sleep assertion where hardware permits them.
Telemetry includes battery temperature, voltage, current, power and cycle count
when provided by the device. Shortcuts can invoke the documented CLI commands.

The IPC protocol changes from version 2 to 3. Upgrade the GUI, CLI and background
service together using the platform installer; a 0.2 client or service cannot
communicate with its 0.3 counterpart.

Configuration versions 0 and 1 migrate to version 2 while preserving the saved
limit and leaving new care features disabled. Version 2 is saved on the next
successful settings update. Back up configuration before upgrading: older
binaries cannot read version 2. For rollback, restore that backup along with the
older application and service. See the configuration guide for manual conversion.

### Installation

- **macOS 11+:** one universal `.pkg` includes Intel and Apple Silicon GUI, CLI and daemon. Bundle build 5; Installer receipt 0.3.0.5. Install over an existing package to preserve charging settings.
- **Windows 10/11 x86_64:** NSIS installs the GUI and protected SYSTEM task. Remove through Windows Settings. Normal charging is restored before removal.
- **Linux x86_64:** native `.deb` and `.rpm` include the app and systemd service, with OpenGL/EGL and X11/Wayland dependencies. Existing sudo/wheel members can control charging. Noto CJK fonts are recommended for Japanese.

Developer archives contain the native GUI executable and system integration
resources. macOS and Windows installers are unsigned; macOS is not notarized.
`SHA256SUMS.txt` covers every downloadable package and archive.

### Hardware boundaries

Charging capabilities remain hardware-dependent. Intel SMC and Apple Silicon
SMC control, and Lenovo WMI conservation mode, remain experimental. Temperature
protection and calibration require device-level validation. Before sleep, the
service requests inhibition for percentage targets below 100%; Apple Silicon
firmware may reset the charging gate while asleep, so a closed-lid percentage
limit is not guaranteed. The idle assertion does not prevent lid-close or manual
sleep.
