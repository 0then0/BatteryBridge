#![forbid(unsafe_code)]

pub mod config;
pub mod policy;
pub use batterybridge_protocol::*;

pub trait BatteryMonitor {
    fn batteries(&self) -> Result<Vec<BatteryStatus>>;
}
pub trait DeviceInfoProvider {
    fn device_info(&self) -> Result<DeviceInfo>;
}
pub trait PowerMonitor {
    fn on_ac(&self) -> Result<Option<bool>>;
}

/// Only a privileged service owns a controller. Read-only clients never receive one.
pub trait ChargeController {
    fn capabilities(&self) -> Capabilities;
    fn apply(&mut self, start: Option<u8>, stop: Option<u8>) -> Result<()>;
    fn force_discharge(&mut self, enabled: bool) -> Result<()> {
        let _ = enabled;
        Err(Error::Unsupported("force discharge".into()))
    }
    /// SMC binary gates use daemon hysteresis; native thresholds ignore this call.
    fn charging(&mut self, enabled: bool) -> Result<()> {
        let _ = enabled;
        Ok(())
    }
    fn release(&mut self) -> Result<()> {
        self.apply(None, None)
    }
}
