<p align="center"><img src="assets/icon.svg" alt="BatteryBridge icon" width="128" height="128"></p>

# BatteryBridge

[![CI](https://img.shields.io/github/actions/workflow/status/0then0/BatteryBridge/ci.yml?label=CI)](https://github.com/0then0/BatteryBridge/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.90%2B-934b32)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/status-experimental-99743b)](docs/hardware.md)

BatteryBridge is a battery charge-control tool for Linux, macOS and Windows. It provides a standalone CLI, a background service and a Tauri tray app, with shared charging policies implemented in Rust.

Available controls depend on your laptop's kernel driver, firmware and vendor interfaces. BatteryBridge detects these capabilities and explains which features are available on your device.

**Release:** 0.1.1. CI tests and builds the project on all three operating systems. Apple Silicon SMC and Lenovo WMI charge control remain experimental and require device-level validation; see [hardware compatibility](docs/hardware.md) for the current support boundaries.

## Features

- Read battery percentage, charging state, power source and available health information.
- Set charge limits and start thresholds where supported by the device.
- Temporarily charge to 100% and restore the saved limit on supported backends.
- Keep charging settings active through a separate background service.
- Inspect device capabilities and export a compatibility report.
- Use the CLI independently of the tray app.
- Use the desktop app in English, Russian, Spanish, German, French, Japanese or Brazilian Portuguese, with system light/dark appearance.

## Downloads

<!-- downloads:start -->

Direct downloads for stable release [v0.1.0](https://github.com/0then0/BatteryBridge/releases/tag/v0.1.0):

| Platform | Architecture | Download | Installation |
| --- | --- | --- | --- |
| macOS 11+ | Apple Silicon (arm64) | [Installer (.pkg)](https://github.com/0then0/BatteryBridge/releases/download/v0.1.0/BatteryBridge-macos-apple-silicon.pkg) | App and background service; no Terminal required |
| macOS 11+ | Apple Silicon (arm64) | [Developer archive (.tar.gz)](https://github.com/0then0/BatteryBridge/releases/download/v0.1.0/batterybridge-macos-aarch64.tar.gz) | Manual service setup |
| Windows | x86_64 | [Developer archive (.zip)](https://github.com/0then0/BatteryBridge/releases/download/v0.1.0/batterybridge-windows-x86_64.zip) | Manual service setup |
| Linux | x86_64 | [Developer archive (.tar.gz)](https://github.com/0then0/BatteryBridge/releases/download/v0.1.0/batterybridge-linux-x86_64.tar.gz) | Manual service setup |

[SHA-256 checksums](https://github.com/0then0/BatteryBridge/releases/download/v0.1.0/SHA256SUMS.txt) · [Installation instructions](docs/install.md)

The macOS installer is unsigned and not notarized. Intel Mac charge control is unsupported. Windows and Linux archives are intended for users comfortable with manual service installation.

<!-- downloads:end -->

## Getting started

On an Apple Silicon Mac, download **BatteryBridge-macos-apple-silicon.pkg** from [Releases](https://github.com/0then0/BatteryBridge/releases). Open it in Finder, complete installation and open **Applications → BatteryBridge**. The package installs and starts the charging service; Terminal is not required. See [installation](docs/install.md) for unsigned-preview prompts, account requirements and supported systems.

Windows and Linux downloads are developer archives with separate service setup. The macOS package is unsigned; the installation guide explains the system prompts.

### Build from source

Build the CLI and service with Rust 1.90 or newer:

```sh
git clone https://github.com/0then0/BatteryBridge.git
cd BatteryBridge
cargo build --release --locked
./target/release/batterybridge status
./target/release/batterybridge capabilities
./target/release/batterybridge probe --json
```

On Windows, use `.\target\release\batterybridge.exe` in place of `./target/release/batterybridge`.

Status and compatibility reports work without installing the service. To change charging settings, follow the [service installation guide](docs/service.md), then run the CLI as your ordinary user:

```sh
batterybridge limit 80
batterybridge limit 80 --start 75
batterybridge limit off
```

A backend may support only some of these settings. For example, the Lenovo WMI backend accepts a fixed 80% limit or off. Check `batterybridge capabilities` before changing settings.

### Desktop app

The tray app uses Tauri, React and TypeScript. Install Node.js 22.12 or newer and the platform prerequisites listed in the [build guide](docs/build.md), then run:

```sh
cd apps/gui
npm ci
npm run tauri -- build --features desktop
```

The resulting app uses an embedded frontend. It does not require a web server on port 1420. A source-built app needs separate service setup; the macOS `.pkg` includes the service. Without service access, the app shows battery information in read-only mode.

See [building from source](docs/build.md) for bundle locations, development mode and standalone debug builds.

## Compatibility

- **Linux:** generic `charge_control_end_threshold` support, with a separate start threshold when exposed by the kernel. No vendor restriction.
- **macOS:** battery monitoring through IOKit; experimental Apple Silicon charge control through private SMC interfaces. Intel Mac charge control is unsupported.
- **Windows:** standard battery monitoring; experimental Lenovo WMI conservation mode with a fixed 80% limit, restricted to devices with one detected battery. Other vendor controllers are not yet implemented.

Capabilities use four states: `supported`, `experimental`, `unsupported` and `unknown`. A readable interface does not guarantee that firmware will permit a write. Full feature descriptions and validation requirements are in [hardware compatibility](docs/hardware.md).

## Documentation

Start with the [documentation index](docs/README.md), or open a guide directly:

- [Build and run](docs/build.md)
- [CLI and desktop usage](docs/usage.md)
- [Service installation and recovery](docs/service.md)
- [Configuration and charging policies](docs/configuration.md)
- [Hardware compatibility and testing](docs/hardware.md)
- [Architecture](docs/architecture.md)
- [Security and IPC](docs/security.md)

Compatibility reports include device model, OS version and detected charging interfaces. They exclude serial numbers, usernames and network identifiers. Review a report before attaching it to an [issue](https://github.com/0then0/BatteryBridge/issues).

## Contributing

[CONTRIBUTING.md](CONTRIBUTING.md) covers local checks, bug reports and pull requests. To add device support, read the [backend development guide](docs/contributing-backends.md). The [repository guide](docs/github.md) explains CI, build artifacts and the release process.

## Roadmap

Current priorities are device validation of the existing backends, broader Lenovo support, and controllers for ASUS, Dell, HP, MSI and Acer. Further work includes additional Linux vendor interfaces, firmware-managed Apple limits and a stable backend API.

## License

BatteryBridge and its original SVG icon are licensed under [Apache-2.0](LICENSE). Hardware-interface references are listed in the [compatibility guide](docs/hardware.md#interface-references).
