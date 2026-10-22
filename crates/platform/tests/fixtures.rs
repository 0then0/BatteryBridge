use batterybridge_platform::{Facts, resolve};
use batterybridge_protocol::*;
use serde::Deserialize;
use std::collections::BTreeMap;
#[derive(Deserialize)]
struct Fixture {
    facts: Facts,
    expected: Expected,
}
#[derive(Deserialize)]
struct Expected {
    backend: String,
    selected: bool,
    capabilities: BTreeMap<String, CapabilityState>,
    battery: BatteryStatus,
}
#[test]
fn compatibility_golden_fixtures() {
    for input in [
        include_str!("../../../tests/fixtures/linux-thinkpad.json"),
        include_str!("../../../tests/fixtures/linux-generic-threshold.json"),
        include_str!("../../../tests/fixtures/linux-no-threshold.json"),
        include_str!("../../../tests/fixtures/windows-lenovo.json"),
        include_str!("../../../tests/fixtures/windows-unknown-vendor.json"),
        include_str!("../../../tests/fixtures/macos-apple-silicon.json"),
    ] {
        let fixture: Fixture = serde_json::from_str(input).unwrap();
        assert_eq!(fixture.facts.batteries[0], fixture.expected.battery);
        let report = resolve(fixture.facts);
        assert_eq!(report.candidates[0].name, fixture.expected.backend);
        assert_eq!(report.candidates[0].selected, fixture.expected.selected);
        let caps = serde_json::to_value(report.capabilities).unwrap();
        for (name, state) in fixture.expected.capabilities {
            assert_eq!(
                caps[&name]["state"],
                serde_json::to_value(state).unwrap(),
                "{name}"
            );
        }
    }
}
#[test]
fn vendor_or_key_name_alone_never_selects_controller() {
    let mut f: Fixture =
        serde_json::from_str(include_str!("../../../tests/fixtures/windows-lenovo.json")).unwrap();
    f.facts.interfaces.remove("WMI:capdata:0x03010001");
    assert!(!resolve(f.facts).capabilities.charge_limit.available());
    let mut f: Fixture = serde_json::from_str(include_str!(
        "../../../tests/fixtures/macos-apple-silicon.json"
    ))
    .unwrap();
    f.facts.device.architecture = "x86_64".into();
    assert!(!resolve(f.facts).capabilities.charge_limit.available());
}
#[test]
fn report_excludes_personal_identifiers() {
    let f: Fixture =
        serde_json::from_str(include_str!("../../../tests/fixtures/windows-lenovo.json")).unwrap();
    let report = serde_json::to_string(&resolve(f.facts))
        .unwrap()
        .to_lowercase();
    for forbidden in [
        "serialnumber",
        "hostname",
        "username",
        "mac_address",
        "ip_address",
        "uuid",
        "__path",
    ] {
        assert!(!report.contains(forbidden), "{forbidden}");
    }
}

#[test]
fn inaccessible_smc_is_unknown_instead_of_a_false_hardware_rejection() {
    let mut f: Fixture = serde_json::from_str(include_str!(
        "../../../tests/fixtures/macos-apple-silicon.json"
    ))
    .unwrap();
    f.facts.interfaces.clear();
    f.facts
        .interfaces
        .insert("AppleSMC".into(), "permission denied: IOKit".into());
    let report = resolve(f.facts);
    assert_eq!(
        report.capabilities.charge_limit.state,
        CapabilityState::Unknown
    );
    assert!(!report.capabilities.charge_limit.available());
}

#[test]
fn a_desktop_with_readable_keys_is_not_a_controllable_battery() {
    let mut f: Fixture = serde_json::from_str(include_str!(
        "../../../tests/fixtures/macos-apple-silicon.json"
    ))
    .unwrap();
    f.facts.batteries.clear();
    let report = resolve(f.facts);
    assert!(!report.candidates[0].selected);
    assert!(!report.capabilities.charge_limit.available());
    assert_eq!(report.candidates[0].reason, "no present battery");
}

#[test]
fn multi_battery_lenovo_is_rejected_by_the_resolver_itself() {
    let mut f: Fixture =
        serde_json::from_str(include_str!("../../../tests/fixtures/windows-lenovo.json")).unwrap();
    let mut second = f.facts.batteries[0].clone();
    second.id = "BAT1".into();
    f.facts.batteries.push(second);
    let report = resolve(f.facts);
    assert!(!report.candidates[0].selected);
    assert!(!report.capabilities.charge_limit.available());
}
