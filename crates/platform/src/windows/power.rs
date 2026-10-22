//! Win32 read-only power boundary.
use batterybridge_core::*;
use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
pub fn on_ac() -> Result<Option<bool>> {
    let mut status = SYSTEM_POWER_STATUS {
        ACLineStatus: 255,
        BatteryFlag: 255,
        BatteryLifePercent: 255,
        SystemStatusFlag: 0,
        BatteryLifeTime: u32::MAX,
        BatteryFullLifeTime: u32::MAX,
    };
    // SAFETY: the API writes exactly one initialized SYSTEM_POWER_STATUS to a
    // valid exclusive pointer. It neither retains the pointer nor writes hardware.
    if unsafe { GetSystemPowerStatus(&mut status) } == 0 {
        return Err(Error::io(
            "GetSystemPowerStatus",
            std::io::Error::last_os_error(),
        ));
    }
    Ok(match status.ACLineStatus {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    })
}
