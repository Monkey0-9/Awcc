use awcc_core::error::AwccError;
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
