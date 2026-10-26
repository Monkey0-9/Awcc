use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GModeStatus {
    Active,
    Inactive,
    NotExposed,
    Unsupported,
}

impl std::fmt::Display for GModeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GModeStatus::Active => write!(f, "ACTIVE"),
            GModeStatus::Inactive => write!(f, "INACTIVE"),
            GModeStatus::NotExposed => write!(
                f,
                "Not directly exposed in sysfs (Actuated via Performance Profile + 255 Boost)"
            ),
            GModeStatus::Unsupported => write!(f, "Unsupported on this platform"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThermalSensor {
    pub label: String,
    pub temp_c: Option<f32>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThermalTelemetry {
    pub cpu_pkg_temp: Option<f32>,
    pub cpu_core_max_temp: Option<f32>,
    pub gpu_temp: Option<f32>,
    pub vrm_temp: Option<f32>,
    pub ambient_temp: Option<f32>,
    pub sensors: Vec<ThermalSensor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanTelemetry {
    pub fan_index: u8,
    pub label: String,
    pub requested_boost: u8,
    pub readback_boost: Option<u8>,
    pub current_rpm: Option<u32>,
    pub reported_min_rpm: Option<u32>,
    pub reported_max_rpm: Option<u32>,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemTelemetry {
    pub dmi_vendor: String,
    pub dmi_product: String,
    pub bios_version: String,
    pub active_platform_profile: String,
    pub platform_profile_choices: Vec<String>,
    pub platform_profile_provider: Option<String>,
    pub gmode_status: GModeStatus,
    pub hwmon_alienware_path: Option<String>,
    pub hwmon_dell_smm_path: Option<String>,
    pub thermals: ThermalTelemetry,
    pub fans: Vec<FanTelemetry>,
}
