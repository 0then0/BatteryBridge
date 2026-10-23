# Building from source

[Documentation index](README.md)

## Requirements

Use stable Rust 1.90 or newer. CLI and service builds require a native toolchain for the target operating system, but do not require Node.js or a WebView.

Desktop builds additionally require Node.js 22.12 or newer, npm and the [Tauri platform prerequisites](https://v2.tauri.app/start/prerequisites/). CI uses Node.js 24. On Windows, use the MSVC toolchain and install the C++ build tools and WebView2 requirements described by Tauri. On macOS, install Xcode Command Line Tools.

On Ubuntu 24.04, the desktop dependencies can be installed with:

```sh
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential libxdo-dev libssl-dev librsvg2-dev libayatana-appindicator3-dev patchelf
```

Use the committed Cargo and npm lockfiles for reproducible dependency resolution.

## CLI and service

Run from the repository root:

```sh
cargo build --release --locked
./target/release/batterybridge --help
./target/release/batterybridge status
```

The default workspace members produce `batterybridge` and `batterybridge-service` in `target/release`. Windows binaries have an `.exe` suffix. A build does not install the service; follow [service installation](service.md) when you are ready to enable charging control.

## Desktop application

Run from the repository root:

```sh
cd apps/gui
npm ci
npm run tauri -- build --features desktop
```

Tauri builds the frontend, embeds it in the application and creates platform bundles under `target/release/bundle`. On macOS, the application is under `target/release/bundle/macos/BatteryBridge.app`. Local bundles are unsigned unless signing is configured separately.

### macOS installer package

On an Apple Silicon Mac, first build the release CLI and service from the repository root with `cargo build --release --locked`, then build the desktop bundle with `npm --prefix apps/gui run tauri -- build --features desktop --bundles app`. Package all three components together:

```sh
python3 -m unittest discover -s packaging/macos/tests -v
python3 packaging/macos/build-pkg.py --output target/release/BatteryBridge-macos-apple-silicon.pkg
```

The builder uses macOS `pkgbuild`, `productbuild` and `lipo`. It does not install or start the service on the build machine. The output is an unsigned graphical installer for the current system volume, with root-owned privileged helpers and a non-relocatable app in Applications. Use Developer ID signing and notarization before general public distribution.

### Standalone debug binary

```sh
cd apps/gui
npm ci
npm run build:native
../../target/debug/batterybridge-gui
```

On Windows, start `..\..\target\debug\batterybridge-gui.exe`. This build embeds the frontend and runs without a development server.

### Development mode

```sh
cd apps/gui
npm ci
npm run tauri -- dev --features desktop
```

Development mode starts Vite on port 1420 and loads it in the native window for live updates. Opening that URL in a regular browser does not provide Tauri's native IPC bridge.

A direct Cargo build with only the `desktop` feature expects the development server. For a standalone Cargo build, first run `npm run build` in `apps/gui`, then use:

```sh
cargo build -p batterybridge-gui --features desktop,custom-protocol --locked
```

Run this Cargo command from the repository root. Tauri CLI selects the appropriate frontend mode automatically.

## Checks

See [Contributing](../CONTRIBUTING.md#local-checks) for the complete frontend and Rust checks. Changes to documentation can also be checked against the CLI's help output and generated Rust API documentation:

```sh
cargo run -p batterybridge-cli --locked -- --help
cargo doc --workspace --all-features --no-deps --locked
```

Building Rust documentation with desktop features requires the built frontend, just like other direct Cargo desktop builds.
