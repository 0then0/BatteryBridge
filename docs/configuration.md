# Configuration and charging policies

[Documentation index](README.md)

## Saved settings

The background service owns one machine-wide policy. Use `batterybridge limit` to change it and `batterybridge config` to inspect the saved TOML:

```toml
version = 1
charge_limit = 80

[start_threshold]
enabled = true
value = 75
```

`version` identifies the configuration format. `charge_limit` is the upper charging percentage. An enabled `start_threshold` requests the lower percentage at which charging resumes. Start must be less than stop, and the backend must support both requested values.

A disabled limit is represented by an omitted `charge_limit` field, not by zero. The `limit off` command also disables the explicit start threshold. Configuration paths are listed in the [service guide](service.md).

Prefer CLI changes over editing a running service's configuration file. For manual edits, stop the service, back up the file, edit it as administrator and restart the service. The service loads configuration at startup; it does not watch the file for external changes.

## Hysteresis

A lower start threshold prevents repeated charging transitions around the upper limit. For a 75–80% band, charging stops at 80% and resumes at the lower boundary.

Without an explicit start threshold, BatteryBridge uses a five-percentage-point band where a native start threshold or software charge gate is available. A device with only an end threshold uses that attribute alone.

On Linux, the backend writes native threshold attributes and reads them back. Off and top-up use start 99 and stop 100 when both attributes exist. A requested stop of 100 with no explicit start also uses start 99. Driver restrictions and rejected values are reported as errors.

On Apple Silicon, the service samples battery state every 30 seconds while awake and controls an SMC charging gate. This is a software policy, so the percentage can cross a boundary between samples. Firmware may change gate state during sleep or shutdown.

On the Lenovo WMI backend, the policy selects a fixed 80% conservation mode or off. Arbitrary percentages and start thresholds are unavailable.

## Temporary modes

Top-up and active force discharge exist only in service memory. They are not saved in TOML. Restarting the service restores the saved limit and clears temporary modes. Setting or disabling a limit also cancels temporary modes.

Top-up completes when the selected battery reaches 100% or reports full. Force discharge stops at its target and is cancelled on sensor failure, orderly shutdown or macOS sleep.

## Persistence and recovery

The service validates changes, applies them, then saves the policy before acknowledging success. If saving fails, it attempts to restore the previous policy. Hardware and filesystem errors remain visible to the caller.

Configuration is written to a sibling `config.toml.new` file, synced and renamed over the committed file. An existing transaction file is preserved for inspection. Recovery steps for interrupted writes are in the [service guide](service.md#troubleshooting).

Versionless configurations and version 0 migrate to version 1 in memory. The next successful limit change saves version 1. Unknown fields, invalid limits and future versions are rejected. Invalid configuration prevents service startup; it is not replaced with defaults. A missing configuration starts with the limit disabled.
