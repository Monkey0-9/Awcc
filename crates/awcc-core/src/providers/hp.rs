use crate::capabilities::{HardwareCapabilities, HardwareVendor, VerificationTier};
use crate::error::AwccError;
use crate::providers::{generic::GenericLinuxProvider, HardwareProvider, ProviderMatch};
use crate::telemetry::SystemTelemetry;
use std::fs;
use std::path::Path;

pub struct HpProvider {
    generic: GenericLinuxProvider,
}

impl Default for HpProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl HpProvider {
    pub fn new() -> Self {
        Self {
            generic: GenericLinuxProvider::new(),
        }
    }

    fn find_hp_driver(sysfs: &Path) -> bool {
        sysfs.join("sys/devices/platform/hp-wmi").exists()
            || sysfs.join("sys/bus/platform/drivers/hp-wmi").exists()
    }
}

impl HardwareProvider for HpProvider {
    fn name(&self) -> &'static str {
        "hp_wmi"
    }

    fn vendor(&self) -> HardwareVendor {
        HardwareVendor::Hp
    }

    fn detect(&self, sysfs: &Path) -> Option<ProviderMatch> {
        let vendor_str = fs::read_to_string(sysfs.join("sys/devices/virtual/dmi/id/sys_vendor"))
            .unwrap_or_default()
            .to_lowercase();
        let is_hp = vendor_str.contains("hp") || vendor_str.contains("hewlett-packard");

        if is_hp && Self::find_hp_driver(sysfs) {
            Some(ProviderMatch {
                priority: 80,
                description: "HP OMEN / Victus (hp-wmi driver active)".to_string(),
            })
        } else if is_hp {
            Some(ProviderMatch {
                priority: 40,
                description: "HP device detected (upstream generic fallback)".to_string(),
            })
        } else {
            None
        }
    }

    fn capabilities(&self, sysfs: &Path) -> HardwareCapabilities {
        let mut caps = self.generic.capabilities(sysfs);
        caps.vendor = HardwareVendor::Hp;
        caps.verification_tier = VerificationTier::Tier2CommunitySupported;
        caps.tier = VerificationTier::Tier2CommunitySupported;
        caps.active_provider_name = self.name().to_string();
        caps
    }

    fn read_telemetry(&self, sysfs: &Path) -> SystemTelemetry {
        self.generic.read_telemetry(sysfs)
    }

    fn set_platform_profile(&self, sysfs: &Path, profile: &str) -> Result<(), AwccError> {
        self.generic.set_platform_profile(sysfs, profile)
    }

    fn set_fan_level(&self, sysfs: &Path, fan_index: u8, level: u8) -> Result<(), AwccError> {
        self.generic.set_fan_level(sysfs, fan_index, level)
    }

    fn set_max_fans(&self, sysfs: &Path) -> Result<(), AwccError> {
        self.generic.set_max_fans(sysfs)
    }

    fn release_fans(&self, sysfs: &Path) -> Result<(), AwccError> {
        self.generic.release_fans(sysfs)
    }
}
