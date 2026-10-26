use crate::error::AwccError;
use crate::telemetry::{FanTelemetry, SystemTelemetry, ThermalTelemetry};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct HardwareManager {
    alienware_hwmon: Option<PathBuf>,
    dell_smm_hwmon: Option<PathBuf>,
    coretemp_hwmon: Option<PathBuf>,
    dell_ddv_hwmon: Option<PathBuf>,
}

impl HardwareManager {
    pub fn new() -> Self {
        let (alienware_hwmon, dell_smm_hwmon, coretemp_hwmon, dell_ddv_hwmon) =
            Self::discover_hwmon_nodes();
        Self {
            alienware_hwmon,
            dell_smm_hwmon,
            coretemp_hwmon,
            dell_ddv_hwmon,
        }
    }

    pub fn is_driver_loaded(&self) -> bool {
        self.alienware_hwmon.is_some()
    }

    pub fn discover_hwmon_nodes() -> (
        Option<PathBuf>,
        Option<PathBuf>,
        Option<PathBuf>,
        Option<PathBuf>,
    ) {
        let hwmon_dir = Path::new("/sys/class/hwmon");
        let mut alienware = None;
        let mut dell_smm = None;
        let mut coretemp = None;
        let mut dell_ddv = None;

        if let Ok(entries) = fs::read_dir(hwmon_dir) {
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

    pub fn get_dmi_info() -> (String, String, String) {
        let read = |p: &str| {
            fs::read_to_string(p)
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| "Unknown".to_string())
        };
        (
            read("/sys/devices/virtual/dmi/id/sys_vendor"),
            read("/sys/devices/virtual/dmi/id/product_name"),
            read("/sys/devices/virtual/dmi/id/bios_version"),
        )
    }

    pub fn get_platform_profile() -> (String, Vec<String>) {
        let current = fs::read_to_string("/sys/firmware/acpi/platform_profile")
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "Unknown".to_string());
        let choices = fs::read_to_string("/sys/firmware/acpi/platform_profile_choices")
            .map(|s| s.split_whitespace().map(|x| x.to_string()).collect())
            .unwrap_or_default();
        (current, choices)
    }

    pub fn set_platform_profile(&self, profile: &str) -> Result<(), AwccError> {
        let path = "/sys/firmware/acpi/platform_profile";
        fs::write(path, profile).map_err(|e| AwccError::Io {
            path: path.to_string(),
            source: e,
        })
    }

    pub fn read_thermal_telemetry(&self) -> ThermalTelemetry {
        let mut cpu_pkg_temp = 0.0;
        let mut cpu_core_max_temp = 0.0;
        let mut vrm_temp = None;
        let mut ambient_temp = None;

        // Ingest from coretemp
        if let Some(ref ct) = self.coretemp_hwmon {
            if let Ok(entries) = fs::read_dir(ct) {
                for entry in entries.flatten() {
                    let fname = entry.file_name().to_string_lossy().to_string();
                    if fname.starts_with("temp") && fname.ends_with("_input") {
                        if let Ok(content) = fs::read_to_string(entry.path()) {
                            if let Ok(val) = content.trim().parse::<f32>() {
                                let c = val / 1000.0;
                                if c > cpu_core_max_temp {
                                    cpu_core_max_temp = c;
                                }
                                if cpu_pkg_temp == 0.0 {
                                    cpu_pkg_temp = c;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Ingest from dell_ddv
        if let Some(ref ddv) = self.dell_ddv_hwmon {
            if let Ok(content) = fs::read_to_string(ddv.join("temp1_input")) {
                if let Ok(val) = content.trim().parse::<f32>() {
                    vrm_temp = Some(val / 1000.0);
                }
            }
            if let Ok(content) = fs::read_to_string(ddv.join("temp2_input")) {
                if let Ok(val) = content.trim().parse::<f32>() {
                    ambient_temp = Some(val / 1000.0);
                }
            }
        }

        // Query NVIDIA GPU temperature via nvidia-smi
        let gpu_temp = Command::new("nvidia-smi")
            .args(["--query-gpu=temperature.gpu", "--format=csv,noheader,nounits"])
            .output()
            .ok()
            .and_then(|out| {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                s.parse::<f32>().ok()
            });

        ThermalTelemetry {
            cpu_pkg_temp,
            cpu_core_max_temp,
            gpu_temp,
            vrm_temp,
            ambient_temp,
        }
    }

    pub fn read_fan_rpm(&self, fan_index: u8) -> u32 {
        if let Some(ref aw) = self.alienware_hwmon {
            let p = aw.join(format!("fan{}_input", fan_index));
            if let Ok(s) = fs::read_to_string(&p) {
                if let Ok(val) = s.trim().parse::<u32>() {
                    return val;
                }
            }
        }
        if let Some(ref smm) = self.dell_smm_hwmon {
            let p = smm.join(format!("fan{}_input", fan_index));
            if let Ok(s) = fs::read_to_string(&p) {
                if let Ok(val) = s.trim().parse::<u32>() {
                    return val;
                }
            }
        }
        0
    }

    pub fn read_fan_boost(&self, fan_index: u8) -> Result<u8, AwccError> {
        let aw = self.alienware_hwmon.as_ref().ok_or(AwccError::DriverNotFound)?;
        let path = aw.join(format!("fan{}_boost", fan_index));
        let content = fs::read_to_string(&path).map_err(|e| AwccError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
        content.trim().parse::<u8>().map_err(|_| AwccError::ParseInt {
            path: path.display().to_string(),
            content,
        })
    }

    pub fn write_fan_boost(&self, fan_index: u8, boost: u8) -> Result<(), AwccError> {
        let aw = self.alienware_hwmon.as_ref().ok_or(AwccError::DriverNotFound)?;
        let path = aw.join(format!("fan{}_boost", fan_index));
        fs::write(&path, boost.to_string()).map_err(|e| AwccError::Io {
            path: path.display().to_string(),
            source: e,
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

    pub fn is_gmode_active(&self) -> bool {
        let (profile, _) = Self::get_platform_profile();
        profile == "performance"
    }

    pub fn get_telemetry(&self) -> SystemTelemetry {
        let (dmi_vendor, dmi_product, bios_version) = Self::get_dmi_info();
        let (active_platform_profile, platform_profile_choices) = Self::get_platform_profile();
        let gmode_active = self.is_gmode_active();
        let thermals = self.read_thermal_telemetry();

        let mut fans = Vec::new();
        for fan_index in 1..=2 {
            let readback_boost = self.read_fan_boost(fan_index).unwrap_or(0);
            let current_rpm = self.read_fan_rpm(fan_index);
            let label = if fan_index == 1 { "Fan 1 (CPU)" } else { "Fan 2 (GPU)" };

            fans.push(FanTelemetry {
                fan_index,
                label: label.to_string(),
                requested_boost: readback_boost,
                readback_boost,
                current_rpm,
                reported_max_rpm: Some(3600),
            });
        }

        SystemTelemetry {
            dmi_vendor,
            dmi_product,
            bios_version,
            active_platform_profile,
            platform_profile_choices,
            gmode_active,
            hwmon_alienware_path: self.alienware_hwmon.as_ref().map(|p| p.display().to_string()),
            hwmon_dell_smm_path: self.dell_smm_hwmon.as_ref().map(|p| p.display().to_string()),
            thermals,
            fans,
        }
    }
}
