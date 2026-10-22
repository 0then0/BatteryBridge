# Developing a backend

## Discovery

`Platform::discover` reads device metadata and allowlisted interfaces into `Facts`. The pure `resolve` function applies OS, architecture, vendor and interface checks and produces a `ProbeReport`. Fixtures use that same resolver. A candidate reports why it was selected or rejected; a failed probe remains a visible diagnostic, not a fabricated success. Linux does not depend on vendor names. Windows Lenovo requires capability records and method availability in addition to the manufacturer.

The preview has one controller candidate per OS. When adding competing vendor adapters, expand the candidate list and give exact interface matches precedence over broad vendor matches; do not replace errors with unconditional fallback writes. Read-only monitoring remains independent of controller activation.

## Traits and platform boundaries

- `BatteryMonitor`: returns battery status without granting writes.
- `ChargeController`: capabilities, validated start/stop application, optional gate control and optional force discharge. `release` restores normal charging.
- `DeviceInfoProvider`: safe OS/vendor/model metadata.
- `PowerMonitor`: minimal AC connectivity, available for future event-driven logic.

Models/DTOs live in `batterybridge-protocol`; policy and traits live in `batterybridge-core`, which forbids unsafe code. Native code belongs in a separate module under `batterybridge-platform` (or a separate backend crate when justified). Do not add model-specific SMC keys, WMI method names or device paths to core. Generic policy must not learn vendor/model conditionals.

## Capabilities

Use `Supported` only for a recognized and validated interface contract. Use `Experimental` for an implemented private hardware interface without recorded device-level write validation; this permits explicit control attempts without claiming verified support. This records interface availability, not a hardware certification. Use `Unknown` when probing cannot establish a contract, and `Unsupported` when absent. Explain the reason per feature. Report fixed thresholds through `allowed_limits`; never round silently. Native thresholds and daemon-enforced SMC thresholds must be described accurately.

A missing charge controller must not disable battery monitoring. Probe must never write even to test access permissions. Check permissions at write time and return a typed error.

## Probe and fixtures

Query only named fields. Battery identifiers are ordinals (`BAT0`), not serials or WMI paths. Keep raw instance paths only inside the Windows adapter. Do not serialize arbitrary firmware/provider objects or include raw command output in diagnostics.

Add JSON under `tests/fixtures` and include it in `crates/platform/tests/fixtures.rs`. Record expected backend selection, all six capability states and battery status. Fixtures must be labelled synthetic unless collected and sanitized from real hardware. Add rejection fixtures when vendor strings match but interfaces do not. Test the actual interface adapter against temporary files or a mock transport as well as the resolver.

## Writes and failure recovery

The privileged daemon is the only production caller of controller mutations. Expose fixed operations over the protocol, never arbitrary keys, register addresses, filesystem paths, shell commands or WMI methods. Validate both the client request and backend input. Read back writes where the provider offers a reliable getter. If readback is unreliable, document that explicitly and do not invent observed state.

Keep start <= stop at each intermediate threshold write. Check each return code, handle partial failure and release force discharge on errors, missing sensors, target completion, shutdown and suspend. Add supervised hardware validation before promoting a platform/model to verified support. The Linux sysfs writer is isolated in `linux.rs`; macOS unsafe calls are in its IOKit/SMC/power modules; Windows unsafe calls are in its Win32 power module. Every unsafe operation needs a local safety justification and matching ABI evidence.

## Extending the IPC contract

Version protocol and configuration independently. Serde request DTOs reject unknown fields. Never use a new boolean or a model-specific branch in core to pretend that a different vendor operation is equivalent. Extend the controller module and its capability metadata first. Keep GUI a consumer of the same daemon commands as CLI.
