//! Platform-independent charging policy, monitoring/controller traits and TOML persistence.
//!
//! Device-specific interfaces are implemented by `batterybridge-platform`.

#![forbid(unsafe_code)]

/// Validation, migration and filesystem persistence for the saved TOML policy.
pub mod config;
/// Charging hysteresis and temporary top-up/discharge state transitions.
pub mod policy;
pub use batterybridge_protocol::*;

/// Read-only battery information, independent of charge-control availability.
pub trait BatteryMonitor {
    /// Read the currently detected batteries in platform discovery order.
    fn batteries(&self) -> Result<Vec<BatteryStatus>>;
}
/// Device metadata suitable for display and compatibility reports.
pub trait DeviceInfoProvider {
    fn device_info(&self) -> Result<DeviceInfo>;
}
/// AC connectivity for the selected device.
pub trait PowerMonitor {
    /// Return `None` when the power source cannot be determined.
    fn on_ac(&self) -> Result<Option<bool>>;
}

/// Only a privileged service owns a controller. Read-only clients never receive one.
pub trait ChargeController {
    /// Describe available operations and any restricted limit values.
    fn capabilities(&self) -> Capabilities;
    /// Apply validated percentages, or request normal charging with both `None`.
    fn apply(&mut self, start: Option<u8>, stop: Option<u8>) -> Result<()>;
    /// Enable or release forced discharge, when supported by the adapter.
    fn force_discharge(&mut self, enabled: bool) -> Result<()> {
        let _ = enabled;
        Err(Error::Unsupported("force discharge".into()))
    }
    /// SMC binary gates use daemon hysteresis; native thresholds ignore this call.
    fn charging(&mut self, enabled: bool) -> Result<()> {
        let _ = enabled;
        Ok(())
    }
    /// Restore normal charging and release any adapter-specific isolation.
    fn release(&mut self) -> Result<()> {
        self.apply(None, None)
    }
}
