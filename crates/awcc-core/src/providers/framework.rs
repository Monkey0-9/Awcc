use crate::capabilities::{HardwareCapabilities, HardwareVendor, VerificationTier};
use crate::error::AwccError;
use crate::providers::{generic::GenericLinuxProvider, HardwareProvider, ProviderMatch};
use crate::telemetry::SystemTelemetry;
use std::fs;
use std::path::Path;

pub struct FrameworkProvider {
    generic: GenericLinuxProvider,
}

impl Default for FrameworkProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameworkProvider {
    pub fn new() -> Self {
        Self {
            generic: GenericLinuxProvider::new(),
        }
    }

    fn find_cros_ec(sysfs: &Path) -> bool {
        sysfs.join("sys/devices/platform/cros-ec-dev").exists()
            || sysfs.join("sys/class/chromeos/cros_ec").exists()
    }
}

impl HardwareProvider for FrameworkProvider {
    fn name(&self) -> &'static str {
        "framework_cros_ec"
    }

    fn vendor(&self) -> HardwareVendor {
        HardwareVendor::Framework
    }

    fn detect(&self, sysfs: &Path) -> Option<ProviderMatch> {
        let vendor_str = fs::read_to_string(sysfs.join("sys/devices/virtual/dmi/id/sys_vendor"))
            .unwrap_or_default()
            .to_lowercase();

        if vendor_str.contains("framework") {
            Some(ProviderMatch {
                priority: 85,
                description: "Framework Laptop (cros_ec / platform-profile)".to_string(),
            })
        } else if Self::find_cros_ec(sysfs) {
            Some(ProviderMatch {
                priority: 75,
                description: "ChromeOS EC platform active".to_string(),
            })
        } else {
            None
        }
    }

    fn capabilities(&self, sysfs: &Path) -> HardwareCapabilities {
        let mut caps = self.generic.capabilities(sysfs);
        caps.vendor = HardwareVendor::Framework;
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
