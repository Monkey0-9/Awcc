use awcc_core::error::AwccError;
use awcc_core::governor::ThermalGovernor;
use awcc_core::telemetry::FanTelemetry;

#[test]
fn test_fan_telemetry_serialization() {
    let fan = FanTelemetry {
        fan_index: 1,
        label: "CPU Fan".to_string(),
        requested_boost: 255,
        readback_boost: Some(255),
        current_rpm: Some(4800),
        reported_min_rpm: Some(0),
        reported_max_rpm: Some(4800),
        provider: "alienware_wmi".to_string(),
    };

    let json = serde_json::to_string(&fan).expect("Failed to serialize fan telemetry");
    assert!(json.contains("\"requested_boost\":255"));
    assert!(json.contains("\"current_rpm\":4800"));
    assert!(json.contains("\"reported_max_rpm\":4800"));

    let deserialized: FanTelemetry =
        serde_json::from_str(&json).expect("Failed to deserialize fan telemetry");
    assert_eq!(deserialized.requested_boost, 255);
    assert_eq!(deserialized.current_rpm, Some(4800));
    assert_eq!(deserialized.reported_max_rpm, Some(4800));
}

#[test]
fn test_error_formatting() {
    let err = AwccError::ReadbackMismatch {
        fan: 1,
        requested: 255,
        readback: 0,
    };
    assert_eq!(
        err.to_string(),
        "Boost verification failed for fan 1: requested 255, readback 0"
    );

    let err_prof = AwccError::InvalidProfile {
        requested: "ultra_turbo".to_string(),
        available: vec!["quiet".to_string(), "performance".to_string()],
    };
    assert_eq!(
        err_prof.to_string(),
        "Invalid platform profile 'ultra_turbo'. Available choices: [quiet, performance]"
    );
}

#[test]
fn test_thermal_governor_curve_and_hysteresis() {
    let mut gov = ThermalGovernor::new();
    assert_eq!(gov.last_boost(), 0);

    // Below 50.0°C: boost is 0
    assert_eq!(gov.compute_boost(35.0), 0);
    assert_eq!(gov.compute_boost(49.9), 0);

    // Ramp up at 85.0°C and above: instant max boost 255
    assert_eq!(gov.compute_boost(85.0), 255);
    assert_eq!(gov.last_boost(), 255);

    assert_eq!(gov.compute_boost(95.0), 255);

    // Instant temp drop from 95°C to 40°C:
    // Raw target is 0, but hysteresis limits decrease to at most 15 units per cycle
    let damped1 = gov.compute_boost(40.0);
    assert_eq!(damped1, 240); // 255 - 15

    let damped2 = gov.compute_boost(40.0);
    assert_eq!(damped2, 225); // 240 - 15

    // Sudden spike back to 85°C: instant ramp-up with no damping
    assert_eq!(gov.compute_boost(85.0), 255);
}

#[test]
fn test_performance_mode_boost_ladder_and_parsing() {
    use awcc_core::discovery::PerformanceMode;

    // Test exact 20% boost ladder values
    assert_eq!(PerformanceMode::Battery.target_boost(), 51);
    assert_eq!(PerformanceMode::Battery.boost_percentage(), 20);
    assert_eq!(PerformanceMode::Battery.acpi_profile(), "quiet");

    assert_eq!(PerformanceMode::Quiet.target_boost(), 102);
    assert_eq!(PerformanceMode::Quiet.boost_percentage(), 40);
    assert_eq!(PerformanceMode::Quiet.acpi_profile(), "quiet");

    assert_eq!(PerformanceMode::Balanced.target_boost(), 153);
    assert_eq!(PerformanceMode::Balanced.boost_percentage(), 60);
    assert_eq!(PerformanceMode::Balanced.acpi_profile(), "balanced");

    assert_eq!(PerformanceMode::Performance.target_boost(), 204);
    assert_eq!(PerformanceMode::Performance.boost_percentage(), 80);
    assert_eq!(PerformanceMode::Performance.acpi_profile(), "performance");

    assert_eq!(PerformanceMode::Ultra.target_boost(), 255);
    assert_eq!(PerformanceMode::Ultra.boost_percentage(), 100);
    assert_eq!(PerformanceMode::Ultra.acpi_profile(), "performance");

    // Test from_str parsing
    assert_eq!(
        PerformanceMode::from_str("battery"),
        Some(PerformanceMode::Battery)
    );
    assert_eq!(
        PerformanceMode::from_str("BAT"),
        Some(PerformanceMode::Battery)
    );
    assert_eq!(
        PerformanceMode::from_str("powersave"),
        Some(PerformanceMode::Battery)
    );
    assert_eq!(
        PerformanceMode::from_str("quiet"),
        Some(PerformanceMode::Quiet)
    );
    assert_eq!(
        PerformanceMode::from_str("silent"),
        Some(PerformanceMode::Quiet)
    );
    assert_eq!(
        PerformanceMode::from_str("balanced"),
        Some(PerformanceMode::Balanced)
    );
    assert_eq!(
        PerformanceMode::from_str("balance"),
        Some(PerformanceMode::Balanced)
    );
    assert_eq!(
        PerformanceMode::from_str("performance"),
        Some(PerformanceMode::Performance)
    );
    assert_eq!(
        PerformanceMode::from_str("high"),
        Some(PerformanceMode::Performance)
    );
    assert_eq!(
        PerformanceMode::from_str("perf"),
        Some(PerformanceMode::Performance)
    );
    assert_eq!(
        PerformanceMode::from_str("ultra"),
        Some(PerformanceMode::Ultra)
    );
    assert_eq!(
        PerformanceMode::from_str("gmode"),
        Some(PerformanceMode::Ultra)
    );
    assert_eq!(
        PerformanceMode::from_str("max"),
        Some(PerformanceMode::Ultra)
    );
    assert_eq!(PerformanceMode::from_str("invalid_mode"), None);
}
