//! Battery-care preferences, schedules and observable temporary operation state.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct BatteryTelemetry {
    /// Tenths of a degree Celsius; not an ambient or CPU temperature.
    pub temperature_deci_c: Option<i16>,
    pub voltage_mv: Option<u32>,
    /// Positive charging, negative discharging; absent when direction is unknown.
    pub current_ma: Option<i32>,
    pub power_mw: Option<i32>,
    pub cycle_count: Option<u32>,
    pub raw_percent: Option<u8>,
    pub remaining_minutes: Option<u32>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LedMode {
    #[default]
    System,
    Automatic,
    Off,
    Green,
    Orange,
    SlowBlinkOrange,
    FastBlinkOrange,
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Repeat {
    #[default]
    Once,
    Daily,
    Weekdays,
    Weekly,
    Biweekly,
    Monthly,
    Yearly,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum ScheduleAction {
    Limit { stop: u8, start: Option<u8> },
    TopUp,
    ChargeTo { target: u8 },
    Discharge { target: u8 },
    Hold,
    Pause,
    Resume,
    Calibrate,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Schedule {
    pub id: u32,
    pub enabled: bool,
    /// Unix seconds of the next occurrence. Repetitions follow the service's local timezone.
    pub next_at: i64,
    /// Original occurrence, retained to preserve month-end dates and wall-clock time.
    #[serde(default)]
    pub anchor_at: Option<i64>,
    pub repeat: Repeat,
    pub action: ScheduleAction,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct CareSettings {
    pub heat_limit_c: Option<u8>,
    pub automatic_discharge: bool,
    pub prevent_sleep: bool,
    pub led: LedMode,
    pub schedules: Vec<Schedule>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationPhase {
    Charging,
    Discharging,
    Recharging,
    Resting,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Calibration {
    pub phase: CalibrationPhase,
    pub started_at: i64,
    pub phase_at: i64,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct RuntimeStatus {
    pub paused: bool,
    pub holding: bool,
    pub charge_target: Option<u8>,
    pub calibration: Option<Calibration>,
    pub automatic_discharge_paused: bool,
    pub heat_paused: bool,
    pub temperature_unavailable: bool,
    pub sleep_prevented: bool,
}
