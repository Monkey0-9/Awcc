use crate::capabilities::{
    FanControlCapability, HardwareCapabilities, HardwareVendor, VerificationTier,
};
use crate::error::AwccError;
use crate::providers::{generic::GenericLinuxProvider, HardwareProvider, ProviderMatch};
use crate::telemetry::SystemTelemetry;
use std::fs;
use std::path::{Path, PathBuf};

pub struct MsiProvider {
    generic: GenericLinuxProvider,
}

impl Default for MsiProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MsiProvider {
    pub fn new() -> Self {
        Self {
            generic: GenericLinuxProvider::new(),
        }
    }

    fn find_msi_driver(sysfs: &Path) -> bool {
        sysfs.join("sys/devices/platform/msi-ec").exists()
            || sysfs.join("sys/devices/platform/msi-wmi").exists()
            || sysfs.join("sys/devices/platform/msi-laptop").exists()
            || sysfs.join("sys/bus/platform/drivers/msi-ec").exists()
            || sysfs.join("sys/bus/platform/drivers/msi-wmi").exists()
    }

    fn find_cooler_boost(sysfs: &Path) -> Option<PathBuf> {
        let ec_path = sysfs.join("sys/devices/platform/msi-ec/cooler_boost");
        if ec_path.exists() {
            return Some(ec_path);
        }
        let wmi_path = sysfs.join("sys/devices/platform/msi-wmi/cooler_boost");
        if wmi_path.exists() {
            return Some(wmi_path);
        }
        let lap_path = sysfs.join("sys/devices/platform/msi-laptop/cooler_boost");
        if lap_path.exists() {
            return Some(lap_path);
        }
        None
    }
}

impl HardwareProvider for MsiProvider {
    fn name(&self) -> &'static str {
        "msi_ec"
    }

    fn vendor(&self) -> HardwareVendor {
        HardwareVendor::Msi
    }

    fn detect(&self, sysfs: &Path) -> Option<ProviderMatch> {
        let vendor_str = fs::read_to_string(sysfs.join("sys/devices/virtual/dmi/id/sys_vendor"))
            .unwrap_or_default()
            .to_lowercase();
        let is_msi = vendor_str.contains("micro-star")
            || vendor_str.contains("msi")
            || vendor_str.contains("star international");

        if is_msi && Self::find_msi_driver(sysfs) {
            Some(ProviderMatch {
                priority: 85,
                description: "MSI Gaming / Creator (msi-ec / msi-wmi active)".to_string(),
            })
        } else if is_msi {
            Some(ProviderMatch {
                priority: 45,
                description: "MSI device detected (upstream generic fallback)".to_string(),
            })
        } else {
            None
        }
    }

    fn capabilities(&self, sysfs: &Path) -> HardwareCapabilities {
        let mut caps = self.generic.capabilities(sysfs);
        caps.vendor = HardwareVendor::Msi;
        caps.verification_tier = VerificationTier::Tier2CommunitySupported;
        caps.tier = VerificationTier::Tier2CommunitySupported;
        caps.active_provider_name = self.name().to_string();

        if Self::find_cooler_boost(sysfs).is_some() {
            caps.fan_control = FanControlCapability::DiscreteLevels {
                levels: vec![
                    "0 (Auto / Normal)".to_string(),
                    "1 (Cooler Boost Active)".to_string(),
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
        if let Some(cb_path) = Self::find_cooler_boost(sysfs) {
            let val = if level >= 200 { "1\n" } else { "0\n" };
            fs::write(&cb_path, val).map_err(|e| AwccError::Io {
                path: cb_path.display().to_string(),
                source: e,
            })?;
            return Ok(());
        }
        self.generic.set_fan_level(sysfs, fan_index, level)
    }

    fn set_max_fans(&self, sysfs: &Path) -> Result<(), AwccError> {
        if let Some(cb_path) = Self::find_cooler_boost(sysfs) {
            return fs::write(&cb_path, "1\n").map_err(|e| AwccError::Io {
                path: cb_path.display().to_string(),
                source: e,
            });
        }
        self.generic.set_max_fans(sysfs)
    }

    fn release_fans(&self, sysfs: &Path) -> Result<(), AwccError> {
        if let Some(cb_path) = Self::find_cooler_boost(sysfs) {
            return fs::write(&cb_path, "0\n").map_err(|e| AwccError::Io {
                path: cb_path.display().to_string(),
                source: e,
            });
        }
        self.generic.release_fans(sysfs)
    }
}
