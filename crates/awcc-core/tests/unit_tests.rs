use awcc_core::telemetry::FanTelemetry;
use awcc_core::error::AwccError;

#[test]
fn test_fan_telemetry_serialization() {
    let fan = FanTelemetry {
        fan_index: 1,
        label: "Fan 1 (CPU)".to_string(),
        requested_boost: 255,
        readback_boost: 255,
        current_rpm: 3500,
        reported_max_rpm: Some(3600),
    };

    let json = serde_json::to_string(&fan).expect("Failed to serialize fan telemetry");
    assert!(json.contains("\"requested_boost\":255"));
    assert!(json.contains("\"current_rpm\":3500"));

    let deserialized: FanTelemetry = serde_json::from_str(&json).expect("Failed to deserialize fan telemetry");
    assert_eq!(deserialized.requested_boost, 255);
    assert_eq!(deserialized.current_rpm, 3500);
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
}
