use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardwareVendor {
    Dell,
    Alienware,
    Asus,
    Lenovo,
    Hp,
    Acer,
    Msi,
    Framework,
    System76,
    Apple,
    GenericPc,
    Unknown(String),
}

impl HardwareVendor {
    pub fn from_dmi(vendor_str: &str) -> Self {
        let v = vendor_str.trim().to_lowercase();
        if v.contains("alienware") {
            HardwareVendor::Alienware
        } else if v.contains("dell") {
            HardwareVendor::Dell
        } else if v.contains("asus") || v.contains("asustek") {
            HardwareVendor::Asus
        } else if v.contains("lenovo") {
            HardwareVendor::Lenovo
        } else if v.contains("hp") || v.contains("hewlett-packard") {
            HardwareVendor::Hp
        } else if v.contains("acer") {
            HardwareVendor::Acer
        } else if v.contains("micro-star") || v.contains("msi") {
            HardwareVendor::Msi
        } else if v.contains("framework") {
            HardwareVendor::Framework
        } else if v.contains("system76") {
            HardwareVendor::System76
        } else if v.contains("apple") {
            HardwareVendor::Apple
        } else if v.is_empty() || v == "unknown" || v == "system manufacturer" {
            HardwareVendor::GenericPc
        } else {
            HardwareVendor::Unknown(vendor_str.trim().to_string())
        }
    }
}

impl fmt::Display for HardwareVendor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HardwareVendor::Dell => write!(f, "Dell Inc."),
            HardwareVendor::Alienware => write!(f, "Alienware"),
            HardwareVendor::Asus => write!(f, "ASUSTeK COMPUTER INC."),
            HardwareVendor::Lenovo => write!(f, "Lenovo"),
            HardwareVendor::Hp => write!(f, "HP Inc."),
            HardwareVendor::Acer => write!(f, "Acer Inc."),
            HardwareVendor::Msi => write!(f, "Micro-Star International (MSI)"),
            HardwareVendor::Framework => write!(f, "Framework Computer Inc."),
            HardwareVendor::System76 => write!(f, "System76"),
            HardwareVendor::Apple => write!(f, "Apple Inc."),
            HardwareVendor::GenericPc => write!(f, "Generic PC / Desktop Motherboard"),
            HardwareVendor::Unknown(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChassisType {
    Laptop,
    Desktop,
    Workstation,
    Server,
    Handheld,
    Unknown,
}

impl ChassisType {
    pub fn from_dmi(chassis_str: &str) -> Self {
        let c = chassis_str.trim();
        match c {
            "8" | "9" | "10" | "14" | "30" | "31" | "32" => ChassisType::Laptop,
            "3" | "4" | "5" | "6" | "7" | "15" | "16" => ChassisType::Desktop,
            "17" | "23" | "28" => ChassisType::Server,
            "18" => ChassisType::Workstation,
            "11" => ChassisType::Handheld,
            _ => {
                let lower = c.to_lowercase();
                if lower.contains("laptop")
                    || lower.contains("notebook")
                    || lower.contains("portable")
                {
                    ChassisType::Laptop
                } else if lower.contains("desktop")
                    || lower.contains("tower")
                    || lower.contains("mini")
                {
                    ChassisType::Desktop
                } else if lower.contains("server") {
                    ChassisType::Server
                } else {
                    ChassisType::Unknown
                }
            }
        }
    }
}

impl fmt::Display for ChassisType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChassisType::Laptop => write!(f, "Laptop / Portable"),
            ChassisType::Desktop => write!(f, "Desktop PC"),
            ChassisType::Workstation => write!(f, "Workstation"),
            ChassisType::Server => write!(f, "Server"),
            ChassisType::Handheld => write!(f, "Handheld Console"),
            ChassisType::Unknown => write!(f, "Unknown Chassis"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationTier {
    Tier1PhysicallyVerified,
    Tier2CommunitySupported,
    Tier3GenericLinuxFallback,
    Tier4Unsupported,
}

#[allow(non_upper_case_globals)]
impl VerificationTier {
    pub const PhysicallyVerified: VerificationTier = VerificationTier::Tier1PhysicallyVerified;
    pub const CommunitySupported: VerificationTier = VerificationTier::Tier2CommunitySupported;
    pub const GenericLinuxFallback: VerificationTier = VerificationTier::Tier3GenericLinuxFallback;
    pub const Unsupported: VerificationTier = VerificationTier::Tier4Unsupported;
}

impl fmt::Display for VerificationTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerificationTier::Tier1PhysicallyVerified => {
                write!(f, "Tier 1: Physically Verified Hardware")
            }
            VerificationTier::Tier2CommunitySupported => {
                write!(f, "Tier 2: Community Supported Hardware")
            }
            VerificationTier::Tier3GenericLinuxFallback => {
                write!(f, "Tier 3: Generic Linux Upstream Fallback")
            }
            VerificationTier::Tier4Unsupported => write!(f, "Tier 4: Unsupported Hardware"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FanControlCapability {
    None,
    BoostOffset { max_boost: u8 },
    StandardPwm { min: u8, max: u8 },
    DiscreteLevels { levels: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlatformProfileCapability {
    Supported {
        provider: String,
        current: String,
        choices: Vec<String>,
    },
    NotSupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CpuGovernorCapability {
    Supported {
        current_governor: String,
        available_governors: Vec<String>,
        current_epp: Option<String>,
        available_epp: Vec<String>,
    },
    NotSupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuTelemetryCapability {
    Nvidia {
        count: usize,
        driver_version: String,
    },
    Amd {
        count: usize,
    },
    Intel {
        count: usize,
    },
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareCapabilities {
    pub vendor: HardwareVendor,
    pub vendor_name: String,
    pub product_name: String,
    pub product: String,
    pub bios_version: String,
    pub chassis: ChassisType,
    pub verification_tier: VerificationTier,
    pub tier: VerificationTier,
    pub platform_profile: PlatformProfileCapability,
    pub fan_control: FanControlCapability,
    pub cpu_governor: CpuGovernorCapability,
    pub gpu_telemetry: GpuTelemetryCapability,
    pub has_game_shift: bool,
    pub has_vrm_sensor: bool,
    pub has_vrm_telemetry: bool,
    pub has_ambient_sensor: bool,
    pub has_ambient_telemetry: bool,
    pub has_gpu_telemetry: bool,
    pub powercap_supported: bool,
    pub secure_boot_active: bool,
    pub active_provider_name: String,
}

impl HardwareCapabilities {
    pub fn has_manual_fan_control(&self) -> bool {
        !matches!(self.fan_control, FanControlCapability::None)
    }

    pub fn fan_control_type_str(&self) -> &'static str {
        match self.fan_control {
            FanControlCapability::None => "None (Firmware Managed)",
            FanControlCapability::BoostOffset { .. } => "Boost Offset (0-255)",
            FanControlCapability::StandardPwm { .. } => "Standard HWMON PWM (0-255)",
            FanControlCapability::DiscreteLevels { .. } => "Discrete Profile Levels",
        }
    }

    pub fn unsupported(vendor_name: &str, product_name: &str, bios_version: &str) -> Self {
        Self {
            vendor: HardwareVendor::from_dmi(vendor_name),
            vendor_name: vendor_name.to_string(),
            product_name: product_name.to_string(),
            product: product_name.to_string(),
            bios_version: bios_version.to_string(),
            chassis: ChassisType::Unknown,
            verification_tier: VerificationTier::Unsupported,
            tier: VerificationTier::Unsupported,
            platform_profile: PlatformProfileCapability::NotSupported,
            fan_control: FanControlCapability::None,
            cpu_governor: CpuGovernorCapability::NotSupported,
            gpu_telemetry: GpuTelemetryCapability::None,
            has_game_shift: false,
            has_vrm_sensor: false,
            has_vrm_telemetry: false,
            has_ambient_sensor: false,
            has_ambient_telemetry: false,
            has_gpu_telemetry: false,
            powercap_supported: false,
            secure_boot_active: false,
            active_provider_name: "none".to_string(),
        }
    }

    pub fn generic_fallback(vendor_name: &str, product_name: &str, bios_version: &str) -> Self {
        Self {
            vendor: HardwareVendor::from_dmi(vendor_name),
            vendor_name: vendor_name.to_string(),
            product_name: product_name.to_string(),
            product: product_name.to_string(),
            bios_version: bios_version.to_string(),
            chassis: ChassisType::Unknown,
            verification_tier: VerificationTier::GenericLinuxFallback,
            tier: VerificationTier::GenericLinuxFallback,
            platform_profile: PlatformProfileCapability::NotSupported,
            fan_control: FanControlCapability::None,
            cpu_governor: CpuGovernorCapability::NotSupported,
            gpu_telemetry: GpuTelemetryCapability::None,
            has_game_shift: false,
            has_vrm_sensor: false,
            has_vrm_telemetry: false,
            has_ambient_sensor: false,
            has_ambient_telemetry: false,
            has_gpu_telemetry: false,
            powercap_supported: false,
            secure_boot_active: false,
            active_provider_name: "generic_linux".to_string(),
        }
    }
}
