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
use std::process::Command;

pub struct DellAwccProvider;

impl Default for DellAwccProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DellAwccProvider {
    pub fn new() -> Self {
        Self
    }

    fn find_hwmon_by_name(sysfs: &Path, target: &str) -> Option<PathBuf> {
        let hwmon_dir = sysfs.join("sys/class/hwmon");
        if let Ok(entries) = fs::read_dir(&hwmon_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Ok(name) = fs::read_to_string(p.join("name")) {
                    if name.trim() == target {
                        return Some(p);
                    }
                }
            }
        }
        None
    }

    fn find_platform_profile_node(sysfs: &Path) -> Option<(PathBuf, String)> {
        let class_dir = sysfs.join("sys/class/platform-profile");
        if let Ok(entries) = fs::read_dir(&class_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Ok(name) = fs::read_to_string(p.join("name")) {
                    if name.trim() == "alienware-wmi" {
                        return Some((p, "alienware-wmi".to_string()));
                    }
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

impl HardwareProvider for DellAwccProvider {
    fn name(&self) -> &'static str {
        "dell_awcc"
    }

    fn vendor(&self) -> HardwareVendor {
        HardwareVendor::Dell
    }

    fn detect(&self, sysfs: &Path) -> Option<ProviderMatch> {
        let aw_hwmon = Self::find_hwmon_by_name(sysfs, "alienware_wmi");
        let wmi_wmax = sysfs
            .join("sys/bus/wmi/devices/A70591CE-A997-11DA-B012-B622A1EF5492-34")
            .exists();
        let vendor_str = fs::read_to_string(sysfs.join("sys/devices/virtual/dmi/id/sys_vendor"))
            .unwrap_or_default()
            .to_lowercase();
        let is_dell = vendor_str.contains("dell") || vendor_str.contains("alienware");

        if aw_hwmon.is_some() || (is_dell && wmi_wmax) {
            Some(ProviderMatch {
                priority: 100,
                description: "Dell Alienware Command Center (alienware-wmi + WMAX ACPI)"
                    .to_string(),
            })
        } else if is_dell {
            Some(ProviderMatch {
                priority: 50,
                description: "Dell laptop detected without alienware-wmi driver active".to_string(),
            })
        } else {
            None
        }
    }

    fn capabilities(&self, sysfs: &Path) -> HardwareCapabilities {
        let vendor_str = fs::read_to_string(sysfs.join("sys/devices/virtual/dmi/id/sys_vendor"))
            .unwrap_or_default();
        let product = fs::read_to_string(sysfs.join("sys/devices/virtual/dmi/id/product_name"))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "Unknown Dell".to_string());
        let chassis_str = fs::read_to_string(sysfs.join("sys/devices/virtual/dmi/id/chassis_type"))
            .unwrap_or_default();

        let vendor = HardwareVendor::from_dmi(&vendor_str);
        let chassis = ChassisType::from_dmi(&chassis_str);

        let tier = if product.contains("G15 5530")
            || product.contains("G16 7630")
            || product.contains("Alienware")
        {
            VerificationTier::Tier1PhysicallyVerified
        } else {
            VerificationTier::Tier2CommunitySupported
        };

        let aw_hwmon = Self::find_hwmon_by_name(sysfs, "alienware_wmi");
        let fan_control = if aw_hwmon.is_some() {
            FanControlCapability::BoostOffset { max_boost: 255 }
        } else {
            FanControlCapability::None
        };

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

        let secure_boot_active = fs::read_to_string(sysfs.join("sys/kernel/security/lockdown"))
            .map(|s| s.contains("[integrity]") || s.contains("[confidentiality]"))
            .unwrap_or(false);

        let bios = fs::read_to_string(sysfs.join("sys/devices/virtual/dmi/id/bios_version"))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "Unknown".to_string());

        let has_vrm = Self::find_hwmon_by_name(sysfs, "dell_ddv").is_some();
        let has_amb = Self::find_hwmon_by_name(sysfs, "dell_ddv").is_some();

        HardwareCapabilities {
            vendor,
            vendor_name: vendor_str.trim().to_string(),
            product_name: product.clone(),
            product,
            bios_version: bios,
            chassis,
            verification_tier: tier,
            tier,
            fan_control,
            platform_profile,
            cpu_governor: CpuGovernorCapability::NotSupported,
            gpu_telemetry: GpuTelemetryCapability::Nvidia {
                count: 1,
                driver_version: "NVIDIA".to_string(),
            },
            has_game_shift: true,
            has_vrm_sensor: has_vrm,
            has_vrm_telemetry: has_vrm,
            has_ambient_sensor: has_amb,
            has_ambient_telemetry: has_amb,
            has_gpu_telemetry: true,
            powercap_supported: sysfs.join("sys/class/powercap").exists(),
            secure_boot_active,
            active_provider_name: self.name().to_string(),
        }
    }

    fn read_telemetry(&self, sysfs: &Path) -> SystemTelemetry {
        let read_dmi = |rel: &str| {
            fs::read_to_string(sysfs.join(rel))
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| "Unknown".to_string())
        };
        let vendor = read_dmi("sys/devices/virtual/dmi/id/sys_vendor");
        let product = read_dmi("sys/devices/virtual/dmi/id/product_name");
        let bios = read_dmi("sys/devices/virtual/dmi/id/bios_version");

        let alienware_hwmon = Self::find_hwmon_by_name(sysfs, "alienware_wmi");
        let dell_smm_hwmon = Self::find_hwmon_by_name(sysfs, "dell_smm");
        let coretemp_hwmon = Self::find_hwmon_by_name(sysfs, "coretemp");
        let dell_ddv_hwmon = Self::find_hwmon_by_name(sysfs, "dell_ddv");

        let mut cpu_pkg_temp = None;
        let mut cpu_core_max_temp = None;
        let mut vrm_temp = None;
        let mut ambient_temp = None;
        let mut ram_temp = None;
        let mut ram_dimm2_temp = None;
        let mut charger_temp = None;
        let mut sensors = Vec::new();

        // 1. Ingest from coretemp
        if let Some(ref ct) = coretemp_hwmon {
            if let Ok(entries) = fs::read_dir(ct) {
                for entry in entries.flatten() {
                    let fname = entry.file_name().to_string_lossy().to_string();
                    if fname.starts_with("temp") && fname.ends_with("_input") {
                        let idx = fname.trim_start_matches("temp").trim_end_matches("_input");
                        let label = fs::read_to_string(ct.join(format!("temp{}_label", idx)))
                            .map(|s| s.trim().to_string())
                            .unwrap_or_else(|_| format!("Coretemp {}", idx));
                        if let Ok(c_str) = fs::read_to_string(entry.path()) {
                            if let Ok(val) = c_str.trim().parse::<f32>() {
                                let c = val / 1000.0;
                                sensors.push(ThermalSensor {
                                    label: label.clone(),
                                    temp_c: Some(c),
                                    source: "coretemp".to_string(),
                                });
                                if label.to_lowercase().contains("pkg")
                                    || label.to_lowercase().contains("package")
                                {
                                    cpu_pkg_temp = Some(c);
                                }
                                if let Some(cur) = cpu_core_max_temp {
                                    if c > cur {
                                        cpu_core_max_temp = Some(c);
                                    }
                                } else {
                                    cpu_core_max_temp = Some(c);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Ingest from dell_ddv (Alienware & Dell Data Vault)
        let mut gpu_temp_fallback = None;
        if let Some(ref ddv) = dell_ddv_hwmon {
            for idx in 1..=10 {
                let input_path = ddv.join(format!("temp{}_input", idx));
                if !input_path.exists() {
                    continue;
                }
                let label = fs::read_to_string(ddv.join(format!("temp{}_label", idx)))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|_| format!("Thermal Sensor {}", idx));
                if let Ok(c_str) = fs::read_to_string(&input_path) {
                    if let Ok(val) = c_str.trim().parse::<f32>() {
                        let c = val / 1000.0;
                        sensors.push(ThermalSensor {
                            label: label.clone(),
                            temp_c: Some(c),
                            source: "dell_ddv".to_string(),
                        });

                        let lower = label.to_lowercase();
                        if lower.contains("sodimm 2") || lower.contains("dimm 2") {
                            ram_dimm2_temp = Some(c);
                        } else if lower.contains("sodimm")
                            || lower.contains("dimm")
                            || lower.contains("ram")
                        {
                            ram_temp = Some(c);
                        } else if lower.contains("charger") {
                            charger_temp = Some(c);
                        } else if lower.contains("video") || lower.contains("gpu") {
                            gpu_temp_fallback = Some(c);
                        } else if lower.contains("cpu") {
                            if cpu_pkg_temp.is_none() {
                                cpu_pkg_temp = Some(c);
                            }
                        } else if idx == 2 {
                            vrm_temp = Some(c);
                        } else if idx == 3 {
                            ambient_temp = Some(c);
                        }
                    }
                }
            }
        }

        // 2b. Fallback to dell_smm for SODIMM, VRM, and Ambient
        if ram_temp.is_none() || vrm_temp.is_none() {
            if let Some(ref smm) = dell_smm_hwmon {
                for (idx, target, default_label) in [
                    (2, &mut vrm_temp, "Motherboard VRM"),
                    (3, &mut ambient_temp, "Ambient Air"),
                    (5, &mut ram_temp, "SODIMM 1"),
                    (6, &mut ram_dimm2_temp, "SODIMM 2"),
                    (7, &mut charger_temp, "Charger VRM"),
                ] {
                    let input_path = smm.join(format!("temp{}_input", idx));
                    if input_path.exists() && target.is_none() {
                        if let Ok(c_str) = fs::read_to_string(&input_path) {
                            if let Ok(val) = c_str.trim().parse::<f32>() {
                                let c = val / 1000.0;
                                *target = Some(c);
                                sensors.push(ThermalSensor {
                                    label: default_label.to_string(),
                                    temp_c: Some(c),
                                    source: "dell_smm".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // 3. Ingest from alienware_wmi
        if let Some(ref aw) = alienware_hwmon {
            for idx in 1..=6 {
                let input_path = aw.join(format!("temp{}_input", idx));
                if input_path.exists() {
                    let label = fs::read_to_string(aw.join(format!("temp{}_label", idx)))
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|_| format!("AWCC Temp {}", idx));
                    if let Ok(c_str) = fs::read_to_string(&input_path) {
                        if let Ok(val) = c_str.trim().parse::<f32>() {
                            let c = val / 1000.0;
                            sensors.push(ThermalSensor {
                                label: label.clone(),
                                temp_c: Some(c),
                                source: "alienware_wmi".to_string(),
                            });
                            if cpu_pkg_temp.is_none() && label.eq_ignore_ascii_case("CPU") {
                                cpu_pkg_temp = Some(c);
                            }
                        }
                    }
                }
            }
        }

        // 4. Query GPU temperature via nvidia-smi
        let gpu_temp = Command::new("nvidia-smi")
            .args([
                "--query-gpu=temperature.gpu",
                "--format=csv,noheader,nounits",
            ])
            .output()
            .ok()
            .and_then(|out| {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                s.lines().next().and_then(|l| l.trim().parse::<f32>().ok())
            });
        if let Some(gt) = gpu_temp {
            sensors.push(ThermalSensor {
                label: "NVIDIA GPU".to_string(),
                temp_c: Some(gt),
                source: "nvidia-smi".to_string(),
            });
        }

        // 5. Discover Fans
        let mut fans = Vec::new();
        if let Some(ref aw) = alienware_hwmon {
            for fan_index in 1..=6 {
                let boost_path = aw.join(format!("fan{}_boost", fan_index));
                let input_path = aw.join(format!("fan{}_input", fan_index));
                if !boost_path.exists() && !input_path.exists() {
                    continue;
                }
                let label = fs::read_to_string(aw.join(format!("fan{}_label", fan_index)))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|_| {
                        if fan_index == 1 {
                            "CPU Fan".to_string()
                        } else if fan_index == 2 {
                            "GPU Fan".to_string()
                        } else {
                            format!("Fan {}", fan_index)
                        }
                    });
                let readback_boost = fs::read_to_string(&boost_path)
                    .ok()
                    .and_then(|s| s.trim().parse::<u8>().ok());
                let current_rpm = fs::read_to_string(&input_path)
                    .ok()
                    .and_then(|s| s.trim().parse::<u32>().ok());
                let reported_min_rpm = fs::read_to_string(aw.join(format!("fan{}_min", fan_index)))
                    .ok()
                    .and_then(|s| s.trim().parse::<u32>().ok());
                let reported_max_rpm = fs::read_to_string(aw.join(format!("fan{}_max", fan_index)))
                    .ok()
                    .and_then(|s| s.trim().parse::<u32>().ok());
                fans.push(FanTelemetry {
                    fan_index,
                    label,
                    requested_boost: readback_boost.unwrap_or(0),
                    readback_boost,
                    current_rpm,
                    reported_min_rpm,
                    reported_max_rpm,
                    provider: "alienware_wmi".to_string(),
                });
            }
        }

        // Fallback to dell_smm hwmon tachometers if alienware_wmi is not loaded
        if fans.is_empty() {
            if let Some(ref smm) = dell_smm_hwmon {
                for fan_index in 1..=6 {
                    let input_path = smm.join(format!("fan{}_input", fan_index));
                    if !input_path.exists() {
                        continue;
                    }
                    let label = fs::read_to_string(smm.join(format!("fan{}_label", fan_index)))
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|_| {
                            if fan_index == 1 {
                                "CPU Fan".to_string()
                            } else if fan_index == 2 {
                                "GPU Fan".to_string()
                            } else {
                                format!("Fan {}", fan_index)
                            }
                        });
                    let current_rpm = fs::read_to_string(&input_path)
                        .ok()
                        .and_then(|s| s.trim().parse::<u32>().ok());
                    let min_path = smm.join(format!("fan{}_min", fan_index));
                    let reported_min_rpm = fs::read_to_string(&min_path)
                        .ok()
                        .and_then(|s| s.trim().parse::<u32>().ok());
                    let max_path = smm.join(format!("fan{}_max", fan_index));
                    let reported_max_rpm = fs::read_to_string(&max_path)
                        .ok()
                        .and_then(|s| s.trim().parse::<u32>().ok());
                    let pwm_path = smm.join(format!("pwm{}", fan_index));
                    let pwm_val = fs::read_to_string(&pwm_path)
                        .ok()
                        .and_then(|s| s.trim().parse::<u8>().ok());

                    fans.push(FanTelemetry {
                        fan_index,
                        label,
                        requested_boost: pwm_val.unwrap_or(0),
                        readback_boost: pwm_val,
                        current_rpm,
                        reported_min_rpm,
                        reported_max_rpm,
                        provider: "dell_smm".to_string(),
                    });
                }
            }
        }

        // 6. Platform Profile & GMode
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

        let gmode_status = if let Some(ref aw) = alienware_hwmon {
            let g = aw.join("gmode");
            if g.exists() {
                if let Ok(s) = fs::read_to_string(&g) {
                    if s.trim() == "1" || s.trim().eq_ignore_ascii_case("on") {
                        GModeStatus::Active
                    } else {
                        GModeStatus::Inactive
                    }
                } else {
                    GModeStatus::NotExposed
                }
            } else {
                GModeStatus::NotExposed
            }
        } else {
            GModeStatus::NotExposed
        };

        SystemTelemetry {
            dmi_vendor: vendor,
            dmi_product: product,
            bios_version: bios,
            active_platform_profile,
            platform_profile_choices,
            platform_profile_provider,
            gmode_status,
            hwmon_alienware_path: alienware_hwmon.map(|p| p.display().to_string()),
            hwmon_dell_smm_path: dell_smm_hwmon.map(|p| p.display().to_string()),
            thermals: ThermalTelemetry {
                cpu_pkg_temp,
                cpu_core_max_temp,
                gpu_temp: gpu_temp.or(gpu_temp_fallback),
                vrm_temp,
                ambient_temp,
                ram_temp,
                ram_dimm2_temp,
                charger_temp,
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
        let aw =
            Self::find_hwmon_by_name(sysfs, "alienware_wmi").ok_or(AwccError::DriverNotFound)?;
        let path = aw.join(format!("fan{}_boost", fan_index));
        if !path.exists() {
            return Err(AwccError::InvalidFanIndex(fan_index));
        }

        fs::write(&path, level.to_string()).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                AwccError::PermissionDenied {
                    path: path.display().to_string(),
                }
            } else {
                AwccError::Io {
                    path: path.display().to_string(),
                    source: e,
                }
            }
        })?;

        let readback = fs::read_to_string(&path)
            .ok()
            .and_then(|s| s.trim().parse::<u8>().ok())
            .unwrap_or(0);

        if readback != level {
            return Err(AwccError::ReadbackMismatch {
                fan: fan_index,
                requested: level,
                readback,
            });
        }

        Ok(())
    }

    fn set_max_fans(&self, sysfs: &Path) -> Result<(), AwccError> {
        let _ = self.set_platform_profile(sysfs, "performance");
        let aw =
            Self::find_hwmon_by_name(sysfs, "alienware_wmi").ok_or(AwccError::DriverNotFound)?;
        for fan_idx in 1..=2 {
            let p = aw.join(format!("fan{}_boost", fan_idx));
            if p.exists() {
                self.set_fan_level(sysfs, fan_idx, 255)?;
            }
        }
        Ok(())
    }

    fn release_fans(&self, sysfs: &Path) -> Result<(), AwccError> {
        let aw =
            Self::find_hwmon_by_name(sysfs, "alienware_wmi").ok_or(AwccError::DriverNotFound)?;
        for fan_idx in 1..=2 {
            let p = aw.join(format!("fan{}_boost", fan_idx));
            if p.exists() {
                self.set_fan_level(sysfs, fan_idx, 0)?;
            }
        }
        let _ = self.set_platform_profile(sysfs, "balanced");
        Ok(())
    }
}
