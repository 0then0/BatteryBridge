use crate::*;
mod iokit;
pub mod power;
mod smc;
fn system_value(program: &str, args: &[&str]) -> String {
    std::process::Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_else(|| "unknown".into())
}
pub fn discover() -> Result<Platform> {
    let monitor = iokit::Monitor;
    let device = DeviceInfo {
        os: "macos".into(),
        os_version: system_value("/usr/bin/sw_vers", &["-productVersion"]),
        architecture: std::env::consts::ARCH.into(),
        vendor: "Apple".into(),
        model: iokit::model(),
    };
    let mut interfaces = BTreeMap::from([("IOKit".into(), "available".into())]);
    if cfg!(any(target_arch = "aarch64", target_arch = "x86_64")) {
        match smc::Smc::open() {
            Ok(smc) => {
                if cfg!(target_arch = "x86_64") {
                    interfaces.insert(
                        "SMC:BCLM".into(),
                        match smc.read(*b"BCLM", 1) {
                            Ok(value) if (50..=100).contains(&value[0]) => "available".into(),
                            Ok(_) => "unsupported: invalid BCLM threshold".into(),
                            Err(e) => e.to_string(),
                        },
                    );
                }
                for (key, len) in [(*b"CHTE", 4), (*b"CH0B", 1), (*b"CH0C", 1), (*b"CH0I", 1)] {
                    let result = smc.read(key, len);
                    interfaces.insert(
                        format!("SMC:{}", String::from_utf8_lossy(&key)),
                        match result {
                            Ok(_) => "available".into(),
                            Err(e) => e.to_string(),
                        },
                    );
                }
            }
            Err(e) => {
                interfaces.insert("AppleSMC".into(), e.to_string());
            }
        }
    }
    let report = resolve(Facts {
        device: device.clone(),
        batteries: monitor.batteries()?,
        interfaces,
    });
    Ok(Platform {
        monitor: Box::new(monitor),
        device,
        report,
    })
}
pub fn controller() -> Result<Box<dyn ChargeController>> {
    let platform = discover()?;
    batterybridge_core::policy::require(&platform.report.capabilities.charge_limit)?;
    Ok(Box::new(Controller {
        smc: smc::Smc::open()?,
        bclm: platform.device.architecture == "x86_64"
            && platform
                .report
                .interfaces
                .get("SMC:BCLM")
                .is_some_and(|value| value == "available"),
        caps: platform.report.capabilities,
    }))
}
struct Controller {
    smc: smc::Smc,
    caps: Capabilities,
    bclm: bool,
}
impl ChargeController for Controller {
    fn capabilities(&self) -> Capabilities {
        self.caps.clone()
    }
    fn apply(&mut self, start: Option<u8>, stop: Option<u8>) -> Result<()> {
        Command::Limit { start, stop }.validate()?;
        if self.bclm {
            return self.smc.set_stop(stop.unwrap_or(100));
        }
        if stop.is_none() {
            self.smc.gate(true)?;
        }
        Ok(())
    }
    fn charging(&mut self, enabled: bool) -> Result<()> {
        if self.bclm {
            return Ok(());
        }
        self.smc.gate(enabled)
    }
    fn force_discharge(&mut self, enabled: bool) -> Result<()> {
        batterybridge_core::policy::require(&self.caps.force_discharge)?;
        self.smc.discharge(enabled)
    }
    fn release(&mut self) -> Result<()> {
        if self.bclm {
            return self.smc.set_stop(100);
        }
        let discharge = if self.caps.force_discharge.available() {
            self.smc.discharge(false)
        } else {
            Ok(())
        };
        let gate = self.smc.gate(true);
        discharge.and(gate)
    }
}
