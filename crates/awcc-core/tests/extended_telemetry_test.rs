use awcc_core::telemetry::{
    CpuComputeTelemetry, FanTelemetry, GModeStatus, GpuComputeTelemetry, MemoryTelemetry,
    PowerTelemetry, SystemTelemetry, ThermalTelemetry,
};
use awcc_core::HardwareManager;

#[test]
fn test_extended_telemetry_serialization() {
    let tele = SystemTelemetry {
        dmi_vendor: "Dell Inc.".to_string(),
        dmi_product: "Dell G15 5530".to_string(),
        bios_version: "1.18.0".to_string(),
        active_platform_profile: "quiet".to_string(),
        platform_profile_choices: vec![
            "quiet".to_string(),
            "balanced".to_string(),
            "performance".to_string(),
        ],
        platform_profile_provider: Some("alienware-wmi".to_string()),
        gmode_status: GModeStatus::Inactive,
        hwmon_alienware_path: Some("/sys/class/hwmon/hwmon4".to_string()),
        hwmon_dell_smm_path: None,
        thermals: ThermalTelemetry {
            cpu_pkg_temp: Some(52.0),
            cpu_core_max_temp: Some(58.0),
            gpu_temp: Some(48.0),
            vrm_temp: Some(54.0),
            ambient_temp: Some(36.0),
            sensors: vec![],
        },
        fans: vec![FanTelemetry {
            fan_index: 1,
            label: "CPU Fan".to_string(),
            requested_boost: 20,
            readback_boost: Some(20),
            current_rpm: Some(1450),
            reported_min_rpm: Some(0),
            reported_max_rpm: Some(4800),
            provider: "alienware_wmi".to_string(),
        }],
        cpu: Some(CpuComputeTelemetry {
            model_name: "i7-13650HX".to_string(),
            cores_logical: 20,
            cur_freq_mhz: Some(1800.0),
            max_freq_mhz: Some(4900.0),
            scaling_governor: Some("powersave".to_string()),
            energy_perf_preference: Some("balance_performance".to_string()),
            usage_percent: Some(4.2),
            package_power_watts: Some(28.5),
        }),
        gpu: Some(GpuComputeTelemetry {
            model_name: Some("RTX 3050 Laptop GPU".to_string()),
            core_clock_mhz: Some(1732),
            memory_clock_mhz: Some(6000),
            utilization_gpu_percent: Some(2),
            memory_used_mb: Some(450),
            memory_total_mb: Some(6144),
            power_draw_watts: Some(7.2),
            power_limit_watts: Some(80.0),
        }),
        memory: Some(MemoryTelemetry {
            ram_used_mb: 8192,
            ram_total_mb: 16384,
            ram_used_percent: 50.0,
            swap_used_mb: 0,
            swap_total_mb: 8192,
        }),
        power: Some(PowerTelemetry {
            ac_online: true,
            battery_percent: Some(100.0),
            battery_status: Some("Full".to_string()),
            charge_rate_watts: None,
        }),
    };

    let json = serde_json::to_string_pretty(&tele).expect("Must serialize to JSON");
    assert!(json.contains("i7-13650HX"));
    assert!(json.contains("RTX 3050 Laptop GPU"));
    assert!(json.contains("ram_used_percent"));
    assert!(json.contains("ac_online"));

    let deserialized: SystemTelemetry =
        serde_json::from_str(&json).expect("Must deserialize from JSON");
    assert_eq!(deserialized.dmi_product, "Dell G15 5530");
    assert_eq!(deserialized.cpu.unwrap().model_name, "i7-13650HX");
    assert_eq!(deserialized.gpu.unwrap().core_clock_mhz, Some(1732));
}

#[test]
fn test_live_hardware_manager_telemetry() {
    let hw = HardwareManager::new();
    let tele = hw.get_telemetry();
    assert!(!tele.dmi_vendor.is_empty());
    // CPU telemetry should be discovered on Linux
    if let Some(ref cpu) = tele.cpu {
        assert!(cpu.cores_logical >= 1);
    }
    // Memory telemetry should be discovered on Linux
    if let Some(ref mem) = tele.memory {
        assert!(mem.ram_total_mb > 0);
    }
}
