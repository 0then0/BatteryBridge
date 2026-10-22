<p align="center"><img src="assets/icon.svg" alt="BatteryBridge icon" width="128" height="128"></p>

# BatteryBridge

[![CI](https://img.shields.io/github/actions/workflow/status/0then0/BatteryBridge/ci.yml?label=CI)](https://github.com/0then0/BatteryBridge/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.90%2B-934b32)](https://www.rust-lang.org/)
[![Stage](https://img.shields.io/badge/stage-development%20preview-99743b)](docs/hardware.md)

A shared charge policy for laptops with different hardware. BatteryBridge exposes battery status, charge limits and compatibility reports through a Rust API, standalone CLI, privileged service and small Tauri tray app.

**Development preview, not a hardware-certified MVP.** The adapters below contain real interface implementations and are covered by synthetic fixtures. No model has yet been validated for hardware writes in this repository. A successful build or a readable SMC key does not prove that firmware permits a write. CI configuration is provided; a green remote run must be observed before declaring cross-platform validation.

There is no universal battery charge-control API. Linux kernel drivers, Apple firmware and Windows vendor providers decide which operations exist. BatteryBridge probes those interfaces and returns an explicit error when a feature is unavailable. It never substitutes a simulated limit for a hardware operation.

## What is implemented

- Separate `BatteryMonitor`, `ChargeController`, `PowerMonitor` and `DeviceInfoProvider` traits.
- Read-only discovery with candidate reasons, per-feature capabilities and JSON diagnostics.
- Linux sysfs thresholds, Apple Silicon SMC gates and a restricted Lenovo WMI conservation-mode adapter.
- Versioned TOML policy, start/stop hysteresis, temporary full charge where available, restoration on restart and periodic reconciliation after resume.
- Framed Serde IPC, Unix socket permissions / Windows pipe ACLs, bounded requests and daemon-owned writes.
- React + TypeScript tray UI over the same daemon API. Closing the window hides it; quitting the UI leaves the policy service running.

## Build

Use current stable Rust (dependencies require at least 1.90). CLI and service need no Node.js or WebView runtime:

```sh
cargo build --locked --release
./target/release/batterybridge status
./target/release/batterybridge probe --json
```

For the desktop app, install Node.js 22.12+ (CI uses 24) and the [Tauri platform prerequisites](https://v2.tauri.app/start/prerequisites/). On Ubuntu 24.04 these include `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `libxdo-dev`, `libssl-dev` and a C/C++ build toolchain.

```sh
cd apps/gui
npm ci
npm run tauri -- dev --features desktop
# Unsigned native application bundle:
npm run tauri -- build --features desktop
```

From the repository root, build a standalone debug binary with the frontend embedded:

```sh
cd apps/gui
npm run build:native
../../target/debug/batterybridge-gui
```

`cargo build --features desktop` alone is a development-server build and expects Vite on port 1420. Standalone Cargo builds need `desktop,custom-protocol` (or `--all-features`); Tauri CLI handles this automatically for `tauri build`. `tauri dev` intentionally uses Vite for live reload.

The GUI feature is optional. Full workspace quality gates include it. The desktop app does not install or elevate the daemon automatically; follow [service installation](docs/service.md). Do not run the GUI as root/admin.

## CLI

```sh
batterybridge status
batterybridge capabilities
batterybridge limit 80
batterybridge limit 80 --start 75
batterybridge limit off
batterybridge top-up
batterybridge discharge 80
batterybridge probe
batterybridge probe --json > compatibility.json
batterybridge config
```

CLI and GUI status, capabilities and compatibility reports work without the service, using a read-only local probe. The GUI disables charge controls and shows read-only mode. In that case policy state is **unavailable**, not a claim that charging limits are off. `probe` always runs locally and never writes hardware. Mutations and configuration reads require the service. `--json` also works with status, capabilities and config. Diagnostics go to stderr; `-v`, `-vv` and `RUST_LOG` control verbosity.

The MVP controls the **first present battery**, with a stable discovery order. Other batteries are listed but are not changed. `limit off` restores the platform's normal charging settings (on sysfs with a start attribute, start 99 / stop 100). It does not restore a previous third-party manager's thresholds. Use one charge-policy manager at a time.

Top-up is temporary: the saved limit remains unchanged and is restored when the first battery reaches 100% or reports full. Ephemeral modes end on daemon restart; a restart immediately restores the persistent policy. Force discharge has a floor of 20%, stops at its target, and is cancelled on sensor failure, shutdown or macOS sleep.

## Hardware support matrix

These are implementation boundaries, **not a list of hardware-tested laptops**. `supported` identifies the standard Linux kernel interface contract. Private SMC and Lenovo WMI interfaces are labelled `experimental` until device-level write validation is recorded. Actual writes can still fail with `PermissionDenied` or `HardwareError`.

- **Linux, generic kernel thresholds:** charge limit when `charge_control_end_threshold` is readable on the selected battery; start threshold only when its separate attribute exists. No vendor-name restriction. Every write is read back. Top-up sets stop 100 temporarily. Force discharge and adapter disable are unsupported in this generic adapter. Driver-specific minimum values are enforced by the kernel; rejected values are reported.
- **macOS, Apple Silicon:** battery reads through IOKit; charge gates through validated `CHTE` or `CH0B` + `CH0C` payloads. The daemon enforces the numerical limit with hysteresis while awake. Optional `CH0I` force discharge requires the corresponding key. Sleep notifications release adapter isolation before acknowledgement and reconcile on wake. Firmware may remove keys or deny even root writes. New firmware-managed `bf*` limits and entitlement bypasses are not implemented. Intel Mac control is unsupported; monitoring can still work.
- **Windows, Lenovo Other Method:** requires Lenovo device metadata, `root\WMI:LENOVO_OTHER_METHOD`, both feature methods, a matching `LENOVO_CAPABILITY_DATA_01` record for `0x03010001`, and a successful safe getter. Control is enabled only with exactly one detected battery. Only **fixed 80% conservation mode / off** is implemented. This is not a generic ThinkPad threshold API. Start thresholds, top-up, discharge and arbitrary limits are unsupported. Known getter ambiguity prevents reporting a numeric observed hardware limit. Hardware validation remains pending.
- **Windows, other vendors:** standard battery information through WMI and AC status through Win32; control is unsupported. ASUS, Dell, HP, MSI and Acer backends are planned.

SMC's software policy is not a firmware percentage threshold. Polling is every 30 seconds while awake, so percentage can cross the target between samples. Limits during sleep/shutdown cannot be guaranteed on firmware that resets the keys. A launchd restart repairs state after a crash, but forced termination is not a hardware fail-safe. See [hardware testing and sources](docs/hardware.md).

## Configuration

The service owns its system-wide policy. Run `batterybridge config` to inspect it. Example:

```toml
version = 1
charge_limit = 80

[start_threshold]
enabled = true
value = 75
```

CLI `limit 80 --start 75` validates and persists this policy. Without `--start`, available native start thresholds and SMC use a five-point hysteresis band. Top-up/off uses native start 99 and stop 100, rather than assuming the generic ABI gives start 0 a special meaning. Start must be below stop. An end-threshold-only device uses only its end threshold. A versionless v0 configuration migrates in memory to v1 and is written as v1 on the next policy change. Unknown fields and future versions fail explicitly. Top-up and active discharge are never written to user policy.

System paths, authorization rules and installation commands: [docs/service.md](docs/service.md).

## Diagnostics and development

Reports contain OS/version/architecture, vendor/model, local battery ordinals, allowlisted interface names, backend decisions and capabilities. They exclude usernames, hostnames, serials, network identifiers, installation UUIDs and raw WMI instance paths. Review reports before sharing; arbitrary provider output is not dumped.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --workspace --all-features
```

Run `npm ci`, `npm run check` and `npm run build` in `apps/gui` before building all desktop features from Cargo. Unit tests cover policy/configuration; six synthetic fixtures cover discovery; hardware tests are ignored unless explicitly enabled. No CI test writes to hardware.

See [CONTRIBUTING.md](CONTRIBUTING.md), [backend development](docs/contributing-backends.md), [IPC and security](docs/security.md), and [GitHub repository metadata](docs/github.md).

CI builds/tests Linux, Windows and macOS. The release workflow prepares CLI/service archives and unsigned GUI bundles on tags or manual dispatch; it does not automatically publish a GitHub Release. Signing/notarization and elevated service installation are separate deployment steps. Dependabot runs monthly, groups patch/minor updates, ignores majors and does not auto-merge.

## Roadmap

Hardware validation of the current adapters comes first. Then: broader Windows Lenovo interfaces; ASUS, Dell, HP, MSI and Acer controllers; additional Linux vendor interfaces; firmware-managed Apple limits; public API stabilization and a backend SDK. Advanced power telemetry follows reliable charge control. Process/GPU analytics, fan control and calibration dashboards are outside this preview.

## License

[Apache-2.0](LICENSE). The SVG icon is original project artwork under the same license. Hardware-interface references are documented with their provenance; no third-party source code has been vendored.
