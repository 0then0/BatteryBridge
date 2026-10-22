# Hardware validation and interface provenance

## Current evidence

No hardware-write certification has been recorded for BatteryBridge yet. Unit/fixture success validates logic and adapters against disposable resources, not a laptop's firmware behavior. The six committed fixtures are synthetic. Native macOS reads have been checked on the development host without issuing writes; the sandbox denied opening AppleSMC, so no control capability was confirmed there. Windows and Linux require their own native CI and device runs.

Only promote a model/firmware combination to verified support after recording: sanitized vendor/model, OS/kernel, firmware/driver version, probed capabilities, requested thresholds, reliable readback where available, actual charging transition, reboot/resume behavior, top-up completion and normal-charge restoration. Never attach device/battery serials.

## Supervised write procedure

1. Use an owned test laptop, keep a working AC adapter and close any other charge manager. Back up the current policy and note the hardware thresholds.
2. Run `batterybridge probe --json` and the ignored read-only hardware test. Check the detected contract, especially Lenovo's fixed 80% mode and firmware-dependent SMC keys.
3. Install the daemon using `service.md`; use CLI/GUI as an ordinary user. Set a limit near the current percentage. On native start/stop hardware, test both raising and lowering threshold pairs.
4. Confirm the real charging transition after a reasonable device-specific delay. A requested configuration is not proof of a hardware limit. Test invalid values and missing/denied interfaces without modifying arbitrary registers.
5. Where supported, start top-up, verify the saved policy is unchanged, and verify restoration at full charge. Test daemon restart and sleep/resume. Discharge testing requires supervision throughout; use a target above 20%.
6. End with `batterybridge limit off`, verify normal charging and remove the service if no longer testing. Do not force-kill a daemon while adapter isolation is active.

The automated hardware test is read-only and ignored by default:

```sh
BB_HARDWARE_TEST=1 cargo test -p batterybridge-platform --test hardware -- --ignored
```

## Sources and limits

These sources establish interface semantics; they are not BatteryBridge hardware test results. The implementation was written for this repository; third-party GPL source was not copied.

- [Linux power_supply sysfs ABI](https://www.kernel.org/doc/Documentation/ABI/testing/sysfs-class-power): `charge_control_start_threshold` / `charge_control_end_threshold`. Drivers define accepted ranges and may reject values.
- [Apple IOKit power-source API](https://developer.apple.com/documentation/iokit/iopowersources): public monitoring concepts. BatteryBridge reads allowlisted AppleSmartBattery registry properties and uses SDK-documented power notifications.
- [Apple IORegisterForSystemPower](https://developer.apple.com/documentation/iokit/1557134-ioregisterforsystempower): sleep acknowledgement and wake notifications; the FFI declarations were checked against the installed macOS SDK headers.
- [batt SMC charging implementation](https://github.com/charlie0129/batt/blob/master/pkg/smc/charging.go) and [battery charge-gate commands](https://github.com/actuallymentor/battery/blob/main/battery.sh): observed private `CH0B`/`CH0C` and `CHTE` payload semantics. BatteryBridge checks sizes/types and readback rather than assuming all Apple Silicon firmware is compatible.
- [batt firmware entitlement issue](https://github.com/charlie0129/batt/issues/152): private SMC writes may be denied even as root. Readable keys do not guarantee write privileges. No entitlement bypass is attempted.
- [Linux Lenovo WMI Other driver](https://github.com/torvalds/linux/blob/master/drivers/platform/x86/lenovo/wmi-other.c): feature `0x03010001`, boolean charge conservation, and unreliable getter caveat.
- [LenovoLegionToolkit WMI Other interface](https://github.com/BartoszCichecki/LenovoLegionToolkit/blob/master/LenovoLegionToolkit.Lib/System/Management/WMI.LenovoOtherMethod.cs): Windows namespace/class and named `GetFeatureValue` / `SetFeatureValue` parameters.
- [Lenovo Commercial Vantage configuration](https://docs.lenovocdrt.com/guides/lcv/configuration/): exported battery information is not a universal WMI threshold setter. BatteryBridge does not invent `Lenovo_PowerManager.SetChargeThreshold` or equate BIOS WMI with charge control.
- [Microsoft GetSystemPowerStatus](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getsystempowerstatus) and [Win32_Battery](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-battery): standard Windows monitoring, separate from vendor control.
