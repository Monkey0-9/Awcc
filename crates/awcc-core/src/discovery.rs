use crate::capabilities::HardwareCapabilities;
use crate::error::AwccError;
use crate::providers::{HardwareProvider, ProviderMatch, ProviderRegistry};
use crate::telemetry::{
    FanTelemetry, GModeStatus, SystemTelemetry, ThermalSensor, ThermalTelemetry,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct HardwareManager {
    sysfs_root: PathBuf,
    alienware_hwmon: Option<PathBuf>,
    dell_smm_hwmon: Option<PathBuf>,
    coretemp_hwmon: Option<PathBuf>,
    dell_ddv_hwmon: Option<PathBuf>,
    registry: ProviderRegistry,
}

impl Default for HardwareManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HardwareManager {
    pub fn new() -> Self {
        Self::with_sysfs_root(PathBuf::from("/"))
    }

    pub fn with_sysfs_root(sysfs_root: PathBuf) -> Self {
        let (alienware_hwmon, dell_smm_hwmon, coretemp_hwmon, dell_ddv_hwmon) =
            Self::discover_hwmon_nodes_at(&sysfs_root);
        Self {
            sysfs_root,
            alienware_hwmon,
            dell_smm_hwmon,
            coretemp_hwmon,
            dell_ddv_hwmon,
            registry: ProviderRegistry::new(),
        }
    }

    pub fn is_driver_loaded(&self) -> bool {
        self.alienware_hwmon.is_some()
    }

    pub fn sysfs_root(&self) -> &Path {
        &self.sysfs_root
    }

    pub fn discover_hwmon_nodes(
        &self,
    ) -> (
        Option<PathBuf>,
        Option<PathBuf>,
        Option<PathBuf>,
        Option<PathBuf>,
    ) {
        Self::discover_hwmon_nodes_at(&self.sysfs_root)
    }

    pub fn discover_hwmon_nodes_at(
        sysfs_root: &Path,
    ) -> (
        Option<PathBuf>,
        Option<PathBuf>,
        Option<PathBuf>,
        Option<PathBuf>,
    ) {
        let hwmon_dir = sysfs_root.join("sys/class/hwmon");
        let mut alienware = None;
        let mut dell_smm = None;
        let mut coretemp = None;
        let mut dell_ddv = None;

        if let Ok(entries) = fs::read_dir(&hwmon_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name_file = path.join("name");
                if let Ok(name_content) = fs::read_to_string(&name_file) {
                    let name = name_content.trim();
                    match name {
                        "alienware_wmi" => alienware = Some(path.clone()),
                        "dell_smm" => dell_smm = Some(path.clone()),
                        "coretemp" => coretemp = Some(path.clone()),
                        "dell_ddv" => dell_ddv = Some(path.clone()),
                        _ => {}
                    }
                }
            }
        }
        (alienware, dell_smm, coretemp, dell_ddv)
    }

    pub fn get_dmi_info(&self) -> (String, String, String) {
        let read = |rel: &str| {
            let p = self.sysfs_root.join(rel);
            fs::read_to_string(&p)
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| "Unknown".to_string())
        };
        (
            read("sys/devices/virtual/dmi/id/sys_vendor"),
            read("sys/devices/virtual/dmi/id/product_name"),
            read("sys/devices/virtual/dmi/id/bios_version"),
        )
    }

    /// Discovers platform-profile interface, prioritizing class provider "alienware-wmi"
    pub fn find_platform_profile_node(&self) -> Option<(PathBuf, String)> {
        let class_dir = self.sysfs_root.join("sys/class/platform-profile");
        if let Ok(entries) = fs::read_dir(&class_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                let name_file = p.join("name");
                if let Ok(name) = fs::read_to_string(&name_file) {
                    if name.trim() == "alienware-wmi" {
                        return Some((p, "alienware-wmi".to_string()));
                    }
                }
            }
            // If alienware-wmi was not found, check other class providers
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
        }

        // Fallback to firmware acpi node
        let acpi_profile = self.sysfs_root.join("sys/firmware/acpi/platform_profile");
        if acpi_profile.exists() {
            Some((
                self.sysfs_root.join("sys/firmware/acpi"),
                "acpi_firmware".to_string(),
            ))
        } else {
            None
        }
    }

    pub fn get_platform_profile(&self) -> (String, Vec<String>, Option<String>) {
        if let Some((node, provider)) = self.find_platform_profile_node() {
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
            (current, choices, Some(provider))
        } else {
            ("Unknown".to_string(), Vec::new(), None)
        }
    }

    pub fn set_platform_profile(&self, requested: &str) -> Result<(), AwccError> {
        let (node, provider) = self
            .find_platform_profile_node()
            .ok_or(AwccError::PlatformProfileNotFound)?;

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

        if !choices.is_empty() && !choices.iter().any(|c| c == requested) {
            return Err(AwccError::InvalidProfile {
                requested: requested.to_string(),
                available: choices,
            });
        }

        fs::write(&prof_file, requested).map_err(|e| {
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

        // Verify readback
        let readback = fs::read_to_string(&prof_file)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        if readback != requested {
            return Err(AwccError::ProfileReadbackMismatch {
                requested: requested.to_string(),
                readback,
            });
        }

        Ok(())
    }

    pub fn read_thermal_telemetry(&self) -> ThermalTelemetry {
        let mut cpu_pkg_temp = None;
        let mut cpu_core_max_temp = None;
        let mut vrm_temp = None;
        let mut ambient_temp = None;
        let mut sensors = Vec::new();

        // 1. Ingest from coretemp
        if let Some(ref ct) = self.coretemp_hwmon {
            if let Ok(entries) = fs::read_dir(ct) {
                for entry in entries.flatten() {
                    let fname = entry.file_name().to_string_lossy().to_string();
                    if fname.starts_with("temp") && fname.ends_with("_input") {
                        let idx = fname.trim_start_matches("temp").trim_end_matches("_input");
                        let label_path = ct.join(format!("temp{}_label", idx));
                        let label = fs::read_to_string(&label_path)
                            .map(|s| s.trim().to_string())
                            .unwrap_or_else(|_| format!("Coretemp {}", idx));

                        if let Ok(content) = fs::read_to_string(entry.path()) {
                            if let Ok(val) = content.trim().parse::<f32>() {
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

                                if let Some(cur_max) = cpu_core_max_temp {
                                    if c > cur_max {
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

        // 2. Ingest from dell_ddv
        if let Some(ref ddv) = self.dell_ddv_hwmon {
            for (idx, target) in [(1, &mut vrm_temp), (2, &mut ambient_temp)] {
                let label_path = ddv.join(format!("temp{}_label", idx));
                let label = fs::read_to_string(&label_path)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|_| if idx == 1 { "VRM" } else { "Ambient" }.to_string());

                if let Ok(content) = fs::read_to_string(ddv.join(format!("temp{}_input", idx))) {
                    if let Ok(val) = content.trim().parse::<f32>() {
                        let c = val / 1000.0;
                        *target = Some(c);
                        sensors.push(ThermalSensor {
                            label,
                            temp_c: Some(c),
                            source: "dell_ddv".to_string(),
                        });
                    }
                }
            }
        }

        // 3. Ingest from alienware_wmi
        if let Some(ref aw) = self.alienware_hwmon {
            for idx in 1..=6 {
                let label_path = aw.join(format!("temp{}_label", idx));
                let input_path = aw.join(format!("temp{}_input", idx));
                if input_path.exists() {
                    let label = fs::read_to_string(&label_path)
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|_| format!("AWCC Temp {}", idx));
                    if let Ok(content) = fs::read_to_string(&input_path) {
                        if let Ok(val) = content.trim().parse::<f32>() {
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

        // 4. Query NVIDIA GPU temperature via nvidia-smi
        let gpu_temp = Command::new("nvidia-smi")
            .args([
                "--query-gpu=temperature.gpu",
                "--format=csv,noheader,nounits",
            ])
            .output()
            .ok()
            .and_then(|out| {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                s.lines()
                    .next()
                    .and_then(|line| line.trim().parse::<f32>().ok())
            });

        if let Some(gt) = gpu_temp {
            sensors.push(ThermalSensor {
                label: "NVIDIA GPU".to_string(),
                temp_c: Some(gt),
                source: "nvidia-smi".to_string(),
            });
        }

        ThermalTelemetry {
            cpu_pkg_temp,
            cpu_core_max_temp,
            gpu_temp,
            vrm_temp,
            ambient_temp,
            sensors,
        }
    }

    pub fn discover_fans(&self) -> Vec<FanTelemetry> {
        let mut fans = Vec::new();
        if let Some(ref aw) = self.alienware_hwmon {
            for fan_index in 1..=6 {
                let boost_path = aw.join(format!("fan{}_boost", fan_index));
                let input_path = aw.join(format!("fan{}_input", fan_index));
                if !boost_path.exists() && !input_path.exists() {
                    continue;
                }

                let label_path = aw.join(format!("fan{}_label", fan_index));
                let label = fs::read_to_string(&label_path)
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

                let min_path = aw.join(format!("fan{}_min", fan_index));
                let reported_min_rpm = fs::read_to_string(&min_path)
                    .ok()
                    .and_then(|s| s.trim().parse::<u32>().ok());

                let max_path = aw.join(format!("fan{}_max", fan_index));
                let reported_max_rpm = fs::read_to_string(&max_path)
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
        fans
    }

    pub fn read_fan_rpm(&self, fan_index: u8) -> Option<u32> {
        if let Some(ref aw) = self.alienware_hwmon {
            let p = aw.join(format!("fan{}_input", fan_index));
            if let Ok(s) = fs::read_to_string(&p) {
                if let Ok(val) = s.trim().parse::<u32>() {
                    return Some(val);
                }
            }
        }
        None
    }

    pub fn read_fan_boost(&self, fan_index: u8) -> Result<u8, AwccError> {
        let aw = self
            .alienware_hwmon
            .as_ref()
            .ok_or(AwccError::DriverNotFound)?;
        let path = aw.join(format!("fan{}_boost", fan_index));
        let content = fs::read_to_string(&path).map_err(|e| {
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

        content
            .trim()
            .parse::<u8>()
            .map_err(|_| AwccError::ParseInt {
                path: path.display().to_string(),
                content,
            })
    }

    pub fn write_fan_boost(&self, fan_index: u8, boost: u8) -> Result<(), AwccError> {
        let aw = self
            .alienware_hwmon
            .as_ref()
            .ok_or(AwccError::DriverNotFound)?;
        let path = aw.join(format!("fan{}_boost", fan_index));

        if !path.exists() {
            return Err(AwccError::InvalidFanIndex(fan_index));
        }

        fs::write(&path, boost.to_string()).map_err(|e| {
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

        let readback = self.read_fan_boost(fan_index)?;
        if readback != boost {
            return Err(AwccError::ReadbackMismatch {
                fan: fan_index,
                requested: boost,
                readback,
            });
        }
        Ok(())
    }

    pub fn check_gmode_status(&self) -> GModeStatus {
        if let Some(ref aw) = self.alienware_hwmon {
            let gmode_sysfs = aw.join("gmode");
            if gmode_sysfs.exists() {
                if let Ok(s) = fs::read_to_string(&gmode_sysfs) {
                    let trimmed = s.trim();
                    if trimmed == "1" || trimmed.eq_ignore_ascii_case("on") {
                        return GModeStatus::Active;
                    } else {
                        return GModeStatus::Inactive;
                    }
                }
            }
        }
        GModeStatus::NotExposed
    }

    pub fn get_telemetry(&self) -> SystemTelemetry {
        let (dmi_vendor, dmi_product, bios_version) = self.get_dmi_info();
        let (active_platform_profile, platform_profile_choices, platform_profile_provider) =
            self.get_platform_profile();
        let gmode_status = self.check_gmode_status();
        let thermals = self.read_thermal_telemetry();
        let fans = self.discover_fans();

        SystemTelemetry {
            dmi_vendor,
            dmi_product,
            bios_version,
            active_platform_profile,
            platform_profile_choices,
            platform_profile_provider,
            gmode_status,
            hwmon_alienware_path: self
                .alienware_hwmon
                .as_ref()
                .map(|p| p.display().to_string()),
            hwmon_dell_smm_path: self
                .dell_smm_hwmon
                .as_ref()
                .map(|p| p.display().to_string()),
            thermals,
            fans,
        }
    }

    pub fn capabilities(&self) -> HardwareCapabilities {
        if let Some((p, _)) = self.registry.find_best_provider(&self.sysfs_root) {
            p.capabilities(&self.sysfs_root)
        } else {
            let gen = crate::providers::generic::GenericLinuxProvider::new();
            crate::providers::HardwareProvider::capabilities(&gen, &self.sysfs_root)
        }
    }

    pub fn best_provider(&self) -> Option<(&dyn HardwareProvider, ProviderMatch)> {
        self.registry.find_best_provider(&self.sysfs_root)
    }

    pub fn set_fan_level(&self, fan_index: u8, level: u8) -> Result<(), AwccError> {
        if self.alienware_hwmon.is_some() {
            self.write_fan_boost(fan_index, level)
        } else if let Some((p, _)) = self.registry.find_best_provider(&self.sysfs_root) {
            p.set_fan_level(&self.sysfs_root, fan_index, level)
        } else {
            Err(AwccError::UnsupportedOperation {
                operation: "set fan level".to_string(),
                reason: "No compatible hardware provider discovered".to_string(),
            })
        }
    }

    pub fn set_max_fans(&self) -> Result<(), AwccError> {
        if self.alienware_hwmon.is_some() {
            let fans = self.discover_fans();
            for f in &fans {
                self.write_fan_boost(f.fan_index, 255)?;
            }
            let _ = self.set_platform_profile("performance");
            Ok(())
        } else if let Some((p, _)) = self.registry.find_best_provider(&self.sysfs_root) {
            p.set_max_fans(&self.sysfs_root)
        } else {
            Err(AwccError::UnsupportedOperation {
                operation: "max fans".to_string(),
                reason: "No compatible hardware provider discovered".to_string(),
            })
        }
    }

    pub fn release_fans(&self) -> Result<(), AwccError> {
        if self.alienware_hwmon.is_some() {
            let fans = self.discover_fans();
            for f in &fans {
                self.write_fan_boost(f.fan_index, 0)?;
            }
            let _ = self.set_platform_profile("balanced");
            Ok(())
        } else if let Some((p, _)) = self.registry.find_best_provider(&self.sysfs_root) {
            p.release_fans(&self.sysfs_root)
        } else {
            Err(AwccError::UnsupportedOperation {
                operation: "release fans".to_string(),
                reason: "No compatible hardware provider discovered".to_string(),
            })
        }
    }
}
