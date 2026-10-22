# Contributing to BatteryBridge

Keep the change focused on reliable charge control. Preserve unsupported states and explain any change to a device contract. Do not claim hardware support from CI or synthetic fixtures.

## Local checks

Use the committed Cargo and npm lockfiles. Stable Rust must meet the dependency MSRV (currently 1.90); Node.js 24 is used in CI.

```sh
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --workspace --all-features
```

For desktop checks, first install the OS prerequisites from Tauri's documentation, then run `npm ci`, `npm run check` and `npm run build` inside `apps/gui`. `npm run check` runs Biome and SVG accessibility checks. `npm run check:fix` formats and applies safe Biome fixes. For a standalone debug GUI, use `npm run build:native`; a Cargo build with only `desktop` requires the Vite dev server. CLI-only development can use `cargo clippy --workspace --all-targets` without the `desktop` feature. Do not install/synchronize unrelated project environments.

## New backend

Follow [docs/contributing-backends.md](docs/contributing-backends.md). Supply a read-only probe, typed errors, limited writes with validation and a synthetic fixture. Add hardware evidence separately: model family, firmware/driver versions, observed behavior, restore procedure. Never include serials, UUIDs or raw WMI instance paths in issues or fixtures.

## Hardware testing

See [docs/hardware.md](docs/hardware.md). The opt-in automated test only reads hardware; real writes require supervised manual validation on a consenting tester's laptop. Do not run hardware tests on CI runners, shared devices or production systems.

```sh
BB_HARDWARE_TEST=1 cargo test -p batterybridge-platform --test hardware -- --ignored
```

PowerShell: `$env:BB_HARDWARE_TEST='1'` before running the Cargo command.

Commit messages use Conventional Commits in English: `feat`, `fix`, `refactor`, `chore`, `docs`, `test`.
