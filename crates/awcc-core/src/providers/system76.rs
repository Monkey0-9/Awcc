use crate::capabilities::{
    FanControlCapability, HardwareCapabilities, HardwareVendor, VerificationTier,
};
use crate::error::AwccError;
use crate::providers::{generic::GenericLinuxProvider, HardwareProvider, ProviderMatch};
use crate::telemetry::SystemTelemetry;
use std::fs;
use std::path::{Path, PathBuf};

pub struct System76Provider {
    generic: GenericLinuxProvider,
}

impl Default for System76Provider {
    fn default() -> Self {
        Self::new()
    }
}

impl System76Provider {
    pub fn new() -> Self {
        Self {
            generic: GenericLinuxProvider::new(),
        }
    }

    fn find_system76_driver(sysfs: &Path) -> bool {
        sysfs.join("sys/devices/platform/system76_acpi").exists()
            || sysfs
                .join("sys/bus/platform/drivers/system76_acpi")
                .exists()
            || sysfs.join("sys/bus/platform/drivers/system76").exists()
    }

    fn find_fan_mode(sysfs: &Path) -> Option<PathBuf> {
        let p = sysfs.join("sys/devices/platform/system76_acpi/fan_mode");
        if p.exists() {
            Some(p)
        } else {
            None
        }
    }
}

impl HardwareProvider for System76Provider {
    fn name(&self) -> &'static str {
        "system76_acpi"
    }

    fn vendor(&self) -> HardwareVendor {
        HardwareVendor::System76
    }

    fn detect(&self, sysfs: &Path) -> Option<ProviderMatch> {
        let vendor_str = fs::read_to_string(sysfs.join("sys/devices/virtual/dmi/id/sys_vendor"))
            .unwrap_or_default()
            .to_lowercase();
        let is_sys76 = vendor_str.contains("system76");

        if (is_sys76 && Self::find_system76_driver(sysfs)) || Self::find_system76_driver(sysfs) {
            Some(ProviderMatch {
                priority: 85,
                description: "System76 Laptop (system76_acpi driver active)".to_string(),
            })
        } else if is_sys76 {
            Some(ProviderMatch {
                priority: 45,
                description: "System76 device detected (upstream generic fallback)".to_string(),
            })
        } else {
            None
        }
    }

    fn capabilities(&self, sysfs: &Path) -> HardwareCapabilities {
        let mut caps = self.generic.capabilities(sysfs);
        caps.vendor = HardwareVendor::System76;
        caps.verification_tier = VerificationTier::Tier2CommunitySupported;
        caps.tier = VerificationTier::Tier2CommunitySupported;
        caps.active_provider_name = self.name().to_string();

        if Self::find_fan_mode(sysfs).is_some() {
            caps.fan_control = FanControlCapability::DiscreteLevels {
                levels: vec![
                    "auto (System Thermal Control)".to_string(),
                    "full (Maximum Fan Boost)".to_string(),
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
        if let Some(fm) = Self::find_fan_mode(sysfs) {
            let val = if level >= 200 { "full\n" } else { "auto\n" };
            fs::write(&fm, val).map_err(|e| AwccError::Io {
                path: fm.display().to_string(),
                source: e,
            })?;
            return Ok(());
        }
        self.generic.set_fan_level(sysfs, fan_index, level)
    }

    fn set_max_fans(&self, sysfs: &Path) -> Result<(), AwccError> {
        if let Some(fm) = Self::find_fan_mode(sysfs) {
            return fs::write(&fm, "full\n").map_err(|e| AwccError::Io {
                path: fm.display().to_string(),
                source: e,
            });
        }
        self.generic.set_max_fans(sysfs)
    }

    fn release_fans(&self, sysfs: &Path) -> Result<(), AwccError> {
        if let Some(fm) = Self::find_fan_mode(sysfs) {
            return fs::write(&fm, "auto\n").map_err(|e| AwccError::Io {
                path: fm.display().to_string(),
                source: e,
            });
        }
        self.generic.release_fans(sysfs)
    }
}
