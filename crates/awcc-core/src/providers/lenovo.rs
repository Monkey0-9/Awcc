use crate::capabilities::{
    FanControlCapability, HardwareCapabilities, HardwareVendor, VerificationTier,
};
use crate::error::AwccError;
use crate::providers::{generic::GenericLinuxProvider, HardwareProvider, ProviderMatch};
use crate::telemetry::SystemTelemetry;
use std::fs;
use std::path::Path;

pub struct LenovoProvider {
    generic: GenericLinuxProvider,
}

impl Default for LenovoProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl LenovoProvider {
    pub fn new() -> Self {
        Self {
            generic: GenericLinuxProvider::new(),
        }
    }

    fn find_lenovo_driver(sysfs: &Path) -> bool {
        sysfs.join("proc/acpi/ibm/fan").exists()
            || sysfs.join("sys/devices/platform/thinkpad_acpi").exists()
            || sysfs.join("sys/devices/platform/ideapad_laptop").exists()
            || sysfs
                .join("sys/bus/platform/drivers/thinkpad_acpi")
                .exists()
    }
}

impl HardwareProvider for LenovoProvider {
    fn name(&self) -> &'static str {
        "lenovo_thinkpad"
    }

    fn vendor(&self) -> HardwareVendor {
        HardwareVendor::Lenovo
    }

    fn detect(&self, sysfs: &Path) -> Option<ProviderMatch> {
        let vendor_str = fs::read_to_string(sysfs.join("sys/devices/virtual/dmi/id/sys_vendor"))
            .unwrap_or_default()
            .to_lowercase();
        let is_lenovo = vendor_str.contains("lenovo") || vendor_str.contains("ibm");

        if is_lenovo && Self::find_lenovo_driver(sysfs) {
            Some(ProviderMatch {
                priority: 85,
                description: "Lenovo ThinkPad / Legion (thinkpad_acpi driver active)".to_string(),
            })
        } else if is_lenovo {
            Some(ProviderMatch {
                priority: 45,
                description: "Lenovo device detected (upstream generic fallback)".to_string(),
            })
        } else {
            None
        }
    }

    fn capabilities(&self, sysfs: &Path) -> HardwareCapabilities {
        let mut caps = self.generic.capabilities(sysfs);
        caps.vendor = HardwareVendor::Lenovo;
        caps.verification_tier = VerificationTier::Tier2CommunitySupported;
        caps.tier = VerificationTier::Tier2CommunitySupported;
        caps.active_provider_name = self.name().to_string();

        let ibm_fan = sysfs.join("proc/acpi/ibm/fan");
        if ibm_fan.exists() {
            caps.fan_control = FanControlCapability::DiscreteLevels {
                levels: vec![
                    "level 0 (off)".to_string(),
                    "level 1-7".to_string(),
                    "auto".to_string(),
                    "disengaged (full speed)".to_string(),
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
        let ibm_fan = sysfs.join("proc/acpi/ibm/fan");
        if ibm_fan.exists() {
            let cmd = if level >= 250 {
                "level disengaged\n"
            } else if level == 0 {
                "level auto\n"
            } else {
                let lvl = (level as f32 / 255.0 * 7.0).round() as u8;
                return fs::write(&ibm_fan, format!("level {}\n", lvl)).map_err(|e| {
                    AwccError::Io {
                        path: ibm_fan.display().to_string(),
                        source: e,
                    }
                });
            };
            return fs::write(&ibm_fan, cmd).map_err(|e| AwccError::Io {
                path: ibm_fan.display().to_string(),
                source: e,
            });
        }
        self.generic.set_fan_level(sysfs, fan_index, level)
    }

    fn set_max_fans(&self, sysfs: &Path) -> Result<(), AwccError> {
        let ibm_fan = sysfs.join("proc/acpi/ibm/fan");
        if ibm_fan.exists() {
            return fs::write(&ibm_fan, "level disengaged\n").map_err(|e| AwccError::Io {
                path: ibm_fan.display().to_string(),
                source: e,
            });
        }
        self.generic.set_max_fans(sysfs)
    }

    fn release_fans(&self, sysfs: &Path) -> Result<(), AwccError> {
        let ibm_fan = sysfs.join("proc/acpi/ibm/fan");
        if ibm_fan.exists() {
            return fs::write(&ibm_fan, "level auto\n").map_err(|e| AwccError::Io {
                path: ibm_fan.display().to_string(),
                source: e,
            });
        }
        self.generic.release_fans(sysfs)
    }
}
