use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalTelemetry {
    pub cpu_pkg_temp: f32,
    pub cpu_core_max_temp: f32,
    pub gpu_temp: Option<f32>,
    pub vrm_temp: Option<f32>,
    pub ambient_temp: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanTelemetry {
    pub fan_index: u8,
    pub label: String,
    pub requested_boost: u8,
    pub readback_boost: u8,
    pub current_rpm: u32,
    pub reported_max_rpm: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemTelemetry {
    pub dmi_vendor: String,
    pub dmi_product: String,
    pub bios_version: String,
    pub active_platform_profile: String,
    pub platform_profile_choices: Vec<String>,
    pub gmode_active: bool,
    pub hwmon_alienware_path: Option<String>,
    pub hwmon_dell_smm_path: Option<String>,
    pub thermals: ThermalTelemetry,
    pub fans: Vec<FanTelemetry>,
}
