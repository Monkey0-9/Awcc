use crate::capabilities::{
    FanControlCapability, HardwareCapabilities, HardwareVendor, VerificationTier,
};
use crate::error::AwccError;
use crate::providers::{generic::GenericLinuxProvider, HardwareProvider, ProviderMatch};
use crate::telemetry::SystemTelemetry;
use std::fs;
use std::path::Path;

pub struct AsusProvider {
    generic: GenericLinuxProvider,
}

impl Default for AsusProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AsusProvider {
    pub fn new() -> Self {
        Self {
            generic: GenericLinuxProvider::new(),
        }
    }

    fn find_asus_wmi(sysfs: &Path) -> bool {
        sysfs.join("sys/devices/platform/asus-nb-wmi").exists()
            || sysfs.join("sys/devices/platform/asus_wmi").exists()
            || sysfs.join("sys/bus/platform/drivers/asus-nb-wmi").exists()
    }
}

impl HardwareProvider for AsusProvider {
    fn name(&self) -> &'static str {
        "asus_wmi"
    }

    fn vendor(&self) -> HardwareVendor {
        HardwareVendor::Asus
    }

    fn detect(&self, sysfs: &Path) -> Option<ProviderMatch> {
        let vendor_str = fs::read_to_string(sysfs.join("sys/devices/virtual/dmi/id/sys_vendor"))
            .unwrap_or_default()
            .to_lowercase();
        let is_asus = vendor_str.contains("asus") || vendor_str.contains("asustek");

        if is_asus && Self::find_asus_wmi(sysfs) {
            Some(ProviderMatch {
                priority: 85,
                description: "ASUSTeK ROG / TUF (asus-nb-wmi driver active)".to_string(),
            })
        } else if is_asus {
            Some(ProviderMatch {
                priority: 45,
                description: "ASUSTeK device detected (upstream generic fallback)".to_string(),
            })
        } else {
            None
        }
    }

    fn capabilities(&self, sysfs: &Path) -> HardwareCapabilities {
        let mut caps = self.generic.capabilities(sysfs);
        caps.vendor = HardwareVendor::Asus;
        caps.verification_tier = VerificationTier::Tier2CommunitySupported;
        caps.tier = VerificationTier::Tier2CommunitySupported;
        caps.active_provider_name = self.name().to_string();

        let throttle_policy =
            sysfs.join("sys/devices/platform/asus-nb-wmi/throttle_thermal_policy");
        if throttle_policy.exists() {
            caps.fan_control = FanControlCapability::DiscreteLevels {
                levels: vec![
                    "0 (Default)".to_string(),
                    "1 (Overboost)".to_string(),
                    "2 (Silent)".to_string(),
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

    fn set_fan_level(&self, sysfs: &Path, _fan_index: u8, level: u8) -> Result<(), AwccError> {
        let throttle_policy =
            sysfs.join("sys/devices/platform/asus-nb-wmi/throttle_thermal_policy");
        if throttle_policy.exists() {
            let policy_val = if level >= 200 {
                "1"
            } else if level <= 50 {
                "2"
            } else {
                "0"
            };
            fs::write(&throttle_policy, policy_val).map_err(|e| AwccError::Io {
                path: throttle_policy.display().to_string(),
                source: e,
            })?;
            return Ok(());
        }
        self.generic.set_fan_level(sysfs, _fan_index, level)
    }

    fn set_max_fans(&self, sysfs: &Path) -> Result<(), AwccError> {
        let throttle_policy =
            sysfs.join("sys/devices/platform/asus-nb-wmi/throttle_thermal_policy");
        if throttle_policy.exists() {
            return fs::write(&throttle_policy, "1\n").map_err(|e| AwccError::Io {
                path: throttle_policy.display().to_string(),
                source: e,
            });
        }
        self.generic.set_max_fans(sysfs)
    }

    fn release_fans(&self, sysfs: &Path) -> Result<(), AwccError> {
        let throttle_policy =
            sysfs.join("sys/devices/platform/asus-nb-wmi/throttle_thermal_policy");
        if throttle_policy.exists() {
            return fs::write(&throttle_policy, "0\n").map_err(|e| AwccError::Io {
                path: throttle_policy.display().to_string(),
                source: e,
            });
        }
        self.generic.release_fans(sysfs)
    }
}
