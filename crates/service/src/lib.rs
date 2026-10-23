//! Service-side command handling, charging-policy reconciliation and lifecycle management.

#![forbid(unsafe_code)]
use batterybridge_core::{policy::Policy, *};
use batterybridge_platform::Platform;
use std::path::PathBuf;

pub struct Service {
    platform: Platform,
    controller: Option<Box<dyn ChargeController>>,
    policy: Policy,
    path: PathBuf,
    last_error: Option<Error>,
    sleep_error: Option<Error>,
}
impl Service {
    pub fn new(platform: Platform, path: PathBuf) -> Result<Self> {
        let policy = Policy::new(config::load(&path)?)?;
        let (controller, last_error) = match platform.controller() {
            Ok(c) => (Some(c), None),
            Err(e) => (None, Some(e)),
        };
        Ok(Self {
            platform,
            controller,
            policy,
            path,
            last_error,
            sleep_error: None,
        })
    }
    fn apply_policy(&mut self, next: &mut Policy) -> Result<()> {
        let controller = self
            .controller
            .as_mut()
            .ok_or_else(|| Error::Unsupported("no active charge controller; run probe".into()))?;
        let batteries = self.platform.monitor.batteries()?;
        let Some(battery) = batteries.first() else {
            controller.release()?;
            return Err(Error::BackendUnavailable("battery missing".into()));
        };
        next.tick(controller.as_mut(), battery)
    }
    pub fn tick(&mut self) {
        if self.controller.is_none() {
            return;
        }
        let mut next = self.policy.clone();
        match self.apply_policy(&mut next) {
            Ok(()) => {
                self.policy = next;
                self.last_error = None;
            }
            Err(e) => {
                if let Some(c) = &mut self.controller {
                    let _ = c.release();
                }
                // Ephemeral modes are cancelled on errors, never restarted blindly.
                self.policy.top_up = false;
                self.policy.discharge_target = None;
                if self.last_error.as_ref() != Some(&e) {
                    tracing::warn!(error=%e,"policy enforcement failed");
                }
                self.last_error = Some(e);
            }
        }
    }
    pub fn prepare_sleep(&mut self) -> Result<()> {
        let result = match &mut self.controller {
            Some(controller) => self.policy.prepare_sleep(controller.as_mut()),
            None => Ok(()),
        };
        // A later successful awake tick does not prove that sleep preparation worked.
        self.sleep_error = result
            .as_ref()
            .err()
            .map(|error| Error::PlatformError(format!("Sleep preparation failed: {error}")));
        result
    }
    pub fn shutdown(&mut self) -> Result<()> {
        if let Some(controller) = &mut self.controller {
            controller.release()?;
        }
        Ok(())
    }
    pub fn handle(&mut self, request: Request) -> Response {
        if let Err(e) = request.validate() {
            return Response::Error(e);
        }
        match self.execute(request.command) {
            Ok(v) => v,
            Err(e) => Response::Error(e),
        }
    }
    fn execute(&mut self, command: Command) -> Result<Response> {
        match command {
            // Monitoring-only installations have no hardware policy to release.
            // A saved active limit still requires a working controller, so removal
            // cannot silently leave previously configured hardware inhibition.
            Command::Limit {
                stop: None,
                start: None,
            } if self.controller.is_none()
                && self.policy.config.charge_limit.is_none()
                && !self.policy.top_up
                && self.policy.discharge_target.is_none() =>
            {
                Ok(Response::Ok)
            }
            Command::Status => Ok(Response::Status(Box::new(Snapshot {
                batteries: self.platform.monitor.batteries()?,
                device: self.platform.device.clone(),
                backend: self.platform.backend(),
                capabilities: self.platform.report.capabilities.clone(),
                config: Some(self.policy.config.clone()),
                top_up_active: self.policy.top_up,
                discharge_target: self.policy.discharge_target,
                last_error: self.last_error.clone().or_else(|| self.sleep_error.clone()),
            }))),
            Command::Capabilities => Ok(Response::Capabilities(
                self.platform.report.capabilities.clone(),
            )),
            Command::Probe => Ok(Response::Probe(Box::new(Platform::discover()?.report))),
            Command::Config => Ok(Response::Config(self.policy.config.clone())),
            command => {
                let mut next = self.policy.clone();
                let caps = &self.platform.report.capabilities;
                let persist = matches!(command, Command::Limit { .. });
                match command {
                    Command::Limit { stop, start } => next.set_limit(stop, start, caps)?,
                    Command::TopUp => next.top_up(caps)?,
                    Command::Discharge { target } => next.discharge(target, caps)?,
                    _ => unreachable!(),
                }
                let apply = self.apply_policy(&mut next);
                if let Err(e) = apply {
                    if let Some(c) = &mut self.controller {
                        let _ = c.release();
                    }
                    let mut previous = self.policy.clone();
                    let _ = self.apply_policy(&mut previous);
                    return Err(e);
                }
                if persist && let Err(e) = config::save(&self.path, &next.config) {
                    let mut previous = self.policy.clone();
                    if self.apply_policy(&mut previous).is_err()
                        && let Some(c) = &mut self.controller
                    {
                        let _ = c.release();
                    }
                    return Err(e);
                }
                self.policy = next;
                self.last_error = None;
                Ok(Response::Ok)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::RefCell,
        collections::BTreeMap,
        rc::Rc,
        sync::atomic::{AtomicU64, Ordering},
    };
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    struct Monitor;
    impl BatteryMonitor for Monitor {
        fn batteries(&self) -> Result<Vec<BatteryStatus>> {
            Ok(vec![BatteryStatus {
                id: "BAT0".into(),
                percent: Some(74),
                state: "charging".into(),
                on_ac: Some(true),
                health_percent: None,
                hardware_start: None,
                hardware_stop: None,
            }])
        }
    }
    struct Controller(Rc<RefCell<Option<u8>>>);
    impl ChargeController for Controller {
        fn capabilities(&self) -> Capabilities {
            let mut c = Capabilities::unsupported("fixture");
            c.charge_limit = Capability::supported("fixture");
            c.stop_threshold = c.charge_limit.clone();
            c.temporary_full_charge = c.charge_limit.clone();
            c
        }
        fn apply(&mut self, _: Option<u8>, stop: Option<u8>) -> Result<()> {
            *self.0.borrow_mut() = stop;
            Ok(())
        }
    }
    fn service(path: PathBuf, state: Rc<RefCell<Option<u8>>>) -> Service {
        let device = DeviceInfo {
            os: "test".into(),
            os_version: "test".into(),
            architecture: "fixture".into(),
            vendor: "fixture".into(),
            model: "fixture".into(),
        };
        let controller = Controller(state);
        let report = ProbeReport {
            schema_version: 1,
            device: device.clone(),
            battery_ids: vec!["BAT0".into()],
            interfaces: BTreeMap::new(),
            candidates: vec![],
            capabilities: controller.capabilities(),
        };
        Service {
            platform: Platform {
                monitor: Box::new(Monitor),
                device,
                report,
            },
            controller: Some(Box::new(controller)),
            policy: Policy::new(config::load(&path).unwrap()).unwrap(),
            path,
            last_error: None,
            sleep_error: None,
        }
    }
    fn dir() -> PathBuf {
        std::env::temp_dir().join(format!(
            "batterybridge-service-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ))
    }
    #[test]
    fn sleep_failure_remains_visible_after_successful_awake_reconciliation() {
        struct SleepFailure;
        impl ChargeController for SleepFailure {
            fn capabilities(&self) -> Capabilities {
                Controller(Rc::new(RefCell::new(None))).capabilities()
            }
            fn apply(&mut self, _: Option<u8>, _: Option<u8>) -> Result<()> {
                Ok(())
            }
            fn charging(&mut self, enabled: bool) -> Result<()> {
                if enabled {
                    Ok(())
                } else {
                    Err(Error::PlatformError("sleep gate refused".into()))
                }
            }
        }
        let mut s = service(dir().join("config.toml"), Rc::new(RefCell::new(None)));
        s.controller = Some(Box::new(SleepFailure));
        s.policy.config.charge_limit = Some(80);
        assert!(s.prepare_sleep().is_err());
        s.tick(); // 74% allows charging, but cannot validate a previous sleep transition.
        let Response::Status(snapshot) = s.handle(Request::new(Command::Status)) else {
            panic!("status");
        };
        assert_eq!(
            snapshot.last_error,
            Some(Error::PlatformError(
                "Sleep preparation failed: platform error: sleep gate refused".into()
            ))
        );
        s.controller = Some(Box::new(Controller(Rc::new(RefCell::new(None)))));
        s.prepare_sleep().unwrap();
        let Response::Status(snapshot) = s.handle(Request::new(Command::Status)) else {
            panic!("status");
        };
        assert_eq!(snapshot.last_error, None);
    }
    #[test]
    fn monitoring_only_uninstall_accepts_off_but_does_not_hide_an_active_policy() {
        let mut s = service(dir().join("config.toml"), Rc::new(RefCell::new(None)));
        s.controller = None;
        assert!(matches!(
            s.handle(Request::new(Command::Limit {
                stop: None,
                start: None
            })),
            Response::Ok
        ));
        s.policy.config.charge_limit = Some(80);
        assert!(matches!(
            s.handle(Request::new(Command::Limit {
                stop: None,
                start: None
            })),
            Response::Error(_)
        ));
        assert_eq!(s.policy.config.charge_limit, Some(80));
    }
    #[test]
    fn persists_policy_and_clears_ephemeral_mode_on_restart() {
        let root = dir();
        let path = root.join("config.toml");
        let state = Rc::new(RefCell::new(None));
        let mut s = service(path.clone(), state.clone());
        assert!(matches!(
            s.handle(Request::new(Command::Limit {
                stop: Some(80),
                start: None
            })),
            Response::Ok
        ));
        assert_eq!(*state.borrow(), Some(80));
        assert!(matches!(
            s.handle(Request::new(Command::TopUp)),
            Response::Ok
        ));
        assert_eq!(*state.borrow(), None);
        let mut restored = service(path, state.clone());
        restored.tick();
        assert_eq!(*state.borrow(), Some(80));
        assert!(!restored.policy.top_up);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn rejects_start_threshold_and_rolls_back_persistence_error() {
        let root = dir();
        let path = root.join("config.toml");
        let state = Rc::new(RefCell::new(None));
        let mut s = service(path.clone(), state.clone());
        assert!(matches!(
            s.handle(Request::new(Command::Limit {
                stop: Some(80),
                start: Some(75)
            })),
            Response::Error(Error::Unsupported(_))
        ));
        assert_eq!(*state.borrow(), None);
        std::fs::create_dir_all(&path).unwrap();
        assert!(matches!(
            s.handle(Request::new(Command::Limit {
                stop: Some(80),
                start: None
            })),
            Response::Error(_)
        ));
        assert_eq!(*state.borrow(), None);
        assert_eq!(s.policy.config.charge_limit, None);
        std::fs::remove_dir_all(root).unwrap();
    }
}
