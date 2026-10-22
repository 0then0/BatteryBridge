# Using BatteryBridge

[Documentation index](README.md)

## Inspect your device

```sh
batterybridge status
batterybridge capabilities
batterybridge probe
```

`status` shows detected batteries, power source, available health information and the selected backend. When connected to the service, it also shows the saved charge limit and temporary top-up state.

`capabilities` lists each charging feature with its availability and an explanation. `probe` collects a compatibility report using read-only local discovery. Both the CLI and tray app can read device information without the service. In that mode, charging policy is shown as unavailable because the service's state is unknown.

## Set a charge limit

Install the [background service](service.md), then run these commands as your ordinary user:

```sh
batterybridge limit 80
batterybridge limit 80 --start 75
batterybridge config
```

The first command sets an upper limit of 80%. The second also requests a 75% start threshold. Explicit start thresholds require the corresponding capability. Stop values range from 1 to 100; hardware may accept a narrower range. The Lenovo WMI backend accepts only 80% or off.

`config` displays the saved TOML policy. Successful limit changes are saved by the service and reapplied after it restarts. See [charging policies](configuration.md) for hysteresis and persistence behavior.

To restore normal charging:

```sh
batterybridge limit off
```

On sysfs devices with both attributes, this sets start 99 and stop 100. It does not restore another charge manager's previous settings. Run one charge-policy manager at a time.

## Charge to 100% temporarily

```sh
batterybridge top-up
```

Top-up requires an existing saved limit and a backend with the `temporary_full_charge` capability. The service temporarily releases the limit, then restores it when the selected battery reaches 100% or reports full. Restarting the service also ends top-up and restores the saved policy.

## Force discharge

```sh
batterybridge discharge 80
```

This advanced command requires a backend with force-discharge support. It stops at the target percentage and has a minimum target of 20%. Keep the device under supervision during discharge. Sensor failure, service restart, orderly shutdown and macOS sleep cancel the temporary discharge mode.

## Use the tray app

The app shows battery status, charging settings, the active backend and device capabilities. In read-only mode, status and reports remain available while controls are disabled.

Closing the window hides it. Choose **Open BatteryBridge** from the tray or menu-bar icon to show it again. **Quit BatteryBridge** exits the GUI; the separately installed service continues enforcing its policy.

## Share a compatibility report

```sh
batterybridge probe --json > compatibility.json
```

You can also choose **Generate compatibility report** in the app, review the JSON and save it. Reports include model and OS information, local battery ordinals and detected interfaces. They exclude serial numbers, usernames, network addresses and installation identifiers. Review the content before attaching it to an issue.

The `--json` option also works with `status`, `capabilities` and `config`. Increase logging with `-v` or `-vv`, or set `RUST_LOG`. Logs are written to stderr so they can be kept separate from JSON output.

## When a command fails

An unsupported feature includes a reason in the capabilities output. If policy is unavailable, check the service and your local access permissions using the [installation and recovery guide](service.md#troubleshooting). Permission and hardware errors can also come from firmware or a missing vendor driver; a successful read-only probe does not establish write access.
