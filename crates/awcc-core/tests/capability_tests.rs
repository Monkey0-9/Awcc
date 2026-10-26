use awcc_core::capabilities::*;
use awcc_core::error::AwccError;

#[test]
fn test_capability_taxonomy_defaults_and_serialization() {
    let caps = HardwareCapabilities {
        vendor: HardwareVendor::Dell,
        vendor_name: "Dell Inc.".to_string(),
        product_name: "Dell G15 5530".to_string(),
        product: "Dell G15 5530".to_string(),
        bios_version: "1.34.0".to_string(),
        chassis: ChassisType::Laptop,
        verification_tier: VerificationTier::PhysicallyVerified,
        tier: VerificationTier::PhysicallyVerified,
        platform_profile: PlatformProfileCapability::Supported {
            provider: "alienware-wmi".to_string(),
            current: "performance".to_string(),
            choices: vec![
                "quiet".to_string(),
                "balanced".to_string(),
                "performance".to_string(),
            ],
        },
        fan_control: FanControlCapability::BoostOffset { max_boost: 255 },
        cpu_governor: CpuGovernorCapability::Supported {
            current_governor: "powersave".to_string(),
            available_governors: vec!["performance".to_string(), "powersave".to_string()],
            current_epp: Some("balance_performance".to_string()),
            available_epp: vec!["performance".to_string(), "balance_performance".to_string()],
        },
        gpu_telemetry: GpuTelemetryCapability::Nvidia {
            count: 1,
            driver_version: "615.71.10".to_string(),
        },
        has_game_shift: true,
        has_vrm_sensor: true,
        has_vrm_telemetry: true,
        has_ambient_sensor: true,
        has_ambient_telemetry: true,
        has_gpu_telemetry: true,
        powercap_supported: true,
        secure_boot_active: true,
        active_provider_name: "dell_awcc".to_string(),
    };

    assert_eq!(caps.vendor, HardwareVendor::Dell);
    assert_eq!(caps.verification_tier, VerificationTier::PhysicallyVerified);
    assert_eq!(caps.tier, VerificationTier::PhysicallyVerified);
    assert!(caps.has_manual_fan_control());
    assert_eq!(caps.fan_control_type_str(), "Boost Offset (0-255)");

    // Test JSON serialization & deserialization
    let json = serde_json::to_string_pretty(&caps).expect("Failed to serialize capabilities");
    assert!(json.contains("Tier1PhysicallyVerified"));
    assert!(json.contains("Dell G15 5530"));

    let deserialized: HardwareCapabilities =
        serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(deserialized.product_name, "Dell G15 5530");
    assert_eq!(deserialized.product, "Dell G15 5530");
}

#[test]
fn test_unsupported_hardware_capability_guards() {
    let unsupp = HardwareCapabilities::unsupported("Acme Corp", "Toaster 2000", "0.0.1");
    assert_eq!(
        unsupp.vendor,
        HardwareVendor::Unknown("Acme Corp".to_string())
    );
    assert_eq!(unsupp.verification_tier, VerificationTier::Unsupported);
    assert_eq!(unsupp.tier, VerificationTier::Unsupported);
    assert!(!unsupp.has_manual_fan_control());
    assert_eq!(unsupp.fan_control_type_str(), "None (Firmware Managed)");
}

#[test]
fn test_new_error_variants() {
    let err = AwccError::UnsupportedOperation {
        operation: "fan_max".to_string(),
        reason: "Hardware lacks manual fan boost control".to_string(),
    };
    let err_str = err.to_string();
    assert!(err_str.contains("fan_max"));
    assert!(err_str.contains("Hardware lacks manual fan boost control"));
}
