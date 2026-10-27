use awcc_core::capabilities::{FanControlCapability, HardwareVendor, VerificationTier};
use awcc_core::HardwareManager;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_asus_mock_sysfs() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let dmi = root.join("sys/devices/virtual/dmi/id");
    fs::create_dir_all(&dmi).unwrap();
    fs::write(dmi.join("sys_vendor"), "ASUSTeK COMPUTER INC.\n").unwrap();
    fs::write(dmi.join("product_name"), "ROG Zephyrus G14\n").unwrap();
    fs::write(dmi.join("chassis_type"), "10\n").unwrap();

    let asus_dev = root.join("sys/devices/platform/asus-nb-wmi");
    fs::create_dir_all(&asus_dev).unwrap();
    fs::write(asus_dev.join("throttle_thermal_policy"), "0\n").unwrap();

    let hw = HardwareManager::with_sysfs_root(root.to_path_buf());
    let caps = hw.capabilities();

    assert_eq!(caps.vendor, HardwareVendor::Asus);
    assert_eq!(caps.tier, VerificationTier::Tier2CommunitySupported);
    assert_eq!(caps.active_provider_name, "asus_wmi");
    assert!(matches!(
        caps.fan_control,
        FanControlCapability::DiscreteLevels { .. }
    ));

    // Test setting fan level sets throttle_thermal_policy
    hw.set_fan_level(1, 255).unwrap();
    assert_eq!(
        fs::read_to_string(asus_dev.join("throttle_thermal_policy"))
            .unwrap()
            .trim(),
        "1"
    );

    hw.release_fans().unwrap();
    assert_eq!(
        fs::read_to_string(asus_dev.join("throttle_thermal_policy"))
            .unwrap()
            .trim(),
        "0"
    );
}

#[test]
fn test_asus_strix_mock_sysfs() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let dmi = root.join("sys/devices/virtual/dmi/id");
    fs::create_dir_all(&dmi).unwrap();
    fs::write(dmi.join("sys_vendor"), "ASUSTeK COMPUTER INC.\n").unwrap();
    fs::write(dmi.join("product_name"), "ROG Strix SCAR 16\n").unwrap();
    fs::write(dmi.join("chassis_type"), "10\n").unwrap();

    let asus_dev = root.join("sys/devices/platform/asus_wmi");
    fs::create_dir_all(&asus_dev).unwrap();
    fs::write(asus_dev.join("throttle_thermal_policy"), "0\n").unwrap();

    let hw = HardwareManager::with_sysfs_root(root.to_path_buf());
    let caps = hw.capabilities();

    assert_eq!(caps.vendor, HardwareVendor::Asus);
    assert_eq!(caps.tier, VerificationTier::Tier2CommunitySupported);
    assert_eq!(caps.active_provider_name, "asus_wmi");
    assert!(matches!(
        caps.fan_control,
        FanControlCapability::DiscreteLevels { .. }
    ));

    hw.set_fan_level(1, 255).unwrap();
    assert_eq!(
        fs::read_to_string(asus_dev.join("throttle_thermal_policy"))
            .unwrap()
            .trim(),
        "1"
    );

    hw.release_fans().unwrap();
    assert_eq!(
        fs::read_to_string(asus_dev.join("throttle_thermal_policy"))
            .unwrap()
            .trim(),
        "0"
    );
}

#[test]
fn test_lenovo_mock_sysfs() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let dmi = root.join("sys/devices/virtual/dmi/id");
    fs::create_dir_all(&dmi).unwrap();
    fs::write(dmi.join("sys_vendor"), "LENOVO\n").unwrap();
    fs::write(dmi.join("product_name"), "ThinkPad X1 Carbon Gen 11\n").unwrap();
    fs::write(dmi.join("chassis_type"), "10\n").unwrap();

    let ibm_dir = root.join("proc/acpi/ibm");
    fs::create_dir_all(&ibm_dir).unwrap();
    fs::write(
        ibm_dir.join("fan"),
        "status: enabled\nspeed: 2400\nlevel: auto\n",
    )
    .unwrap();

    let hw = HardwareManager::with_sysfs_root(root.to_path_buf());
    let caps = hw.capabilities();

    assert_eq!(caps.vendor, HardwareVendor::Lenovo);
    assert_eq!(caps.tier, VerificationTier::Tier2CommunitySupported);
    assert_eq!(caps.active_provider_name, "lenovo_thinkpad");

    // Test max fans -> disengaged
    hw.set_max_fans().unwrap();
    assert!(fs::read_to_string(ibm_dir.join("fan"))
        .unwrap()
        .contains("disengaged"));

    // Test release fans -> auto
    hw.release_fans().unwrap();
    assert!(fs::read_to_string(ibm_dir.join("fan"))
        .unwrap()
        .contains("auto"));
}

#[test]
fn test_hp_mock_sysfs() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let dmi = root.join("sys/devices/virtual/dmi/id");
    fs::create_dir_all(&dmi).unwrap();
    fs::write(dmi.join("sys_vendor"), "HP\n").unwrap();
    fs::write(dmi.join("product_name"), "OMEN by HP Laptop 16-xf0xxx\n").unwrap();
    fs::write(dmi.join("chassis_type"), "10\n").unwrap();

    let hp_dev = root.join("sys/devices/platform/hp-wmi");
    fs::create_dir_all(&hp_dev).unwrap();

    let hw = HardwareManager::with_sysfs_root(root.to_path_buf());
    let caps = hw.capabilities();

    assert_eq!(caps.vendor, HardwareVendor::Hp);
    assert_eq!(caps.tier, VerificationTier::Tier2CommunitySupported);
    assert_eq!(caps.active_provider_name, "hp_wmi");
}

#[test]
fn test_acer_mock_sysfs() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let dmi = root.join("sys/devices/virtual/dmi/id");
    fs::create_dir_all(&dmi).unwrap();
    fs::write(dmi.join("sys_vendor"), "Acer\n").unwrap();
    fs::write(dmi.join("product_name"), "Predator PH16-71\n").unwrap();
    fs::write(dmi.join("chassis_type"), "10\n").unwrap();

    let acer_dev = root.join("sys/devices/platform/acer-wmi");
    fs::create_dir_all(&acer_dev).unwrap();
    fs::write(acer_dev.join("turbo_mode"), "0\n").unwrap();

    let hw = HardwareManager::with_sysfs_root(root.to_path_buf());
    let caps = hw.capabilities();

    assert_eq!(caps.vendor, HardwareVendor::Acer);
    assert_eq!(caps.tier, VerificationTier::Tier2CommunitySupported);
    assert_eq!(caps.active_provider_name, "acer_wmi");
    assert!(matches!(
        caps.fan_control,
        FanControlCapability::DiscreteLevels { .. }
    ));

    // Test max fans -> turbo_mode = 1
    hw.set_max_fans().unwrap();
    assert_eq!(
        fs::read_to_string(acer_dev.join("turbo_mode"))
            .unwrap()
            .trim(),
        "1"
    );

    // Test release fans -> turbo_mode = 0
    hw.release_fans().unwrap();
    assert_eq!(
        fs::read_to_string(acer_dev.join("turbo_mode"))
            .unwrap()
            .trim(),
        "0"
    );
}

#[test]
fn test_msi_mock_sysfs() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let dmi = root.join("sys/devices/virtual/dmi/id");
    fs::create_dir_all(&dmi).unwrap();
    fs::write(
        dmi.join("sys_vendor"),
        "Micro-Star International Co., Ltd.\n",
    )
    .unwrap();
    fs::write(dmi.join("product_name"), "Katana 15 B13VGK\n").unwrap();
    fs::write(dmi.join("chassis_type"), "10\n").unwrap();

    let msi_dev = root.join("sys/devices/platform/msi-ec");
    fs::create_dir_all(&msi_dev).unwrap();
    fs::write(msi_dev.join("cooler_boost"), "0\n").unwrap();

    let hw = HardwareManager::with_sysfs_root(root.to_path_buf());
    let caps = hw.capabilities();

    assert_eq!(caps.vendor, HardwareVendor::Msi);
    assert_eq!(caps.tier, VerificationTier::Tier2CommunitySupported);
    assert_eq!(caps.active_provider_name, "msi_ec");
    assert!(matches!(
        caps.fan_control,
        FanControlCapability::DiscreteLevels { .. }
    ));

    // Test max fans -> cooler_boost = 1
    hw.set_max_fans().unwrap();
    assert_eq!(
        fs::read_to_string(msi_dev.join("cooler_boost"))
            .unwrap()
            .trim(),
        "1"
    );

    // Test release fans -> cooler_boost = 0
    hw.release_fans().unwrap();
    assert_eq!(
        fs::read_to_string(msi_dev.join("cooler_boost"))
            .unwrap()
            .trim(),
        "0"
    );
}

#[test]
fn test_system76_mock_sysfs() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let dmi = root.join("sys/devices/virtual/dmi/id");
    fs::create_dir_all(&dmi).unwrap();
    fs::write(dmi.join("sys_vendor"), "System76\n").unwrap();
    fs::write(dmi.join("product_name"), "Oryx Pro\n").unwrap();
    fs::write(dmi.join("chassis_type"), "10\n").unwrap();

    let sys76_dev = root.join("sys/devices/platform/system76_acpi");
    fs::create_dir_all(&sys76_dev).unwrap();
    fs::write(sys76_dev.join("fan_mode"), "auto\n").unwrap();

    let hw = HardwareManager::with_sysfs_root(root.to_path_buf());
    let caps = hw.capabilities();

    assert_eq!(caps.vendor, HardwareVendor::System76);
    assert_eq!(caps.tier, VerificationTier::Tier2CommunitySupported);
    assert_eq!(caps.active_provider_name, "system76_acpi");
    assert!(matches!(
        caps.fan_control,
        FanControlCapability::DiscreteLevels { .. }
    ));

    // Test max fans -> fan_mode = full
    hw.set_max_fans().unwrap();
    assert_eq!(
        fs::read_to_string(sys76_dev.join("fan_mode"))
            .unwrap()
            .trim(),
        "full"
    );

    // Test release fans -> fan_mode = auto
    hw.release_fans().unwrap();
    assert_eq!(
        fs::read_to_string(sys76_dev.join("fan_mode"))
            .unwrap()
            .trim(),
        "auto"
    );
}

#[test]
fn test_framework_mock_sysfs() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let dmi = root.join("sys/devices/virtual/dmi/id");
    fs::create_dir_all(&dmi).unwrap();
    fs::write(dmi.join("sys_vendor"), "Framework\n").unwrap();
    fs::write(
        dmi.join("product_name"),
        "Laptop 13 (AMD Ryzen 7040Series)\n",
    )
    .unwrap();
    fs::write(dmi.join("chassis_type"), "10\n").unwrap();

    let hw = HardwareManager::with_sysfs_root(root.to_path_buf());
    let caps = hw.capabilities();

    assert_eq!(caps.vendor, HardwareVendor::Framework);
    assert_eq!(caps.tier, VerificationTier::Tier2CommunitySupported);
    assert_eq!(caps.active_provider_name, "framework_cros_ec");
}

#[test]
fn test_unsupported_pc_graceful_degradation() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let dmi = root.join("sys/devices/virtual/dmi/id");
    fs::create_dir_all(&dmi).unwrap();
    fs::write(dmi.join("sys_vendor"), "Unknown OEM\n").unwrap();
    fs::write(dmi.join("product_name"), "Barebone Mini PC\n").unwrap();

    let hw = HardwareManager::with_sysfs_root(root.to_path_buf());
    let caps = hw.capabilities();

    assert_eq!(caps.tier, VerificationTier::Tier3GenericLinuxFallback);
    assert_eq!(caps.fan_control, FanControlCapability::None);

    // Unsupported fan control returns explicit Error, never panics
    let err = hw.set_fan_level(1, 100).unwrap_err();
    assert!(err.to_string().contains("Unsupported operation"));
}
