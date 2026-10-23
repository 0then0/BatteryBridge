# Repository and releases

[Documentation index](README.md)

## Find project resources

- [Issues](https://github.com/0then0/BatteryBridge/issues): bugs, device compatibility and feature requests.
- [Pull requests](https://github.com/0then0/BatteryBridge/pulls): proposed changes and code review.
- [CI](https://github.com/0then0/BatteryBridge/actions/workflows/ci.yml): build and test results for Linux, Windows and macOS.
- [Release artifacts](https://github.com/0then0/BatteryBridge/actions/workflows/release.yml): packaged platform builds.
- [Releases](https://github.com/0then0/BatteryBridge/releases): published releases, when available.

For a compatibility issue, include the affected feature, device model, relevant firmware/driver versions and a reviewed `batterybridge probe --json` report. See [Contributing](../CONTRIBUTING.md) for reporting and pull request guidance.

## Continuous integration

The CI workflow runs on pushes and pull requests. It installs desktop build prerequisites, checks the frontend with Biome and TypeScript, then runs Rust formatting, Clippy, tests and builds with all features enabled. Windows also runs the installer ACL regression script.

A successful run confirms software checks on the runner operating systems. Device-level charging behavior is validated separately using the [hardware procedure](hardware.md#device-validation).

## Build artifacts

The release workflow runs for `v*` tags or through manual workflow dispatch. It produces archives for Linux x86_64, Windows x86_64 and macOS Apple Silicon, containing:

- CLI and service executables.
- Native desktop bundles under `gui`.
- Service integration files under `packaging`.
- Documentation, license and icon assets.
- `SHA256SUMS.txt` with hashes for the staged files.

Download an artifact from the completed workflow run and extract its archive. GitHub may require sign-in to download workflow artifacts. Desktop bundles are unsigned unless signing is configured separately. The additional `macos-installer` artifact contains a graphical `.pkg` with the app and charging service. Windows and Linux archives still require separate service installation.

## Prepare a release

Maintainers should verify the selected commit's CI results, review release notes in `packaging/macos/RELEASE.md` and compatibility information before creating a version tag. Pushing a `v*` tag builds all three platforms and publishes a GitHub Release only after every build succeeds. Tags containing a hyphen, such as `v0.1.0-dev.1`, create pre-releases. Manual workflow dispatch builds artifacts without publishing.

Release `v0.1.1` uses application version `0.1.1`, macOS bundle build `3` and Installer receipt version `0.1.1.3`, upgrading stable `0.1.0` (build `2`, receipt `0.1.0.2`) and the first preview (receipt `0.1.0.1`). The `v0.1.0-dev.1` tag remains available as a pre-release. Increment the receipt version for subsequent package updates using the builder's `--version` argument; keep the workflow and release tag consistent. Release assets include an outer `SHA256SUMS.txt` covering each downloadable package/archive.

Version numbers are declared in the root `Cargo.toml`, `apps/gui/package.json` and `apps/gui/src-tauri/tauri.conf.json`. Keep the corresponding lockfile metadata consistent when preparing a release.

Keep CLI, GUI and service versions together when distributing a build. Protocol-incompatible components reject requests explicitly.

## Dependency updates

Dependabot checks Cargo, npm and GitHub Actions dependencies monthly. Patch and minor updates are grouped, major updates are excluded from automatic proposals, and automatic merging is disabled. Review dependency changes through the same checks as other pull requests.
