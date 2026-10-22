//! Read-only, explicit opt-in. Write validation is a supervised manual procedure.
#[test]
#[ignore = "requires a real laptop; see docs/hardware.md"]
fn real_device_read_and_probe() {
    assert_eq!(std::env::var("BB_HARDWARE_TEST").as_deref(), Ok("1"));
    let platform = batterybridge_platform::Platform::discover().unwrap();
    let batteries = platform.monitor.batteries().unwrap();
    assert!(
        !batteries.is_empty(),
        "run on a laptop with a present battery"
    );
    for b in batteries {
        assert!(b.percent.is_none_or(|v| v <= 100));
    }
    assert_eq!(platform.report.schema_version, 1);
}
