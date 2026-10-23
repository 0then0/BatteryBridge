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

On a macOS build host, install both Rust targets and build the universal GUI, CLI and daemon:

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
python3 packaging/macos/build-universal.py
python3 packaging/macos/build-pkg.py --output target/BatteryBridge-macos-universal.pkg
```

The package builder verifies both architectures in every executable and refuses a single-architecture payload. Universal output is in `target/universal-apple-darwin/release`. Native source builds remain useful for development but cannot be passed to this universal package builder.

Windows installer build (after `cargo build --release --locked`), from `apps/gui`:

```sh
npm run tauri -- build --features desktop --config ../../packaging/windows/tauri.installer.conf.json
```

On Linux, use `../../packaging/linux/tauri.deb.conf.json` or `tauri.rpm.conf.json` instead. These configs include the release CLI and daemon, so build them before bundling. Install/uninstall integration tests run only on disposable CI machines; do not execute the `test-installer` scripts on your personal system.

## Checks

See [Contributing](../CONTRIBUTING.md#local-checks) for the complete frontend and Rust checks. Changes to documentation can also be checked against the CLI's help output and generated Rust API documentation:

```sh
cargo run -p batterybridge-cli --locked -- --help
cargo doc --workspace --all-features --no-deps --locked
```

Building Rust documentation with desktop features requires the built frontend, just like other direct Cargo desktop builds.
