# Install BatteryBridge

[Documentation index](README.md)

## macOS developer preview

The graphical installer supports Apple Silicon Macs running macOS 11 or newer. It installs both the desktop app and the charging service. No Terminal commands are required.

1. Download **BatteryBridge-macos-apple-silicon.pkg** from a [GitHub pre-release](https://github.com/0then0/BatteryBridge/releases).
2. Open it in Finder, follow Installer and enter an administrator password when prompted.
3. Open **Applications → BatteryBridge** from a local administrator account.
4. Choose a stop percentage and **Apply limit**. The saved limit should appear above the controls. If firmware refuses the operation, the app displays an error.

The preview is unsigned and not notarized. If macOS blocks it, review its origin and open **System Settings → Privacy & Security** after the blocked attempt. Choose **Open Anyway** if you trust this developer preview, then follow macOS prompts. Do not disable Gatekeeper. Organization-managed Macs may prohibit unsigned installers; distributing a signed and notarized package remains a release requirement for general users.

A new installation starts with the charge limit off. The service runs after the app closes and starts at boot. Updating closes an existing BatteryBridge app, restarts the service and preserves its settings. Open the installed copy in Applications after an update, rather than a previous development binary.

The GUI should no longer show **Read-only mode** for a local administrator. Standard accounts retain read-only access. Installing the service does not establish firmware compatibility: Apple Silicon SMC charging operations are experimental.

To restore normal charging, select **Turn off** in the app. Deleting the app alone leaves the background service installed. Administrative removal instructions are in the [service guide](service.md#remove-the-packaged-macos-service).

## Windows and Linux

The first preview provides developer archives on these platforms. Desktop bundles do not yet install their charging service. Follow [service installation](service.md) to enable control, or [build from source](build.md). Battery status and compatibility reports work without the service.

## Verify a download

The release includes `SHA256SUMS.txt` for all assets. These checksums detect download corruption; they do not replace Developer ID signing or notarization. Obtain the installer only from this repository's release page.
