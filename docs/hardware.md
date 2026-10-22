# Hardware compatibility and validation

[Documentation index](README.md)

Charge-control support depends on the device's kernel driver, vendor provider and firmware. Run `batterybridge capabilities` for feature availability and `batterybridge probe --json` for discovery details.

## Capability states

- `supported`: a recognized standard interface contract is available. Linux threshold attributes use this state; actual writes still depend on permissions and driver restrictions.
- `experimental`: an implemented private interface is available, but device-level write validation has not been recorded. Apple Silicon SMC and Lenovo WMI control currently use this state.
- `unsupported`: the feature or a required interface is unavailable.
- `unknown`: discovery could not establish availability, for example because access was denied.

No completed device-level write-validation reports are currently recorded in the project. The committed device fixtures are synthetic. CI validates software behavior on Linux, Windows and macOS; it does not establish charging behavior on a particular laptop.

## Linux: kernel threshold attributes

The generic backend uses `/sys/class/power_supply/` and selects the first present battery in sorted discovery order. It requires a readable `charge_control_end_threshold` for charge limits. A readable `charge_control_start_threshold` adds explicit start-threshold support. Selection is independent of the device vendor.

Writes are read back, and threshold pairs are ordered to keep start below stop. The adapter attempts to restore the previous pair after a failed update. Driver-specific limits or rounded values can cause a rejected request or readback error.

Top-up temporarily sets the end threshold to 100. When the start attribute exists, off and top-up use start 99 and stop 100. Generic force discharge and adapter disable are unsupported. Additional batteries are listed but are not controlled.

## macOS: Apple Silicon SMC

Battery status is read through IOKit. Experimental charging control uses the private `CHTE` gate or the `CH0B`/`CH0C` pair, with payload size/type validation and write readback. The service enforces the requested percentage using hysteresis and a 30-second sampling interval while awake.

Temporary full charge is available when a compatible gate is found. Force discharge additionally requires a compatible `CH0I` key. Firmware may remove keys or deny writes, including privileged writes. Firmware-managed `bf*` limits are not implemented. Intel Mac charge control is unsupported; monitoring can still work.

On sleep, the service releases adapter isolation before acknowledging IOKit. With a saved limit and no top-up, it then requests charging inhibition. Policy is reconciled on wake. Firmware may reset gate state during sleep or shutdown, so percentage limits cannot be guaranteed in those states. Abrupt process termination can leave a gate latched until the service restarts.

## Windows: Lenovo WMI

Standard monitoring uses `Win32_Battery` and Win32 power status. Experimental Lenovo control requires all of the following:

- Lenovo device metadata and exactly one detected battery.
- `root\WMI:LENOVO_OTHER_METHOD` with `GetFeatureValue` and `SetFeatureValue`.
- A matching boolean `LENOVO_CAPABILITY_DATA_01` record for feature `0x03010001`.
- A successful read-only getter with a recognized value.

This backend supports fixed **80% conservation mode or off**. Arbitrary limits, explicit start thresholds, top-up and force discharge are unavailable. The getter may describe inhibition rather than the configured mode, so BatteryBridge does not report it as an observed numerical limit.

Other Windows vendor controllers, including ASUS, Dell, HP, MSI and Acer, are not yet implemented. Standard monitoring remains available when the required system interfaces can be read.

## Device validation

Test on a laptop whose owner has agreed to the procedure. Keep an AC adapter available, record the current thresholds and policy, and stop other charge managers before testing.

1. Save `batterybridge probe --json` and run the read-only hardware test below.
2. Install the [service](service.md) and use CLI/GUI as an ordinary user.
3. Set a limit near the current percentage. For native threshold pairs, test both raising and lowering the pair.
4. Observe actual charging transitions and reliable readback where available. A saved setting alone does not confirm enforcement. Record errors from unavailable interfaces and rejected values.
5. Where supported, test top-up completion, service restart and sleep/resume. Verify that the saved limit is restored. Supervise force discharge throughout and use a target above 20%.
6. Run `batterybridge limit off`, confirm normal charging, and remove the service if it is no longer needed. Use orderly shutdown while adapter isolation is active.

A validation report should include sanitized vendor/model, OS/kernel version, firmware/driver versions, requested settings, observed behavior, readback results and restoration steps. Exclude device and battery serial numbers.

### Automated read-only test

The hardware test is ignored by default and only reads device information:

```sh
BB_HARDWARE_TEST=1 cargo test -p batterybridge-platform --test hardware --locked -- --ignored
```

In PowerShell:

```powershell
$env:BB_HARDWARE_TEST = '1'
cargo test -p batterybridge-platform --test hardware --locked -- --ignored
Remove-Item Env:BB_HARDWARE_TEST
```

## Interface references

These references describe interface semantics and implementation constraints. They are separate from BatteryBridge device-validation results.

- [Linux power_supply sysfs ABI](https://www.kernel.org/doc/Documentation/ABI/testing/sysfs-class-power): native start/end thresholds and driver-defined ranges.
- [Apple IOKit power-source API](https://developer.apple.com/documentation/iokit/iopowersources): public battery-monitoring interfaces.
- [Apple IORegisterForSystemPower](https://developer.apple.com/documentation/iokit/1557134-ioregisterforsystempower): sleep acknowledgement and wake notifications.
- [batt SMC charging implementation](https://github.com/charlie0129/batt/blob/master/pkg/smc/charging.go) and [battery charge-gate commands](https://github.com/actuallymentor/battery/blob/main/battery.sh): observed private charge-gate payload semantics.
- [batt firmware entitlement issue](https://github.com/charlie0129/batt/issues/152): firmware-dependent restrictions on private SMC writes.
- [Linux Lenovo WMI Other driver](https://github.com/torvalds/linux/blob/master/drivers/platform/x86/lenovo/wmi-other.c): feature `0x03010001`, conservation mode and getter limitations.
- [LenovoLegionToolkit WMI Other interface](https://github.com/BartoszCichecki/LenovoLegionToolkit/blob/master/LenovoLegionToolkit.Lib/System/Management/WMI.LenovoOtherMethod.cs): Windows namespace, class and named method parameters.
- [Lenovo Commercial Vantage configuration](https://docs.lenovocdrt.com/guides/lcv/configuration/): vendor information exports and configuration scope.
- [Microsoft GetSystemPowerStatus](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getsystempowerstatus) and [Win32_Battery](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-battery): standard Windows monitoring.
