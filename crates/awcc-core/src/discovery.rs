use crate::capabilities::HardwareCapabilities;
use crate::error::AwccError;
use crate::providers::{HardwareProvider, ProviderMatch, ProviderRegistry};
use crate::telemetry::{
    CpuComputeTelemetry, FanTelemetry, GModeStatus, GpuComputeTelemetry, MemoryTelemetry,
    PowerTelemetry, SystemTelemetry, ThermalSensor, ThermalTelemetry,
};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::Instant;

/// Canonical 5-tier performance and thermal modes with gradual 20% fan boost increments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PerformanceMode {
    /// 20% Fan Boost (51/255) + Quiet ACPI Platform Profile (~1,000-1,500 RPM)
    Battery,
    /// 40% Fan Boost (102/255) + Quiet ACPI Platform Profile (~2,000-2,500 RPM)
    Quiet,
    /// 60% Fan Boost (153/255) + Balanced ACPI Platform Profile (~3,000-3,500 RPM)
    Balanced,
    /// 80% Fan Boost (204/255) + Performance ACPI Platform Profile (~4,000-4,500 RPM)
    Performance,
    /// 100% Fan Boost (255/255 Max RPM) + Performance ACPI Platform Profile + Game Shift
    Ultra,
}

impl PerformanceMode {
    /// Fan boost level (0-255). Increases in 20% increments (51) to 255 (max RPM).
    pub fn target_boost(&self) -> u8 {
        match self {
            Self::Battery => 51,      // 20% of 255
            Self::Quiet => 102,       // 40% of 255
            Self::Balanced => 153,    // 60% of 255
            Self::Performance => 204, // 80% of 255
            Self::Ultra => 255,       // 100% of 255 (Max Boost)
        }
    }

    /// Boost percentage (20%, 40%, 60%, 80%, 100%).
    pub fn boost_percentage(&self) -> u8 {
        match self {
            Self::Battery => 20,
            Self::Quiet => 40,
            Self::Balanced => 60,
            Self::Performance => 80,
            Self::Ultra => 100,
        }
    }

    /// Underlying Linux ACPI platform profile name.
    pub fn acpi_profile(&self) -> &'static str {
        match self {
            Self::Battery => "quiet",
            Self::Quiet => "quiet",
            Self::Balanced => "balanced",
            Self::Performance => "performance",
            Self::Ultra => "performance",
        }
    }

    /// Human-friendly display label.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Battery => "Battery Saver",
            Self::Quiet => "Quiet",
            Self::Balanced => "Balanced",
            Self::Performance => "Performance",
            Self::Ultra => "Ultra Performance",
        }
    }

    /// Parse case-insensitive string into a PerformanceMode variant.
    pub fn parse_mode(s: &str) -> Option<Self> {
        let cleaned = s.trim().to_lowercase().replace(['-', '_', ' '], "");
        match cleaned.as_str() {
            "battery" | "bat" | "powersave" | "low" | "1" => Some(Self::Battery),
            "quiet" | "silent" | "2" => Some(Self::Quiet),
            "balanced" | "balance" | "normal" | "3" => Some(Self::Balanced),
            "performance" | "perf" | "high" | "4" | "5" => Some(Self::Performance),
            "ultra" | "gmode" | "max" | "6" => Some(Self::Ultra),
            _ => None,
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        Self::parse_mode(s)
    }
}

impl std::str::FromStr for PerformanceMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_mode(s).ok_or(())
    }
}

pub struct HardwareManager {
    sysfs_root: PathBuf,
    alienware_hwmon: Option<PathBuf>,
    dell_smm_hwmon: Option<PathBuf>,
    coretemp_hwmon: Option<PathBuf>,
    dell_ddv_hwmon: Option<PathBuf>,
    registry: ProviderRegistry,
    #[allow(dead_code)]
    prev_cpu_stat: Mutex<Option<(f64, f64)>>,
    #[allow(dead_code)]
    prev_rapl_stat: Mutex<Option<(u64, Instant)>>,
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
            prev_cpu_stat: Mutex::new(None),
            prev_rapl_stat: Mutex::new(None),
        }
    }

    pub fn is_driver_loaded(&self) -> bool {
        self.alienware_hwmon.is_some() || Self::find_alienware_hwmon(&self.sysfs_root).is_some()
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

    pub fn find_alienware_hwmon(sysfs_root: &Path) -> Option<PathBuf> {
        Self::discover_hwmon_nodes_at(sysfs_root).0
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

        if let Err(e) = fs::write(&prof_file, target) {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                // Try passwordless sudo tee if configured in sudoers
                if let Ok(mut c) = Command::new("sudo")
                    .args(["-n", "tee", prof_file.to_str().unwrap_or("")])
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                {
                    if let Some(mut stdin) = c.stdin.take() {
                        let _ = stdin.write_all(target.as_bytes());
                    }
                    if c.wait().map(|s| s.success()).unwrap_or(false) {
                        let readback = fs::read_to_string(&prof_file)
                            .map(|s| s.trim().to_string())
                            .unwrap_or_default();
                        if readback == target {
                            return Ok(());
                        }
                    }
                }

                // Try fallback to /sys/firmware/acpi/platform_profile if it exists and is writable
                let acpi_fallback = self.sysfs_root.join("sys/firmware/acpi/platform_profile");
                if acpi_fallback.exists() && acpi_fallback != prof_file {
                    if fs::write(&acpi_fallback, target).is_ok() {
                        let readback = fs::read_to_string(&acpi_fallback)
                            .map(|s| s.trim().to_string())
                            .unwrap_or_default();
                        if readback == target {
                            return Ok(());
                        }
                    }

                    if let Ok(mut c) = Command::new("sudo")
                        .args(["-n", "tee", acpi_fallback.to_str().unwrap_or("")])
                        .stdin(std::process::Stdio::piped())
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .spawn()
                    {
                        if let Some(mut stdin) = c.stdin.take() {
                            let _ = stdin.write_all(target.as_bytes());
                        }
                        if c.wait().map(|s| s.success()).unwrap_or(false) {
                            let readback = fs::read_to_string(&acpi_fallback)
                                .map(|s| s.trim().to_string())
                                .unwrap_or_default();
                            if readback == target {
                                return Ok(());
                            }
                        }
                    }
                }

                return Err(AwccError::PermissionDenied {
                    path: prof_file.display().to_string(),
                });
            } else {
                return Err(AwccError::Io {
                    path: prof_file.display().to_string(),
                    source: e,
                });
            }
        }

        // Verify readback
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

    pub fn read_thermal_telemetry(&self) -> ThermalTelemetry {
        let mut cpu_pkg_temp = None;
        let mut cpu_core_max_temp = None;
        let mut vrm_temp = None;
        let mut ambient_temp = None;
        let mut ram_temp = None;
        let mut ram_dimm2_temp = None;
        let mut charger_temp = None;
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

        // 2. Ingest from dell_ddv (Alienware & Dell Data Vault)
        let mut gpu_temp_fallback = None;
        if let Some(ref ddv) = self.dell_ddv_hwmon {
            for idx in 1..=10 {
                let input_path = ddv.join(format!("temp{}_input", idx));
                if !input_path.exists() {
                    continue;
                }
                let label_path = ddv.join(format!("temp{}_label", idx));
                let label = fs::read_to_string(&label_path)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|_| format!("Thermal Sensor {}", idx));

                if let Ok(content) = fs::read_to_string(&input_path) {
                    if let Ok(val) = content.trim().parse::<f32>() {
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
            if let Some(ref smm) = self.dell_smm_hwmon {
                for (idx, target, default_label) in [
                    (2, &mut vrm_temp, "Motherboard VRM"),
                    (3, &mut ambient_temp, "Ambient Air"),
                    (5, &mut ram_temp, "SODIMM 1"),
                    (6, &mut ram_dimm2_temp, "SODIMM 2"),
                    (7, &mut charger_temp, "Charger VRM"),
                ] {
                    let input_path = smm.join(format!("temp{}_input", idx));
                    if input_path.exists() && target.is_none() {
                        if let Ok(content) = fs::read_to_string(&input_path) {
                            if let Ok(val) = content.trim().parse::<f32>() {
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
                            if gpu_temp_fallback.is_none() && label.eq_ignore_ascii_case("GPU") {
                                gpu_temp_fallback = Some(c);
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
            })
            .or(gpu_temp_fallback);

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
            ram_temp,
            ram_dimm2_temp,
            charger_temp,
            sensors,
        }
    }

    pub fn discover_fans(&self) -> Vec<FanTelemetry> {
        let mut fans = Vec::new();
        let aw_buf;
        let aw_opt = match self.alienware_hwmon.as_ref() {
            Some(aw) => Some(aw),
            None => {
                aw_buf = Self::find_alienware_hwmon(&self.sysfs_root);
                aw_buf.as_ref()
            }
        };

        if let Some(aw) = aw_opt {
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

        // Fallback to dell_smm hwmon tachometers if alienware_wmi is not loaded
        if fans.is_empty() {
            if let Some(ref smm) = self.dell_smm_hwmon {
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
        if let Some(ref smm) = self.dell_smm_hwmon {
            let p = smm.join(format!("fan{}_input", fan_index));
            if let Ok(s) = fs::read_to_string(&p) {
                if let Ok(val) = s.trim().parse::<u32>() {
                    return Some(val);
                }
            }
        }
        None
    }

    pub fn read_fan_boost(&self, fan_index: u8) -> Result<u8, AwccError> {
        let aw_buf;
        let aw = match self.alienware_hwmon.as_ref() {
            Some(aw) => aw,
            None => {
                aw_buf = Self::find_alienware_hwmon(&self.sysfs_root)
                    .ok_or(AwccError::DriverNotFound)?;
                &aw_buf
            }
        };
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
        let aw_buf;
        let aw = match self.alienware_hwmon.as_ref() {
            Some(aw) => aw,
            None => {
                aw_buf = Self::find_alienware_hwmon(&self.sysfs_root)
                    .ok_or(AwccError::DriverNotFound)?;
                &aw_buf
            }
        };
        let path = aw.join(format!("fan{}_boost", fan_index));

        if !path.exists() {
            return Err(AwccError::InvalidFanIndex(fan_index));
        }

        if let Err(e) = fs::write(&path, boost.to_string()) {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                if let Ok(mut c) = Command::new("sudo")
                    .args(["-n", "tee", path.to_str().unwrap_or("")])
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                {
                    if let Some(mut stdin) = c.stdin.take() {
                        let _ = stdin.write_all(boost.to_string().as_bytes());
                    }
                    if !c.wait().map(|s| s.success()).unwrap_or(false) {
                        return Err(AwccError::PermissionDenied {
                            path: path.display().to_string(),
                        });
                    }
                } else {
                    return Err(AwccError::PermissionDenied {
                        path: path.display().to_string(),
                    });
                }
            } else {
                return Err(AwccError::Io {
                    path: path.display().to_string(),
                    source: e,
                });
            }
        }

        let readback = self.read_fan_boost(fan_index)?;
        let verified = if boost == 255 {
            readback >= 240
        } else {
            readback == boost
        };
        if !verified {
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

    pub fn read_cpu_compute_telemetry(&self) -> Option<CpuComputeTelemetry> {
        let cpuinfo_path = self.sysfs_root.join("proc/cpuinfo");
        let content = fs::read_to_string(&cpuinfo_path)
            .or_else(|_| fs::read_to_string("/proc/cpuinfo"))
            .ok()?;

        let mut model_name = "CPU".to_string();
        let mut core_count = 0;
        let mut mhz_sum = 0.0;
        let mut mhz_count = 0;

        for line in content.lines() {
            if line.starts_with("model name") {
                if let Some(pos) = line.find(':') {
                    if model_name == "CPU" {
                        let raw = line[pos + 1..].trim();
                        let cleaned = raw
                            .replace("(R)", "")
                            .replace("(TM)", "")
                            .replace("Processor", "")
                            .replace("CPU", "")
                            .trim()
                            .to_string();

                        if let Some(idx) = cleaned
                            .find("i3-")
                            .or_else(|| cleaned.find("i5-"))
                            .or_else(|| cleaned.find("i7-"))
                            .or_else(|| cleaned.find("i9-"))
                            .or_else(|| cleaned.find("Ultra "))
                            .or_else(|| cleaned.find("Ryzen "))
                        {
                            let part = &cleaned[idx..];
                            let words: Vec<&str> = part.split_whitespace().collect();
                            if !words.is_empty() {
                                if words[0].starts_with("Ryzen") && words.len() >= 3 {
                                    model_name = format!("{} {} {}", words[0], words[1], words[2]);
                                } else {
                                    model_name = words[0].to_string();
                                }
                            } else {
                                model_name = cleaned;
                            }
                        } else {
                            let words: Vec<&str> = cleaned.split_whitespace().collect();
                            if words.len() >= 2 {
                                model_name = format!("{} {}", words[0], words[1]);
                            } else {
                                model_name = cleaned;
                            }
                        }
                    }
                }
            } else if line.starts_with("processor") {
                core_count += 1;
            } else if line.starts_with("cpu MHz") {
                if let Some(pos) = line.find(':') {
                    if let Ok(v) = line[pos + 1..].trim().parse::<f32>() {
                        mhz_sum += v;
                        mhz_count += 1;
                    }
                }
            }
        }

        let cur_freq_mhz = if mhz_count > 0 {
            Some(mhz_sum / mhz_count as f32)
        } else {
            None
        };

        let max_freq_path = self
            .sysfs_root
            .join("sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq");
        let max_freq_mhz = fs::read_to_string(&max_freq_path)
            .ok()
            .and_then(|s| s.trim().parse::<f32>().ok())
            .map(|khz| khz / 1000.0);

        let gov_path = self
            .sysfs_root
            .join("sys/devices/system/cpu/cpu0/cpufreq/scaling_governor");
        let scaling_governor = fs::read_to_string(&gov_path)
            .ok()
            .map(|s| s.trim().to_string());

        let epp_path = self
            .sysfs_root
            .join("sys/devices/system/cpu/cpu0/cpufreq/energy_perf_preference");
        let energy_perf_preference = fs::read_to_string(&epp_path)
            .ok()
            .map(|s| s.trim().to_string());

        let stat_path = self.sysfs_root.join("proc/stat");
        let parse_stat = |s: &str| -> Option<(f64, f64)> {
            let first_line = s.lines().next()?;
            let parts: Vec<&str> = first_line.split_whitespace().collect();
            if parts.len() >= 5 && parts[0] == "cpu" {
                let user: f64 = parts[1].parse().ok()?;
                let nice: f64 = parts[2].parse().ok()?;
                let system: f64 = parts[3].parse().ok()?;
                let idle: f64 = parts[4].parse().ok()?;
                let iowait: f64 = parts.get(5).and_then(|p| p.parse().ok()).unwrap_or(0.0);
                let irq: f64 = parts.get(6).and_then(|p| p.parse().ok()).unwrap_or(0.0);
                let softirq: f64 = parts.get(7).and_then(|p| p.parse().ok()).unwrap_or(0.0);
                let total = user + nice + system + idle + iowait + irq + softirq;
                let idle_all = idle + iowait;
                Some((total, idle_all))
            } else {
                None
            }
        };

        let read_stat = || {
            fs::read_to_string(&stat_path)
                .or_else(|_| fs::read_to_string("/proc/stat"))
                .ok()
                .and_then(|s| parse_stat(&s))
        };

        let usage_percent = if let Some((cur_total, cur_idle)) = read_stat() {
            let mut prev_guard = self.prev_cpu_stat.lock().unwrap_or_else(|e| e.into_inner());
            match *prev_guard {
                Some((prev_total, prev_idle)) if cur_total > prev_total => {
                    let diff_total = cur_total - prev_total;
                    let diff_idle = cur_idle - prev_idle;
                    let val = ((diff_total - diff_idle) / diff_total) * 100.0;
                    *prev_guard = Some((cur_total, cur_idle));
                    Some(val.clamp(0.0, 100.0) as f32)
                }
                _ => {
                    // First reading: sleep 25ms to establish instantaneous baseline
                    std::thread::sleep(std::time::Duration::from_millis(25));
                    if let Some((next_total, next_idle)) = read_stat() {
                        let diff_total = next_total - cur_total;
                        let diff_idle = next_idle - cur_idle;
                        let val = if diff_total > 0.0 {
                            ((diff_total - diff_idle) / diff_total) * 100.0
                        } else {
                            0.0
                        };
                        *prev_guard = Some((next_total, next_idle));
                        Some(val.clamp(0.0, 100.0) as f32)
                    } else {
                        *prev_guard = Some((cur_total, cur_idle));
                        None
                    }
                }
            }
        } else {
            None
        };

        let rapl_energy_path = self
            .sysfs_root
            .join("sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj");
        let now = Instant::now();
        let package_power_watts = if let Ok(s1) = fs::read_to_string(&rapl_energy_path) {
            if let Ok(cur_uj) = s1.trim().parse::<u64>() {
                let mut prev_guard = self
                    .prev_rapl_stat
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                match *prev_guard {
                    Some((prev_uj, prev_time)) if cur_uj >= prev_uj => {
                        let diff_uj = cur_uj.saturating_sub(prev_uj);
                        let elapsed = now.duration_since(prev_time).as_secs_f32();
                        *prev_guard = Some((cur_uj, now));
                        if elapsed > 0.05 {
                            Some((diff_uj as f32 / 1_000_000.0) / elapsed)
                        } else {
                            None
                        }
                    }
                    _ => {
                        // First reading: sleep 25ms to establish instantaneous delta
                        std::thread::sleep(std::time::Duration::from_millis(25));
                        if let Ok(s2) = fs::read_to_string(&rapl_energy_path) {
                            if let Ok(next_uj) = s2.trim().parse::<u64>() {
                                let diff_uj = next_uj.saturating_sub(cur_uj);
                                *prev_guard = Some((next_uj, Instant::now()));
                                Some(diff_uj as f32 / 25_000.0)
                            } else {
                                *prev_guard = Some((cur_uj, now));
                                None
                            }
                        } else {
                            *prev_guard = Some((cur_uj, now));
                            None
                        }
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        Some(CpuComputeTelemetry {
            model_name,
            cores_logical: if core_count > 0 { core_count } else { 1 },
            cur_freq_mhz,
            max_freq_mhz,
            scaling_governor,
            energy_perf_preference,
            usage_percent,
            package_power_watts,
        })
    }

    pub fn read_gpu_compute_telemetry(&self) -> Option<GpuComputeTelemetry> {
        let output = Command::new("nvidia-smi")
            .args([
                "--query-gpu=name,utilization.gpu,clocks.current.graphics,clocks.current.memory,memory.used,memory.total,power.draw.average,power.draw.instant,enforced.power.limit",
                "--format=csv,noheader,nounits",
            ])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let line = stdout.lines().next()?;
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if parts.is_empty() {
            return None;
        }

        let raw_name = parts[0];
        let model_name = raw_name
            .replace("NVIDIA GeForce ", "")
            .replace(" Laptop GPU", "")
            .trim()
            .to_string();

        let utilization_gpu_percent = parts.get(1).and_then(|p| p.parse::<u32>().ok());
        let core_clock_mhz = parts.get(2).and_then(|p| p.parse::<u32>().ok());
        let memory_clock_mhz = parts.get(3).and_then(|p| p.parse::<u32>().ok());
        let memory_used_mb = parts.get(4).and_then(|p| p.parse::<u64>().ok());
        let memory_total_mb = parts.get(5).and_then(|p| p.parse::<u64>().ok());

        let power_draw_watts = parts
            .get(6)
            .and_then(|p| p.parse::<f32>().ok())
            .filter(|&w| w < 200.0)
            .or_else(|| {
                parts
                    .get(7)
                    .and_then(|p| p.parse::<f32>().ok())
                    .filter(|&w| w < 200.0)
            });

        let power_limit_watts = parts.get(8).and_then(|p| p.parse::<f32>().ok());

        Some(GpuComputeTelemetry {
            model_name: Some(model_name),
            core_clock_mhz,
            memory_clock_mhz,
            utilization_gpu_percent,
            memory_used_mb,
            memory_total_mb,
            power_draw_watts,
            power_limit_watts,
        })
    }

    pub fn read_memory_telemetry(&self) -> Option<MemoryTelemetry> {
        let meminfo_path = self.sysfs_root.join("proc/meminfo");
        let content = fs::read_to_string(&meminfo_path)
            .or_else(|_| fs::read_to_string("/proc/meminfo"))
            .ok()?;

        let mut total_kb: Option<u64> = None;
        let mut avail_kb: Option<u64> = None;
        let mut swap_total_kb: Option<u64> = None;
        let mut swap_free_kb: Option<u64> = None;

        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                total_kb = line.split_whitespace().nth(1).and_then(|s| s.parse().ok());
            } else if line.starts_with("MemAvailable:") {
                avail_kb = line.split_whitespace().nth(1).and_then(|s| s.parse().ok());
            } else if line.starts_with("SwapTotal:") {
                swap_total_kb = line.split_whitespace().nth(1).and_then(|s| s.parse().ok());
            } else if line.starts_with("SwapFree:") {
                swap_free_kb = line.split_whitespace().nth(1).and_then(|s| s.parse().ok());
            }
        }

        let ram_total_mb = total_kb.map(|kb| kb / 1024).unwrap_or(0);
        let ram_avail_mb = avail_kb.map(|kb| kb / 1024).unwrap_or(0);
        let ram_used_mb = ram_total_mb.saturating_sub(ram_avail_mb);
        let ram_used_percent = if ram_total_mb > 0 {
            (ram_used_mb as f32 / ram_total_mb as f32) * 100.0
        } else {
            0.0
        };

        let swap_total_mb = swap_total_kb.map(|kb| kb / 1024).unwrap_or(0);
        let swap_free_mb = swap_free_kb.map(|kb| kb / 1024).unwrap_or(0);
        let swap_used_mb = swap_total_mb.saturating_sub(swap_free_mb);

        Some(MemoryTelemetry {
            ram_used_mb,
            ram_total_mb,
            ram_used_percent,
            swap_used_mb,
            swap_total_mb,
        })
    }

    pub fn read_power_telemetry(&self) -> Option<PowerTelemetry> {
        let ac_path = self.sysfs_root.join("sys/class/power_supply/AC/online");
        let ac_online = fs::read_to_string(&ac_path)
            .ok()
            .map(|s| s.trim() == "1")
            .unwrap_or(true);

        let bat_cap_path = self.sysfs_root.join("sys/class/power_supply/BAT0/capacity");
        let battery_percent = fs::read_to_string(&bat_cap_path)
            .ok()
            .and_then(|s| s.trim().parse::<f32>().ok());

        let bat_status_path = self.sysfs_root.join("sys/class/power_supply/BAT0/status");
        let battery_status = fs::read_to_string(&bat_status_path)
            .ok()
            .map(|s| s.trim().to_string());

        let power_now_path = self
            .sysfs_root
            .join("sys/class/power_supply/BAT0/power_now");
        let charge_rate_watts = fs::read_to_string(&power_now_path)
            .ok()
            .and_then(|s| s.trim().parse::<f32>().ok())
            .map(|u| u / 1_000_000.0);

        Some(PowerTelemetry {
            ac_online,
            battery_percent,
            battery_status,
            charge_rate_watts,
        })
    }

    pub fn measure_sysfs_bus_latency(&self) -> u64 {
        let path = self.sysfs_root.join("sys/firmware/acpi/platform_profile");
        let target = if path.exists() {
            path
        } else {
            self.sysfs_root.join("proc/stat")
        };

        let (_, cycles) = crate::hw::rdtsc::measure_cycles(|| {
            let _ = fs::read(&target);
        });

        crate::hw::rdtsc::cycles_to_nanos(cycles)
    }

    pub fn get_telemetry(&self) -> SystemTelemetry {
        let (dmi_vendor, dmi_product, bios_version) = self.get_dmi_info();
        let (active_platform_profile, platform_profile_choices, platform_profile_provider) =
            self.get_platform_profile();
        let gmode_status = self.check_gmode_status();
        let thermals = self.read_thermal_telemetry();
        let fans = self.discover_fans();
        let cpu = self.read_cpu_compute_telemetry();
        let gpu = self.read_gpu_compute_telemetry();
        let memory = self.read_memory_telemetry();
        let power = self.read_power_telemetry();
        let hw_features = Some(crate::hw::cpuid::probe_cpu_features());
        let bus_latency_nanos = Some(self.measure_sysfs_bus_latency());

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
            cpu,
            gpu,
            memory,
            power,
            hw_features,
            bus_latency_nanos,
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

    pub fn set_performance_mode(&self, mode: PerformanceMode) -> Result<(), AwccError> {
        let pprof = mode.acpi_profile();
        self.set_platform_profile(pprof)?;

        let boost = mode.target_boost();
        let fans = self.discover_fans();
        for f in &fans {
            let _ = self.set_fan_level(f.fan_index, boost);
        }

        // Synchronize Dell Game Shift state file
        let state_file = Path::new("/tmp/.awcc_gmode_state");
        if mode == PerformanceMode::Ultra {
            let _ = fs::write(state_file, "ON\n");
        } else {
            let _ = fs::write(state_file, "OFF\n");
        }

        Ok(())
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
