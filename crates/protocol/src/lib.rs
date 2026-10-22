#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PROTOCOL_VERSION: u32 = 2;
pub const MAX_FRAME: usize = 64 * 1024;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error, PartialEq, Eq)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum Error {
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("service unavailable: {0}")]
    ServiceUnavailable(String),
    #[error("backend unavailable: {0}")]
    BackendUnavailable(String),
    #[error("hardware error: {0}")]
    HardwareError(String),
    #[error("invalid limit: {0}")]
    InvalidLimit(String),
    #[error("platform error: {0}")]
    PlatformError(String),
    #[error("configuration error: {0}")]
    ConfigurationError(String),
    #[error("protocol error: {0}")]
    ProtocolError(String),
}
impl Error {
    pub fn io(operation: &str, e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied(operation.into()),
            _ => Self::PlatformError(format!("{operation}: {e}")),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    Supported,
    Experimental,
    Unsupported,
    Unknown,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Capability {
    pub state: CapabilityState,
    pub reason: String,
}
impl Capability {
    pub fn supported(reason: impl Into<String>) -> Self {
        Self {
            state: CapabilityState::Supported,
            reason: reason.into(),
        }
    }
    pub fn experimental(reason: impl Into<String>) -> Self {
        Self {
            state: CapabilityState::Experimental,
            reason: reason.into(),
        }
    }
    pub fn unsupported(reason: impl Into<String>) -> Self {
        Self {
            state: CapabilityState::Unsupported,
            reason: reason.into(),
        }
    }
    pub fn unknown(reason: impl Into<String>) -> Self {
        Self {
            state: CapabilityState::Unknown,
            reason: reason.into(),
        }
    }
    pub fn available(&self) -> bool {
        matches!(
            self.state,
            CapabilityState::Supported | CapabilityState::Experimental
        )
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Capabilities {
    pub charge_limit: Capability,
    pub start_threshold: Capability,
    pub stop_threshold: Capability,
    pub force_discharge: Capability,
    pub adapter_disable: Capability,
    pub temporary_full_charge: Capability,
    /// None means a continuous range 1..=100; Some restricts accepted limits.
    pub allowed_limits: Option<Vec<u8>>,
}
impl Capabilities {
    pub fn unsupported(reason: &str) -> Self {
        let c = Capability::unsupported(reason);
        Self {
            charge_limit: c.clone(),
            start_threshold: c.clone(),
            stop_threshold: c.clone(),
            force_discharge: c.clone(),
            adapter_disable: c.clone(),
            temporary_full_charge: c,
            allowed_limits: None,
        }
    }
    pub fn entries(&self) -> [(&str, &Capability); 6] {
        [
            ("Charge limit", &self.charge_limit),
            ("Start threshold", &self.start_threshold),
            ("Stop threshold", &self.stop_threshold),
            ("Force discharge", &self.force_discharge),
            ("Adapter disable", &self.adapter_disable),
            ("Top up", &self.temporary_full_charge),
        ]
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BatteryStatus {
    /// A local ordinal such as BAT0, never a serial number or WMI instance path.
    pub id: String,
    pub percent: Option<u8>,
    pub state: String,
    pub health_percent: Option<u8>,
    pub on_ac: Option<bool>,
    pub hardware_start: Option<u8>,
    pub hardware_stop: Option<u8>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceInfo {
    pub os: String,
    pub os_version: String,
    pub architecture: String,
    pub vendor: String,
    pub model: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Candidate {
    pub name: String,
    pub selected: bool,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProbeReport {
    pub schema_version: u32,
    pub device: DeviceInfo,
    pub battery_ids: Vec<String>,
    pub interfaces: BTreeMap<String, String>,
    pub candidates: Vec<Candidate>,
    pub capabilities: Capabilities,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StartThreshold {
    pub enabled: bool,
    pub value: u8,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    pub charge_limit: Option<u8>,
    pub start_threshold: StartThreshold,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            charge_limit: None,
            start_threshold: StartThreshold {
                enabled: false,
                value: 75,
            },
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub batteries: Vec<BatteryStatus>,
    pub device: DeviceInfo,
    pub backend: String,
    pub capabilities: Capabilities,
    /// None when the daemon policy is unavailable; never synthesize a saved limit.
    pub config: Option<Config>,
    pub top_up_active: bool,
    pub discharge_target: Option<u8>,
    pub last_error: Option<Error>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Status,
    Capabilities,
    Probe,
    Config,
    Limit { stop: Option<u8>, start: Option<u8> },
    TopUp,
    Discharge { target: u8 },
}
impl Command {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Limit { stop, start } => {
                if let Some(v) = stop {
                    percent(*v)?;
                }
                if let Some(v) = start
                    && stop.is_none_or(|s| *v >= s)
                {
                    return Err(Error::InvalidLimit("start must be below stop".into()));
                }
            }
            Self::Discharge { target } => {
                percent(*target)?;
                if *target < 20 {
                    return Err(Error::InvalidLimit("discharge floor is 20%".into()));
                }
            }
            _ => {}
        }
        Ok(())
    }
}
pub fn percent(value: u8) -> Result<()> {
    if !(1..=100).contains(&value) {
        return Err(Error::InvalidLimit("expected 1..=100".into()));
    }
    Ok(())
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: u32,
    pub command: Command,
}
impl Request {
    pub fn new(command: Command) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            command,
        }
    }
    pub fn validate(&self) -> Result<()> {
        if self.version != PROTOCOL_VERSION {
            return Err(Error::ProtocolError("incompatible protocol version".into()));
        }
        self.command.validate()
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum Response {
    Status(Box<Snapshot>),
    Capabilities(Capabilities),
    Probe(Box<ProbeReport>),
    Config(Config),
    Ok,
    Error(Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_untrusted_requests() {
        assert!(
            Request::new(Command::Limit {
                stop: Some(0),
                start: None
            })
            .validate()
            .is_err()
        );
        assert!(
            Request::new(Command::Limit {
                stop: Some(80),
                start: Some(80)
            })
            .validate()
            .is_err()
        );
        assert!(
            Request::new(Command::Discharge { target: 10 })
                .validate()
                .is_err()
        );
        assert!(
            serde_json::from_str::<Command>(r#"{"command":"limit","stop":80,"shell":"rm"}"#)
                .is_err()
        );
    }
}
