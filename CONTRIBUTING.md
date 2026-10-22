# Contributing to BatteryBridge

Contributions are welcome for device support, charging-policy correctness, diagnostics, accessibility and documentation. Start with the [architecture guide](docs/architecture.md) to find the relevant component.

## Report a problem

Search existing [issues](https://github.com/0then0/BatteryBridge/issues) before opening a new one. Include the BatteryBridge version, OS version, expected behavior, actual behavior and reproduction steps. For compatibility problems, attach a reviewed `batterybridge probe --json` report and describe the affected charging feature.

Remove personal identifiers from logs and reports. Include device model and relevant firmware or driver versions, but omit serials, UUIDs and raw WMI instance paths. CI results and synthetic fixtures should be identified separately from tests on a physical laptop.

## Development setup

Follow [building from source](docs/build.md). Use the committed Cargo and npm lockfiles. Rust 1.90 or newer is required; CI uses stable Rust and Node.js 24. The desktop app also requires Tauri's platform dependencies.

## Local checks

From the repository root, prepare and check the frontend first:

```sh
cd apps/gui
npm ci
npm run check
npm run build
cd ../..
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo build --workspace --all-features --locked
```

`npm run check` runs Biome and SVG accessibility checks. Use `npm run check:fix` in `apps/gui` to apply safe fixes, and `cargo fmt --all` from the root to format Rust. Inspect formatting changes before committing. Git attributes keep text files in LF format on all platforms.

For CLI/service work without desktop prerequisites, check the default workspace members:

```sh
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo build --locked
```

Tests use synthetic facts, fake controllers and temporary files. Normal CI does not write battery controls or install services. Windows CI additionally checks installer ACL behavior using disposable directories.

## Add a backend

Read [developing a backend](docs/contributing-backends.md). Include discovery reasons, per-feature capabilities, typed errors and tests for both matching and rejected devices. Hardware validation belongs in a separate report with the tested model, firmware and restore procedure.

## Hardware testing

The [hardware guide](docs/hardware.md#device-validation) describes supervised validation. The automated hardware test only reads device information and is ignored by default. Run writes only on a device whose owner has agreed to the test, and supervise force discharge throughout.

## Submit a pull request

Explain the problem, resulting behavior and checks you ran. Keep unrelated changes separate and add regression coverage for changed behavior. Update user documentation when commands, capabilities or installation requirements change.

Use Conventional Commits in English: `feat`, `fix`, `refactor`, `chore`, `docs` or `test`. The same format is suitable for pull request titles.
