use crate::capabilities::{
    FanControlCapability, HardwareCapabilities, HardwareVendor, VerificationTier,
};
use crate::error::AwccError;
use crate::providers::{generic::GenericLinuxProvider, HardwareProvider, ProviderMatch};
use crate::telemetry::SystemTelemetry;
use std::fs;
use std::path::Path;

pub struct AcerProvider {
    generic: GenericLinuxProvider,
}

impl Default for AcerProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AcerProvider {
    pub fn new() -> Self {
        Self {
            generic: GenericLinuxProvider::new(),
        }
    }

    fn find_acer_driver(sysfs: &Path) -> bool {
        sysfs.join("sys/devices/platform/acer-wmi").exists()
            || sysfs.join("sys/bus/platform/drivers/acer-wmi").exists()
    }

    fn find_turbo_mode(sysfs: &Path) -> Option<std::path::PathBuf> {
        let direct = sysfs.join("sys/devices/platform/acer-wmi/turbo_mode");
        if direct.exists() {
            return Some(direct);
        }
        let pred4 = sysfs.join("sys/devices/platform/acer-wmi/predator_v4/turbo_mode");
        if pred4.exists() {
            return Some(pred4);
        }
        let gaming = sysfs.join("sys/devices/platform/acer-wmi/gaming_mode");
        if gaming.exists() {
            return Some(gaming);
        }
        None
    }
}

impl HardwareProvider for AcerProvider {
    fn name(&self) -> &'static str {
        "acer_wmi"
    }

    fn vendor(&self) -> HardwareVendor {
        HardwareVendor::Acer
    }

    fn detect(&self, sysfs: &Path) -> Option<ProviderMatch> {
        let vendor_str = fs::read_to_string(sysfs.join("sys/devices/virtual/dmi/id/sys_vendor"))
            .unwrap_or_default()
            .to_lowercase();
        let is_acer = vendor_str.contains("acer");

        if is_acer && Self::find_acer_driver(sysfs) {
            Some(ProviderMatch {
                priority: 85,
                description: "Acer Predator / Nitro (acer-wmi driver active)".to_string(),
            })
        } else if is_acer {
            Some(ProviderMatch {
                priority: 45,
                description: "Acer device detected (upstream generic fallback)".to_string(),
            })
        } else {
            None
        }
    }

    fn capabilities(&self, sysfs: &Path) -> HardwareCapabilities {
        let mut caps = self.generic.capabilities(sysfs);
        caps.vendor = HardwareVendor::Acer;
        caps.verification_tier = VerificationTier::Tier2CommunitySupported;
        caps.tier = VerificationTier::Tier2CommunitySupported;
        caps.active_provider_name = self.name().to_string();

        if Self::find_turbo_mode(sysfs).is_some() {
            caps.fan_control = FanControlCapability::DiscreteLevels {
                levels: vec![
                    "0 (Normal / Auto)".to_string(),
                    "1 (Predator Turbo Boost)".to_string(),
                ],
            };
        }
        caps
    }

    fn read_telemetry(&self, sysfs: &Path) -> SystemTelemetry {
        self.generic.read_telemetry(sysfs)
    }

    fn set_platform_profile(&self, sysfs: &Path, profile: &str) -> Result<(), AwccError> {
        self.generic.set_platform_profile(sysfs, profile)
    }

    fn set_fan_level(&self, sysfs: &Path, fan_index: u8, level: u8) -> Result<(), AwccError> {
        if let Some(turbo_path) = Self::find_turbo_mode(sysfs) {
            let val = if level >= 200 { "1\n" } else { "0\n" };
            fs::write(&turbo_path, val).map_err(|e| AwccError::Io {
                path: turbo_path.display().to_string(),
                source: e,
            })?;
            return Ok(());
        }
        self.generic.set_fan_level(sysfs, fan_index, level)
    }

    fn set_max_fans(&self, sysfs: &Path) -> Result<(), AwccError> {
        if let Some(turbo_path) = Self::find_turbo_mode(sysfs) {
            return fs::write(&turbo_path, "1\n").map_err(|e| AwccError::Io {
                path: turbo_path.display().to_string(),
                source: e,
            });
        }
        self.generic.set_max_fans(sysfs)
    }

    fn release_fans(&self, sysfs: &Path) -> Result<(), AwccError> {
        if let Some(turbo_path) = Self::find_turbo_mode(sysfs) {
            return fs::write(&turbo_path, "0\n").map_err(|e| AwccError::Io {
                path: turbo_path.display().to_string(),
                source: e,
            });
        }
        self.generic.release_fans(sysfs)
    }
}
