//! Shared application boundary: offline reads do not grant a charge controller.
use crate::*;

pub fn query(command: Command, send: impl FnOnce(Command) -> Result<Response>) -> Result<Response> {
    query_with(command, send, Platform::discover)
}
fn query_with(
    command: Command,
    send: impl FnOnce(Command) -> Result<Response>,
    discover: impl FnOnce() -> Result<Platform>,
) -> Result<Response> {
    command.validate()?;
    if matches!(command, Command::Probe) {
        return Ok(Response::Probe(Box::new(discover()?.report)));
    }
    match send(command.clone()) {
        Ok(response) => Ok(response),
        Err(error @ (Error::ServiceUnavailable(_) | Error::PermissionDenied(_)))
            if matches!(command, Command::Status | Command::Capabilities) =>
        {
            let platform = discover()?;
            if matches!(command, Command::Capabilities) {
                return Ok(Response::Capabilities(platform.report.capabilities));
            }
            Ok(Response::Status(Box::new(Snapshot {
                batteries: platform.monitor.batteries()?,
                device: platform.device.clone(),
                backend: platform.backend(),
                capabilities: platform.report.capabilities,
                config: None,
                top_up_active: false,
                discharge_target: None,
                last_error: Some(error),
            })))
        }
        Err(error) => Err(error),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Monitor;
    impl BatteryMonitor for Monitor {
        fn batteries(&self) -> Result<Vec<BatteryStatus>> {
            Ok(vec![BatteryStatus {
                id: "BAT0".into(),
                percent: Some(74),
                state: "charging".into(),
                health_percent: None,
                on_ac: Some(true),
                hardware_start: None,
                hardware_stop: None,
            }])
        }
    }
    fn platform() -> Result<Platform> {
        let facts: Facts = serde_json::from_value(
            serde_json::from_str::<serde_json::Value>(include_str!(
                "../../../tests/fixtures/linux-generic-threshold.json"
            ))
            .unwrap()["facts"]
                .clone(),
        )
        .unwrap();
        let device = facts.device.clone();
        Ok(Platform {
            device,
            monitor: Box::new(Monitor),
            report: resolve(facts),
        })
    }
    #[test]
    fn offline_status_reads_battery_without_inventing_a_policy() {
        let response = query_with(
            Command::Status,
            |_| Err(Error::ServiceUnavailable("offline".into())),
            platform,
        )
        .unwrap();
        let Response::Status(status) = response else {
            panic!("expected status");
        };
        assert_eq!(status.batteries[0].percent, Some(74));
        assert!(status.config.is_none());
        assert!(matches!(
            status.last_error,
            Some(Error::ServiceUnavailable(_))
        ));
    }
    #[test]
    fn writes_never_fall_back_or_retry_and_live_errors_are_preserved() {
        for command in [
            Command::Limit {
                stop: Some(80),
                start: None,
            },
            Command::TopUp,
            Command::Discharge { target: 80 },
        ] {
            assert!(
                query_with(
                    command,
                    |_| Err(Error::ServiceUnavailable("offline".into())),
                    || panic!("no local hardware fallback for writes")
                )
                .is_err()
            );
        }
        assert!(matches!(
            query_with(
                Command::Status,
                |_| Err(Error::HardwareError("sensor".into())),
                || panic!("do not hide live daemon errors")
            ),
            Err(Error::HardwareError(_))
        ));
    }
    #[test]
    fn probe_is_local_and_never_contacts_or_starts_a_daemon() {
        assert!(matches!(
            query_with(
                Command::Probe,
                |_| panic!("probe must remain local"),
                platform
            ),
            Ok(Response::Probe(_))
        ));
    }
}
