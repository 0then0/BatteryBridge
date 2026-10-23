## First developer preview

### Install on an Apple Silicon Mac

1. Download **BatteryBridge-macos-apple-silicon.pkg** below. The archives are intended for developers and manual installations.
2. Open the package in Finder and follow Installer. Enter an administrator password when macOS asks.
3. Open **Applications → BatteryBridge** using an administrator account. The installer includes and starts the charging service, so no Terminal commands are needed.
4. Set a charge limit and choose **Apply limit**. Check that the app shows the saved limit and no policy error. The service starts after reboot and keeps running when the app closes.

**Unsigned preview:** this package is not Developer ID signed or notarized. macOS may block it. After attempting to open it, review the package origin and use **System Settings → Privacy & Security → Open Anyway** if you choose to trust this preview. Do not disable Gatekeeper. Managed Macs may prohibit unsigned packages entirely. A signed, notarized installer is required for a frictionless public release.

### What is included

- System light/dark appearance, battery status, charging settings and compatibility reports.
- The app in `/Applications/BatteryBridge.app` and a root-owned launchd service.
- Private CLI, service and launcher executables in `/Library/PrivilegedHelperTools`.
- Machine-wide settings preserved across updates. A new installation starts with the charge limit off. Updating closes the previous app and restarts the service.

### Limits of this preview

- The graphical installer targets Apple Silicon Macs, macOS 11 or newer. Intel Mac charging control is unsupported.
- Apple Silicon SMC charge control is experimental. Software checks do not establish that your firmware accepts a hardware write. A hardware refusal must be reported as an error in the app.
- Only local administrator accounts can change charging settings. Other accounts can see read-only status.
- Windows and Linux archives contain native GUI builds, CLI and service definitions, but service installation on those platforms is still manual. Their GUI installers do not install the charging service.
- There is no automatic updater. Install a newer package to update. Use **Turn off** to restore normal charging before removing the app; dragging the app to Trash does not uninstall the service. Administrative removal is documented in `docs/service.md`.

### Please check

Installation without Terminal, absence of “Read-only mode” under an administrator account, applying an 80% limit, visible firmware errors, **Turn off**, **Top up**, system theme changes, and reconnecting after reboot. If you already run BatteryBridge manually, quit that copy before testing the installed app.

Include your macOS version, Mac model and a reviewed compatibility report from the app when reporting an issue. Do not include serial numbers or other private identifiers. SHA-256 checksums for the downloads are in `SHA256SUMS.txt`.
