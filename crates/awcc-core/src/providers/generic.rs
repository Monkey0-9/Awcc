use crate::capabilities::{
    ChassisType, CpuGovernorCapability, FanControlCapability, GpuTelemetryCapability,
    HardwareCapabilities, HardwareVendor, PlatformProfileCapability, VerificationTier,
};
use crate::error::AwccError;
use crate::providers::{HardwareProvider, ProviderMatch};
use crate::telemetry::{
    FanTelemetry, GModeStatus, SystemTelemetry, ThermalSensor, ThermalTelemetry,
};
use std::fs;
use std::path::{Path, PathBuf};

pub struct GenericLinuxProvider;

impl Default for GenericLinuxProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl GenericLinuxProvider {
    pub fn new() -> Self {
        Self
    }

    fn read_dmi(sysfs: &Path) -> (HardwareVendor, String, String, ChassisType) {
        let read = |rel: &str| {
            let p = sysfs.join(rel);
            fs::read_to_string(&p)
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| "Unknown".to_string())
        };
        let vendor_str = read("sys/devices/virtual/dmi/id/sys_vendor");
        let product = read("sys/devices/virtual/dmi/id/product_name");
        let bios = read("sys/devices/virtual/dmi/id/bios_version");
        let chassis_str = read("sys/devices/virtual/dmi/id/chassis_type");
        let vendor = HardwareVendor::from_dmi(&vendor_str);
        let chassis = ChassisType::from_dmi(&chassis_str);
        (vendor, product, bios, chassis)
    }

    fn find_platform_profile_node(sysfs: &Path) -> Option<(PathBuf, String)> {
        let class_dir = sysfs.join("sys/class/platform-profile");
        if let Ok(entries) = fs::read_dir(&class_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                let name = fs::read_to_string(p.join("name"))
                    .unwrap_or_else(|_| "unknown".to_string())
                    .trim()
                    .to_string();
                if p.join("profile").exists() {
                    return Some((p, name));
                }
            }
        }
        let acpi_profile = sysfs.join("sys/firmware/acpi/platform_profile");
        if acpi_profile.exists() {
            Some((sysfs.join("sys/firmware/acpi"), "acpi_firmware".to_string()))
        } else {
            None
        }
    }
}

impl HardwareProvider for GenericLinuxProvider {
    fn name(&self) -> &'static str {
        "generic_linux"
    }

    fn vendor(&self) -> HardwareVendor {
        HardwareVendor::GenericPc
    }

    fn detect(&self, _sysfs: &Path) -> Option<ProviderMatch> {
        // Universal fallback: always matches with lowest priority (10)
        Some(ProviderMatch {
            priority: 10,
            description: "Generic Linux upstream kernel driver (sysfs/platform-profile/hwmon)"
                .to_string(),
        })
    }

    fn capabilities(&self, sysfs: &Path) -> HardwareCapabilities {
        let (vendor, product, bios, chassis) = Self::read_dmi(sysfs);
        let vendor_name = fs::read_to_string(sysfs.join("sys/devices/virtual/dmi/id/sys_vendor"))
            .unwrap_or_else(|_| "Unknown".to_string())
            .trim()
            .to_string();

        let platform_profile =
            if let Some((node, provider)) = Self::find_platform_profile_node(sysfs) {
                let (prof_file, choices_file) = if provider == "acpi_firmware" {
                    (
                        node.join("platform_profile"),
                        node.join("platform_profile_choices"),
                    )
                } else {
                    (node.join("profile"), node.join("choices"))
                };
                let current = fs::read_to_string(&prof_file)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|_| "Unknown".to_string());
                let choices = fs::read_to_string(&choices_file)
                    .map(|s| s.split_whitespace().map(|x| x.to_string()).collect())
                    .unwrap_or_default();
                PlatformProfileCapability::Supported {
                    provider,
                    current,
                    choices,
                }
            } else {
                PlatformProfileCapability::NotSupported
            };

        // Check if any standard hwmon has pwm controls (pwm1, pwm2, etc.)
        let mut has_pwm = false;
        let hwmon_dir = sysfs.join("sys/class/hwmon");
        if let Ok(entries) = fs::read_dir(&hwmon_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.join("pwm1").exists() || p.join("pwm2").exists() {
                    has_pwm = true;
                    break;
                }
            }
        }

        let fan_control = if has_pwm {
            FanControlCapability::StandardPwm { min: 0, max: 255 }
        } else {
            FanControlCapability::None
        };

        let secure_boot_active = fs::read_to_string(sysfs.join("sys/kernel/security/lockdown"))
            .map(|s| s.contains("[integrity]") || s.contains("[confidentiality]"))
            .unwrap_or(false);

        HardwareCapabilities {
            vendor,
            vendor_name,
            product_name: product.clone(),
            product,
            bios_version: bios,
            chassis,
            verification_tier: VerificationTier::Tier3GenericLinuxFallback,
            tier: VerificationTier::Tier3GenericLinuxFallback,
            fan_control,
            platform_profile,
            cpu_governor: CpuGovernorCapability::NotSupported,
            gpu_telemetry: GpuTelemetryCapability::None,
            has_game_shift: false,
            has_vrm_sensor: false,
            has_vrm_telemetry: false,
            has_ambient_sensor: false,
            has_ambient_telemetry: false,
            has_gpu_telemetry: false,
            powercap_supported: sysfs.join("sys/class/powercap").exists(),
            secure_boot_active,
            active_provider_name: self.name().to_string(),
        }
    }

    fn read_telemetry(&self, sysfs: &Path) -> SystemTelemetry {
        let (vendor, product, bios, _) = Self::read_dmi(sysfs);
        let mut sensors = Vec::new();
        let mut fans = Vec::new();
        let mut cpu_pkg_temp = None;
        let mut cpu_core_max_temp = None;

        let hwmon_dir = sysfs.join("sys/class/hwmon");
        if let Ok(entries) = fs::read_dir(&hwmon_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                let hwmon_name = fs::read_to_string(p.join("name"))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|_| "hwmon".to_string());

                // Read thermal inputs
                if let Ok(files) = fs::read_dir(&p) {
                    for f in files.flatten() {
                        let fname = f.file_name().to_string_lossy().to_string();
                        if fname.starts_with("temp") && fname.ends_with("_input") {
                            let idx = fname.trim_start_matches("temp").trim_end_matches("_input");
                            let lbl = fs::read_to_string(p.join(format!("temp{}_label", idx)))
                                .map(|s| s.trim().to_string())
                                .unwrap_or_else(|_| format!("{} Temp {}", hwmon_name, idx));
                            if let Ok(c_str) = fs::read_to_string(f.path()) {
                                if let Ok(val) = c_str.trim().parse::<f32>() {
                                    let temp_c = val / 1000.0;
                                    sensors.push(ThermalSensor {
                                        label: lbl.clone(),
                                        temp_c: Some(temp_c),
                                        source: hwmon_name.clone(),
                                    });
                                    if lbl.to_lowercase().contains("package")
                                        || lbl.to_lowercase().contains("pkg")
                                    {
                                        cpu_pkg_temp = Some(temp_c);
                                    }
                                    if let Some(cur) = cpu_core_max_temp {
                                        if temp_c > cur {
                                            cpu_core_max_temp = Some(temp_c);
                                        }
                                    } else {
                                        cpu_core_max_temp = Some(temp_c);
                                    }
                                }
                            }
                        } else if fname.starts_with("fan") && fname.ends_with("_input") {
                            let idx = fname.trim_start_matches("fan").trim_end_matches("_input");
                            let fan_idx = idx.parse::<u8>().unwrap_or(1);
                            let lbl = fs::read_to_string(p.join(format!("fan{}_label", idx)))
                                .map(|s| s.trim().to_string())
                                .unwrap_or_else(|_| format!("Fan {}", idx));
                            let rpm = fs::read_to_string(f.path())
                                .ok()
                                .and_then(|s| s.trim().parse::<u32>().ok());
                            let max_rpm = fs::read_to_string(p.join(format!("fan{}_max", idx)))
                                .ok()
                                .and_then(|s| s.trim().parse::<u32>().ok());
                            fans.push(FanTelemetry {
                                fan_index: fan_idx,
                                label: lbl,
                                requested_boost: 0,
                                readback_boost: None,
                                current_rpm: rpm,
                                reported_min_rpm: None,
                                reported_max_rpm: max_rpm,
                                provider: hwmon_name.clone(),
                            });
                        }
                    }
                }
            }
        }

        let (active_platform_profile, platform_profile_choices, platform_profile_provider) =
            if let Some((node, provider)) = Self::find_platform_profile_node(sysfs) {
                let (prof_file, choices_file) = if provider == "acpi_firmware" {
                    (
                        node.join("platform_profile"),
                        node.join("platform_profile_choices"),
                    )
                } else {
                    (node.join("profile"), node.join("choices"))
                };
                let cur = fs::read_to_string(&prof_file)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|_| "Unknown".to_string());
                let choices = fs::read_to_string(&choices_file)
                    .map(|s| s.split_whitespace().map(|x| x.to_string()).collect())
                    .unwrap_or_default();
                (cur, choices, Some(provider))
            } else {
                ("Unknown".to_string(), Vec::new(), None)
            };

        SystemTelemetry {
            dmi_vendor: vendor.to_string(),
            dmi_product: product,
            bios_version: bios,
            active_platform_profile,
            platform_profile_choices,
            platform_profile_provider,
            gmode_status: GModeStatus::NotExposed,
            hwmon_alienware_path: None,
            hwmon_dell_smm_path: None,
            thermals: ThermalTelemetry {
                cpu_pkg_temp,
                cpu_core_max_temp,
                gpu_temp: None,
                vrm_temp: None,
                ambient_temp: None,
                ram_temp: None,
                ram_dimm2_temp: None,
                charger_temp: None,
                sensors,
            },
            fans,
            cpu: None,
            gpu: None,
            memory: None,
            power: None,
            hw_features: None,
            bus_latency_nanos: None,
        }
    }

    fn set_platform_profile(&self, sysfs: &Path, requested: &str) -> Result<(), AwccError> {
        let (node, provider) =
            Self::find_platform_profile_node(sysfs).ok_or(AwccError::PlatformProfileNotFound)?;

        let (prof_file, choices_file) = if provider == "acpi_firmware" {
            (
                node.join("platform_profile"),
                node.join("platform_profile_choices"),
            )
        } else {
            (node.join("profile"), node.join("choices"))
        };

        let choices: Vec<String> = fs::read_to_string(&choices_file)
            .map(|s| s.split_whitespace().map(|x| x.to_string()).collect())
            .unwrap_or_default();

        let req_lower = requested.to_lowercase();
        let effective_target: &str = match req_lower.trim() {
            "battery" | "bat" | "powersave" => {
                if choices.iter().any(|c| c == "low-power") {
                    "low-power"
                } else if choices.iter().any(|c| c == "cool") {
                    "cool"
                } else {
                    "quiet"
                }
            }
            "ultra" => "performance",
            "balance" => "balanced",
            "high" | "perf" => "performance",
            "silent" => "quiet",
            _ => requested,
        };

        let target = if !choices.is_empty() && choices.iter().any(|c| c == requested) {
            requested
        } else if choices.is_empty() || choices.iter().any(|c| c == effective_target) {
            effective_target
        } else {
            return Err(AwccError::InvalidProfile {
                requested: requested.to_string(),
                available: choices,
            });
        };

        fs::write(&prof_file, target).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                AwccError::PermissionDenied {
                    path: prof_file.display().to_string(),
                }
            } else {
                AwccError::Io {
                    path: prof_file.display().to_string(),
                    source: e,
                }
            }
        })?;

        let readback = fs::read_to_string(&prof_file)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        if readback != target {
            return Err(AwccError::ProfileReadbackMismatch {
                requested: requested.to_string(),
                readback,
            });
        }

        Ok(())
    }

    fn set_fan_level(&self, sysfs: &Path, fan_index: u8, level: u8) -> Result<(), AwccError> {
        // Standard Linux hwmon pwm control
        let hwmon_dir = sysfs.join("sys/class/hwmon");
        if let Ok(entries) = fs::read_dir(&hwmon_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                let pwm_path = p.join(format!("pwm{}", fan_index));
                if pwm_path.exists() {
                    // Enable manual mode if pwmX_enable exists (usually mode 1 = manual)
                    let enable_path = p.join(format!("pwm{}_enable", fan_index));
                    if enable_path.exists() {
                        let _ = fs::write(&enable_path, "1");
                    }
                    fs::write(&pwm_path, level.to_string()).map_err(|e| {
                        if e.kind() == std::io::ErrorKind::PermissionDenied {
                            AwccError::PermissionDenied {
                                path: pwm_path.display().to_string(),
                            }
                        } else {
                            AwccError::Io {
                                path: pwm_path.display().to_string(),
                                source: e,
                            }
                        }
                    })?;
                    return Ok(());
                }
            }
        }

        Err(AwccError::UnsupportedOperation {
            operation: "manual fan control".to_string(),
            reason: "Standard hwmon PWM control not exposed for this fan index".to_string(),
        })
    }

    fn set_max_fans(&self, sysfs: &Path) -> Result<(), AwccError> {
        // In generic Linux, set profile to performance or pwm to 255
        if Self::find_platform_profile_node(sysfs).is_some() {
            self.set_platform_profile(sysfs, "performance")
        } else {
            self.set_fan_level(sysfs, 1, 255)
        }
    }

    fn release_fans(&self, sysfs: &Path) -> Result<(), AwccError> {
        // Reset pwmX_enable to 2 (automatic/firmware)
        let hwmon_dir = sysfs.join("sys/class/hwmon");
        if let Ok(entries) = fs::read_dir(&hwmon_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                for i in 1..=6 {
                    let enable_path = p.join(format!("pwm{}_enable", i));
                    if enable_path.exists() {
                        let _ = fs::write(&enable_path, "2");
                    }
                }
            }
        }
        if Self::find_platform_profile_node(sysfs).is_some() {
            let _ = self.set_platform_profile(sysfs, "balanced");
        }
        Ok(())
    }
}
