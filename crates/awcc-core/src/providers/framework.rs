use crate::capabilities::{HardwareCapabilities, HardwareVendor, VerificationTier};
use crate::error::AwccError;
use crate::providers::{generic::GenericLinuxProvider, HardwareProvider, ProviderMatch};
use crate::telemetry::SystemTelemetry;
use std::fs;
use std::path::Path;

pub struct FrameworkSystem76Provider {
    generic: GenericLinuxProvider,
}

impl Default for FrameworkSystem76Provider {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameworkSystem76Provider {
    pub fn new() -> Self {
        Self {
            generic: GenericLinuxProvider::new(),
        }
    }

    fn find_cros_or_system76(sysfs: &Path) -> bool {
        sysfs.join("sys/devices/platform/cros-ec-dev").exists()
            || sysfs.join("sys/devices/platform/system76_acpi").exists()
            || sysfs.join("sys/bus/platform/drivers/system76").exists()
    }
}

impl HardwareProvider for FrameworkSystem76Provider {
    fn name(&self) -> &'static str {
        "framework_system76"
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
        } else if vendor_str.contains("system76") {
            Some(ProviderMatch {
                priority: 85,
                description: "System76 Laptop (system76_acpi / power-daemon)".to_string(),
            })
        } else if Self::find_cros_or_system76(sysfs) {
            Some(ProviderMatch {
                priority: 75,
                description: "ChromeOS EC / System76 platform active".to_string(),
            })
        } else {
            None
        }
    }

    fn capabilities(&self, sysfs: &Path) -> HardwareCapabilities {
        let mut caps = self.generic.capabilities(sysfs);
        let vendor_str = fs::read_to_string(sysfs.join("sys/devices/virtual/dmi/id/sys_vendor"))
            .unwrap_or_default()
            .to_lowercase();
        if vendor_str.contains("system76") {
            caps.vendor = HardwareVendor::System76;
        } else {
            caps.vendor = HardwareVendor::Framework;
        }
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
