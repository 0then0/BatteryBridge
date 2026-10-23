## BatteryBridge 0.1.1

This patch adds graphical macOS removal and corrects charging-policy transitions around sleep.

### Changes

- **Uninstall without Terminal:** choose **Uninstall BatteryBridge** at the bottom of the macOS app, confirm and approve the administrator dialog. The app restores normal charging, unloads the service, and removes the installed app, helpers, saved policy and log. Removal stops if charging restoration or service unloading fails. Moving the app to Trash alone still leaves the service installed.
- **Sleep and resume:** preparing for sleep no longer briefly releases the saved charging limit. Charging inhibition is retained after wake until the resume threshold is reached, instead of reopening the gate inside the hysteresis band.
- **Visible diagnostics:** errors preparing for sleep survive successful awake polling. The UI identifies a percentage as a saved setting and warns when telemetry shows charging above it outside an explicit top-up.
- New messages and removal controls are translated into English, Russian, Spanish, German, French, Japanese and Brazilian Portuguese.
- The README includes direct downloads and checksums for each published platform.

### Install or update

On an Apple Silicon Mac with macOS 11 or newer, download **BatteryBridge-macos-apple-silicon.pkg**, open it in Finder and follow Installer. Open **Applications → BatteryBridge** after installation. Updating closes the running app, restarts the service and keeps the saved charge policy. No Terminal commands are required.

CLI, GUI and service use version `0.1.1`. The macOS bundle build is `3`; the Installer receipt is `0.1.1.3`. These versions replace `0.1.0` and the earlier preview without removing saved settings.

Windows and Linux assets are developer archives with manual charging-service setup. `SHA256SUMS.txt` covers every downloadable asset.

### Hardware and distribution limitations

Apple Silicon SMC and Lenovo WMI charge control remain experimental. The sleep-transition regression is covered by automated tests, but this release has not been validated through a real closed-lid charging cycle. Private SMC gates can be reset by firmware during sleep: this patch does not establish a firmware-managed limit or guarantee a closed-lid percentage cap. Firmware may also refuse writes while awake. A saved setting alone is not proof of enforcement.

The macOS package is unsigned and not notarized. If macOS blocks it, review its origin and use **System Settings → Privacy & Security → Open Anyway** only if you trust this release. Do not disable Gatekeeper. Intel Mac charging control is unsupported. Only local administrator accounts can change charging settings.

To report a problem, include the OS version, device model and a reviewed compatibility report from the app. Reports exclude serial numbers and network identifiers.
