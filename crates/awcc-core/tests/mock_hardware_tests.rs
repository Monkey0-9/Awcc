use awcc_core::error::AwccError;
use awcc_core::telemetry::GModeStatus;
use awcc_core::HardwareManager;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_mock_sysfs_platform_profile_prioritizes_alienware_wmi() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create dual platform profile class nodes: platform-profile-0 (dell-pc) and platform-profile-1 (alienware-wmi)
    let pp0 = root.join("sys/class/platform-profile/platform-profile-0");
    let pp1 = root.join("sys/class/platform-profile/platform-profile-1");
    fs::create_dir_all(&pp0).unwrap();
    fs::create_dir_all(&pp1).unwrap();

    fs::write(pp0.join("name"), "dell-pc\n").unwrap();
    fs::write(pp0.join("profile"), "cool\n").unwrap();
    fs::write(pp0.join("choices"), "cool quiet balanced performance\n").unwrap();

    fs::write(pp1.join("name"), "alienware-wmi\n").unwrap();
    fs::write(pp1.join("profile"), "balanced\n").unwrap();
    fs::write(
        pp1.join("choices"),
        "low-power quiet balanced balanced-performance performance custom\n",
    )
    .unwrap();

    let hw = HardwareManager::with_sysfs_root(root.to_path_buf());
    let (cur, choices, provider) = hw.get_platform_profile();

    assert_eq!(provider, Some("alienware-wmi".to_string()));
    assert_eq!(cur, "balanced");
    assert!(choices.contains(&"custom".to_string()));
    assert!(choices.contains(&"balanced-performance".to_string()));

    // Test profile setting with validation
    hw.set_platform_profile("performance").unwrap();
    let (new_cur, _, _) = hw.get_platform_profile();
    assert_eq!(new_cur, "performance");

    // Test rejection of invalid profile
    let err = hw.set_platform_profile("invalid_profile").unwrap_err();
    match err {
        AwccError::InvalidProfile {
            requested,
            available,
        } => {
            assert_eq!(requested, "invalid_profile");
            assert!(available.contains(&"performance".to_string()));
        }
        other => panic!("Expected InvalidProfile error, got: {:?}", other),
    }
}

#[test]
fn test_mock_sysfs_arbitrary_hwmon_and_fan_discovery() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create alienware_wmi at arbitrary high index: hwmon11
    let hw11 = root.join("sys/class/hwmon/hwmon11");
    fs::create_dir_all(&hw11).unwrap();
    fs::write(hw11.join("name"), "alienware_wmi\n").unwrap();
    fs::write(hw11.join("fan1_label"), "Chassis Primary Fan\n").unwrap();
    fs::write(hw11.join("fan1_input"), "3200\n").unwrap();
    fs::write(hw11.join("fan1_min"), "0\n").unwrap();
    fs::write(hw11.join("fan1_max"), "5400\n").unwrap();
    fs::write(hw11.join("fan1_boost"), "128\n").unwrap();

    let hw = HardwareManager::with_sysfs_root(root.to_path_buf());
    assert!(hw.is_driver_loaded());

    let fans = hw.discover_fans();
    assert_eq!(fans.len(), 1);
    assert_eq!(fans[0].fan_index, 1);
    assert_eq!(fans[0].label, "Chassis Primary Fan");
    assert_eq!(fans[0].current_rpm, Some(3200));
    assert_eq!(fans[0].reported_max_rpm, Some(5400));
    assert_eq!(fans[0].readback_boost, Some(128));

    // Test boost write and readback
    hw.write_fan_boost(1, 200).unwrap();
    assert_eq!(hw.read_fan_boost(1).unwrap(), 200);

    // Test invalid fan index
    let err = hw.write_fan_boost(9, 255).unwrap_err();
    assert!(matches!(err, AwccError::InvalidFanIndex(9)));
}

#[test]
fn test_mock_sysfs_missing_sensors_remain_none() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Mock empty hwmon without coretemp or dell_ddv
    let hwmon_dir = root.join("sys/class/hwmon");
    fs::create_dir_all(&hwmon_dir).unwrap();

    let hw = HardwareManager::with_sysfs_root(root.to_path_buf());
    let thermals = hw.read_thermal_telemetry();

    // Sensor readings from absent sysfs nodes must be None, NOT fake 0.0°C
    assert_eq!(thermals.cpu_pkg_temp, None);
    assert_eq!(thermals.cpu_core_max_temp, None);
    assert_eq!(thermals.vrm_temp, None);
    assert_eq!(thermals.ambient_temp, None);
    // Ensure no sysfs sensors were ingested from the empty mock directory
    assert!(thermals
        .sensors
        .iter()
        .all(|s| s.source != "coretemp" && s.source != "dell_ddv" && s.source != "alienware_wmi"));
}

#[test]
fn test_mock_sysfs_gmode_status_reporting() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let hw1 = root.join("sys/class/hwmon/hwmon1");
    fs::create_dir_all(&hw1).unwrap();
    fs::write(hw1.join("name"), "alienware_wmi\n").unwrap();

    let hw = HardwareManager::with_sysfs_root(root.to_path_buf());
    // Without dedicated gmode sysfs node, reports NotExposed
    assert_eq!(hw.check_gmode_status(), GModeStatus::NotExposed);

    // If driver exposes gmode node = 1, reports Active
    fs::write(hw1.join("gmode"), "1\n").unwrap();
    assert_eq!(hw.check_gmode_status(), GModeStatus::Active);

    // If driver exposes gmode node = 0, reports Inactive
    fs::write(hw1.join("gmode"), "0\n").unwrap();
    assert_eq!(hw.check_gmode_status(), GModeStatus::Inactive);
}

#[test]
fn test_mock_sysfs_profile_normalization() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let pp0 = root.join("sys/class/platform-profile/platform-profile-0");
    fs::create_dir_all(&pp0).unwrap();
    fs::write(pp0.join("name"), "dell-pc\n").unwrap();
    fs::write(pp0.join("profile"), "balanced\n").unwrap();
    fs::write(pp0.join("choices"), "cool quiet balanced performance\n").unwrap();

    let hw = HardwareManager::with_sysfs_root(root.to_path_buf());

    // "battery" should normalize to "cool" or "quiet"
    hw.set_platform_profile("battery").unwrap();
    let (cur, _, _) = hw.get_platform_profile();
    assert!(cur == "cool" || cur == "quiet");

    // "ultra" should normalize to "performance"
    hw.set_platform_profile("ultra").unwrap();
    let (cur, _, _) = hw.get_platform_profile();
    assert_eq!(cur, "performance");

    // "balance" should normalize to "balanced"
    hw.set_platform_profile("balance").unwrap();
    let (cur, _, _) = hw.get_platform_profile();
    assert_eq!(cur, "balanced");
}

#[test]
fn test_mock_sysfs_dell_smm_fan_fallback() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Mock dell_smm at hwmon4 without alienware_wmi
    let hw4 = root.join("sys/class/hwmon/hwmon4");
    fs::create_dir_all(&hw4).unwrap();
    fs::write(hw4.join("name"), "dell_smm\n").unwrap();
    fs::write(hw4.join("fan1_label"), "Processor Fan\n").unwrap();
    fs::write(hw4.join("fan1_input"), "2100\n").unwrap();
    fs::write(hw4.join("fan1_max"), "3700\n").unwrap();
    fs::write(hw4.join("pwm1"), "128\n").unwrap();

    fs::write(hw4.join("fan2_label"), "Video Fan\n").unwrap();
    fs::write(hw4.join("fan2_input"), "2400\n").unwrap();
    fs::write(hw4.join("fan2_max"), "4000\n").unwrap();
    fs::write(hw4.join("pwm2"), "128\n").unwrap();

    let hw = HardwareManager::with_sysfs_root(root.to_path_buf());
    let fans = hw.discover_fans();

    assert_eq!(fans.len(), 2);
    assert_eq!(fans[0].fan_index, 1);
    assert_eq!(fans[0].label, "Processor Fan");
    assert_eq!(fans[0].current_rpm, Some(2100));
    assert_eq!(fans[0].reported_max_rpm, Some(3700));
    assert_eq!(fans[0].provider, "dell_smm");

    assert_eq!(fans[1].fan_index, 2);
    assert_eq!(fans[1].label, "Video Fan");
    assert_eq!(fans[1].current_rpm, Some(2400));
    assert_eq!(fans[1].reported_max_rpm, Some(4000));
    assert_eq!(fans[1].provider, "dell_smm");

    assert_eq!(hw.read_fan_rpm(1), Some(2100));
    assert_eq!(hw.read_fan_rpm(2), Some(2400));
}
