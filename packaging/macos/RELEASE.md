## BatteryBridge 0.2.0

### Native installation on each platform

- **macOS:** one **BatteryBridge-macos-universal.pkg** for Intel and Apple Silicon, macOS 11+. GUI, CLI and daemon all contain native arm64 and x86_64 code. Open the package in Finder and approve the administrator prompt. Use the app's uninstall action for complete removal.
- **Windows 10/11 x86_64:** **BatteryBridge-windows-x86_64-setup.exe** installs the GUI and a protected SYSTEM background task. No PowerShell setup is needed. Remove through Windows Settings; the uninstaller restores normal charging first.
- **Linux x86_64:** `.deb` and `.rpm` include the GUI and systemd service. Open the package in a compatible graphical package manager. Debian packages authorize existing `sudo` group members; RPM uses `wheel`. No users are added to groups. Requires systemd and compatible WebKitGTK 4.1 libraries. Package removal disables the daemon and retains configuration for reinstallation.

Updates preserve saved charging settings. The macOS app build is `4`; the Installer receipt is `0.2.0.4`. Install the new package over an earlier release. Developer archives remain available but do not install the background component themselves.

### Intel charge control

Intel Macs now probe compatible SMC interfaces. A readable, valid `BCLM` key enables experimental firmware-managed thresholds from 50% to 100%, with write readback and temporary full charge. BCLM does not provide an adjustable start threshold or forced discharge. Compatible binary gates remain an experimental fallback. Unsupported hardware retains battery monitoring and reports capabilities explicitly.

Compilation and automated tests on Intel are not validation of charging on every Intel Mac. Physical Intel charging, sleep and firmware behavior still require device-level testing. Apple Silicon binary gates can be reset by firmware during sleep; a closed-lid percentage cap is not guaranteed. Lenovo WMI charge control remains experimental and fixed at 80%.

### Distribution

Windows and macOS installers are unsigned; macOS is not notarized. Review origin and platform security prompts. Do not disable Gatekeeper or other system protections. Managed devices may prohibit unsigned installation.

`SHA256SUMS.txt` covers all downloadable assets. The README lists direct links for each installer and developer archive.
