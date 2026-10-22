use crate::*;

#[derive(Debug, Default, Clone)]
pub struct Policy {
    pub config: Config,
    pub top_up: bool,
    pub discharge_target: Option<u8>,
    inhibited: bool,
}
impl Policy {
    pub fn new(config: Config) -> Result<Self> {
        config::validate(&config)?;
        Ok(Self {
            config,
            ..Self::default()
        })
    }
    pub fn validate_for(&self, caps: &Capabilities) -> Result<()> {
        config::validate(&self.config)?;
        if let Some(stop) = self.config.charge_limit {
            require(&caps.charge_limit)?;
            if caps
                .allowed_limits
                .as_ref()
                .is_some_and(|v| !v.contains(&stop))
            {
                return Err(Error::Unsupported(format!(
                    "allowed charge limits: {:?}",
                    caps.allowed_limits
                )));
            }
        }
        if self.config.start_threshold.enabled {
            require(&caps.start_threshold)?;
        }
        Ok(())
    }
    pub fn set_limit(
        &mut self,
        stop: Option<u8>,
        start: Option<u8>,
        caps: &Capabilities,
    ) -> Result<()> {
        require(&caps.charge_limit)?;
        let next = Config {
            version: 1,
            charge_limit: stop,
            start_threshold: StartThreshold {
                enabled: start.is_some(),
                value: start.unwrap_or(75),
            },
        };
        let candidate = Self::new(next)?;
        candidate.validate_for(caps)?;
        *self = candidate;
        Ok(())
    }
    pub fn top_up(&mut self, caps: &Capabilities) -> Result<()> {
        require(&caps.temporary_full_charge)?;
        if self.config.charge_limit.is_none() {
            return Err(Error::ConfigurationError(
                "set a charge limit before top-up".into(),
            ));
        }
        self.top_up = true;
        self.discharge_target = None;
        Ok(())
    }
    pub fn discharge(&mut self, target: u8, caps: &Capabilities) -> Result<()> {
        Command::Discharge { target }.validate()?;
        require(&caps.force_discharge)?;
        self.top_up = false;
        self.discharge_target = Some(target);
        Ok(())
    }
    pub fn tick(
        &mut self,
        controller: &mut dyn ChargeController,
        battery: &BatteryStatus,
    ) -> Result<()> {
        require(&controller.capabilities().charge_limit)?;
        self.validate_for(&controller.capabilities())?;
        let Some(percent) = battery.percent else {
            // Never leave adapter isolation active after losing the sensor.
            if self.discharge_target.take().is_some() {
                controller.force_discharge(false)?;
            }
            return Err(Error::BackendUnavailable(
                "battery percentage unavailable".into(),
            ));
        };
        if self.top_up && (percent == 100 || battery.state == "full") {
            self.top_up = false;
        }
        if let Some(target) = self.discharge_target {
            if percent <= target {
                self.discharge_target = None;
                controller.force_discharge(false)?;
            } else {
                controller.force_discharge(true)?;
                return Ok(());
            }
        } else if controller.capabilities().force_discharge.available() {
            controller.force_discharge(false)?;
        }
        if self.top_up || self.config.charge_limit.is_none() {
            controller.apply(None, None)?;
            controller.charging(true)?;
            self.inhibited = false;
            return Ok(());
        }
        let stop = self.config.charge_limit.expect("checked limit");
        let start = if self.config.start_threshold.enabled {
            self.config.start_threshold.value
        } else {
            stop.saturating_sub(5)
        };
        controller.apply(
            self.config.start_threshold.enabled.then_some(start),
            Some(stop),
        )?;
        if percent >= stop {
            self.inhibited = true;
        } else if percent <= start {
            self.inhibited = false;
        }
        controller.charging(!self.inhibited)
    }
}
pub fn require(cap: &Capability) -> Result<()> {
    if !cap.available() {
        return Err(Error::Unsupported(cap.reason.clone()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fake {
        enabled: bool,
        stop: Option<u8>,
        discharge: bool,
    }
    impl ChargeController for Fake {
        fn capabilities(&self) -> Capabilities {
            let mut c = Capabilities::unsupported("test");
            c.charge_limit = Capability::supported("test");
            c.start_threshold = c.charge_limit.clone();
            c.temporary_full_charge = c.charge_limit.clone();
            c.force_discharge = c.charge_limit.clone();
            c
        }
        fn apply(&mut self, _: Option<u8>, stop: Option<u8>) -> Result<()> {
            self.stop = stop;
            Ok(())
        }
        fn charging(&mut self, v: bool) -> Result<()> {
            self.enabled = v;
            Ok(())
        }
        fn force_discharge(&mut self, v: bool) -> Result<()> {
            self.discharge = v;
            Ok(())
        }
    }
    fn battery(p: u8) -> BatteryStatus {
        BatteryStatus {
            id: "BAT0".into(),
            percent: Some(p),
            state: "charging".into(),
            health_percent: None,
            on_ac: Some(true),
            hardware_start: None,
            hardware_stop: None,
        }
    }
    #[test]
    fn hysteresis_top_up_and_restore() {
        let mut f = Fake {
            enabled: true,
            stop: None,
            discharge: false,
        };
        let mut p = Policy::default();
        p.set_limit(Some(80), Some(75), &f.capabilities()).unwrap();
        p.tick(&mut f, &battery(80)).unwrap();
        assert!(!f.enabled);
        p.tick(&mut f, &battery(79)).unwrap();
        assert!(!f.enabled);
        p.tick(&mut f, &battery(75)).unwrap();
        assert!(f.enabled);
        p.top_up(&f.capabilities()).unwrap();
        p.tick(&mut f, &battery(90)).unwrap();
        assert_eq!(f.stop, None);
        p.tick(&mut f, &battery(100)).unwrap();
        assert_eq!(f.stop, Some(80));
        assert!(!p.top_up);
        assert_eq!(p.config.charge_limit, Some(80));
    }
    #[test]
    fn discharge_stops_at_target_and_sensor_loss() {
        let mut f = Fake {
            enabled: true,
            stop: None,
            discharge: false,
        };
        let mut p = Policy::default();
        p.discharge(80, &f.capabilities()).unwrap();
        p.tick(&mut f, &battery(90)).unwrap();
        assert!(f.discharge);
        p.tick(&mut f, &battery(80)).unwrap();
        assert!(!f.discharge);
        p.discharge(80, &f.capabilities()).unwrap();
        p.tick(&mut f, &battery(90)).unwrap();
        let mut b = battery(90);
        b.percent = None;
        assert!(p.tick(&mut f, &b).is_err());
        assert!(!f.discharge);
    }
    #[test]
    fn changing_or_disabling_limit_cancels_ephemeral_overrides() {
        let mut p = Policy::default();
        let f = Fake {
            enabled: true,
            stop: None,
            discharge: false,
        };
        let caps = f.capabilities();
        p.set_limit(Some(80), None, &caps).unwrap();
        p.top_up(&caps).unwrap();
        p.set_limit(None, None, &caps).unwrap();
        assert!(!p.top_up);
        assert_eq!(p.config.charge_limit, None);
        p.discharge(80, &caps).unwrap();
        p.set_limit(Some(85), None, &caps).unwrap();
        assert_eq!(p.discharge_target, None);
    }
    #[test]
    fn unsupported_controller_is_never_written_even_for_default_policy() {
        struct Unsupported(bool);
        impl ChargeController for Unsupported {
            fn capabilities(&self) -> Capabilities {
                Capabilities::unsupported("fixture")
            }
            fn apply(&mut self, _: Option<u8>, _: Option<u8>) -> Result<()> {
                self.0 = true;
                Ok(())
            }
        }
        let mut controller = Unsupported(false);
        assert!(
            Policy::default()
                .tick(&mut controller, &battery(74))
                .is_err()
        );
        assert!(!controller.0);
    }
}
