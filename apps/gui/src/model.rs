//! UI policy and editable draft. The daemon remains the source of truth.
use batterybridge_protocol::{CareSettings, Command, Error, Snapshot};

#[derive(Debug)]
pub struct Model {
    pub snapshot: Option<Snapshot>,
    pub stop: u8,
    pub start: u8,
    pub use_start: bool,
    pub edited: bool,
    pub error: Option<String>,
    pub updated: bool,
    pub care: CareSettings,
    pub care_base: CareSettings,
    pub care_edited: bool,
    pub raw_percent: bool,
    pub history: std::collections::VecDeque<(i64, u8)>,
}
impl Default for Model {
    fn default() -> Self {
        Self {
            snapshot: None,
            stop: 80,
            start: 75,
            use_start: false,
            edited: false,
            error: None,
            updated: false,
            care: CareSettings::default(),
            care_base: CareSettings::default(),
            care_edited: false,
            raw_percent: false,
            history: std::collections::VecDeque::new(),
        }
    }
}
impl Model {
    pub fn mark_edited(&mut self) {
        self.edited = true;
        self.error = None;
    }
    pub fn read_only(&self) -> bool {
        self.snapshot.as_ref().is_none_or(|s| s.config.is_none())
    }
    pub fn accept(&mut self, snapshot: Snapshot) {
        if !self.edited {
            self.stop = snapshot
                .config
                .as_ref()
                .and_then(|c| c.charge_limit)
                .unwrap_or_else(|| {
                    snapshot
                        .capabilities
                        .allowed_limits
                        .as_ref()
                        .and_then(|limits| limits.first().copied())
                        .unwrap_or(80)
                });
            self.use_start = snapshot
                .config
                .as_ref()
                .is_some_and(|c| c.start_threshold.enabled);
            self.start = snapshot
                .config
                .as_ref()
                .map_or(75, |c| c.start_threshold.value);
        }
        if !self.care_edited {
            self.care = snapshot
                .config
                .as_ref()
                .map(|c| c.care.clone())
                .unwrap_or_default();
            self.care_base = self.care.clone();
        }
        if let Some(percent) = snapshot.batteries.first().and_then(|b| b.percent) {
            let now = chrono::Utc::now().timestamp();
            if self.history.back().is_none_or(|(at, _)| now > *at) {
                self.history.push_back((now, percent));
                if self.history.len() > 240 {
                    self.history.pop_front();
                }
            }
        }
        self.snapshot = Some(snapshot);
        self.error = None;
    }
    pub fn limit_command(&self) -> Result<Command, Error> {
        if self.read_only() {
            return Err(Error::ServiceUnavailable(
                "charging settings are unavailable".into(),
            ));
        }
        let command = Command::Limit {
            stop: Some(self.stop),
            start: self.use_start.then_some(self.start),
        };
        command.validate()?;
        if let Some(snapshot) = &self.snapshot {
            if !snapshot.capabilities.charge_limit.available() {
                return Err(Error::Unsupported("charge limit".into()));
            }
            if self.use_start && !snapshot.capabilities.start_threshold.available() {
                return Err(Error::Unsupported("start threshold".into()));
            }
            if snapshot
                .capabilities
                .allowed_limits
                .as_ref()
                .is_some_and(|limits| !limits.contains(&self.stop))
            {
                return Err(Error::InvalidLimit("unsupported device threshold".into()));
            }
        }
        Ok(command)
    }
}

pub fn charging_above_limit(snapshot: &Snapshot) -> bool {
    snapshot
        .config
        .as_ref()
        .and_then(|config| config.charge_limit)
        .is_some_and(|stop| {
            !snapshot.runtime.paused
                && snapshot.runtime.charge_target.is_none()
                && snapshot.runtime.calibration.is_none()
                && !snapshot.top_up_active
                && snapshot.discharge_target.is_none()
                && snapshot.batteries.first().is_some_and(|battery| {
                    battery.percent.is_some_and(|p| p > stop)
                        && battery.on_ac == Some(true)
                        && battery.state.eq_ignore_ascii_case("charging")
                })
        })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use batterybridge_protocol::*;
    pub fn fixture() -> Snapshot {
        serde_json::from_str(include_str!("../tests/status.json")).unwrap()
    }
    #[test]
    fn status_refresh_preserves_edits_and_offline_policy_is_unavailable() {
        let mut model = Model::default();
        model.accept(fixture());
        assert_eq!(model.stop, 80);
        model.edited = true;
        model.stop = 90;
        model.accept(fixture());
        assert_eq!(model.stop, 90);
        let mut offline = fixture();
        offline.config = None;
        model.accept(offline);
        assert!(model.read_only());
    }
    #[test]
    fn saved_intent_does_not_hide_observed_charging_above_limit() {
        let mut snapshot = fixture();
        assert!(charging_above_limit(&snapshot));
        snapshot.top_up_active = true;
        assert!(!charging_above_limit(&snapshot));
        snapshot.top_up_active = false;
        snapshot.config = None;
        assert!(!charging_above_limit(&snapshot));
    }
    #[test]
    fn fixed_hardware_thresholds_and_start_capability_are_validated() {
        let mut snapshot = fixture();
        snapshot.capabilities.charge_limit = Capability::experimental("fixed");
        snapshot.capabilities.allowed_limits = Some(vec![80]);
        snapshot.capabilities.start_threshold = Capability::unsupported("not available");
        let mut model = Model::default();
        model.accept(snapshot);
        assert!(model.limit_command().is_ok());
        model.stop = 85;
        assert!(model.limit_command().is_err());
        model.stop = 80;
        model.use_start = true;
        assert!(model.limit_command().is_err());
    }
    #[test]
    fn refresh_preserves_unsaved_care_and_the_original_compare_value() {
        let mut model = Model::default();
        model.accept(fixture());
        let original = model.care_base.clone();
        model.care.heat_limit_c = Some(35);
        model.care_edited = true;
        let mut updated = fixture();
        updated.config.as_mut().unwrap().care.prevent_sleep = true;
        model.accept(updated);
        assert_eq!(model.care.heat_limit_c, Some(35));
        assert_eq!(model.care_base, original);
        model.care_edited = false;
        model.accept(fixture());
        assert_eq!(model.care, model.care_base);
    }
}
