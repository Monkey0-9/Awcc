pub mod asus;
pub mod dell;
pub mod framework;
pub mod generic;
pub mod hp;
pub mod lenovo;

use crate::capabilities::{HardwareCapabilities, HardwareVendor};
use crate::error::AwccError;
use crate::telemetry::SystemTelemetry;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderMatch {
    pub priority: u8,
    pub description: String,
}

pub trait HardwareProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn vendor(&self) -> HardwareVendor;
    fn detect(&self, sysfs: &Path) -> Option<ProviderMatch>;
    fn capabilities(&self, sysfs: &Path) -> HardwareCapabilities;
    fn read_telemetry(&self, sysfs: &Path) -> SystemTelemetry;
    fn set_platform_profile(&self, sysfs: &Path, profile: &str) -> Result<(), AwccError>;
    fn set_fan_level(&self, sysfs: &Path, fan_index: u8, level: u8) -> Result<(), AwccError>;
    fn set_max_fans(&self, sysfs: &Path) -> Result<(), AwccError>;
    fn release_fans(&self, sysfs: &Path) -> Result<(), AwccError>;
}

pub struct ProviderRegistry {
    providers: Vec<Box<dyn HardwareProvider>>,
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderRegistry {
    pub fn new() -> Self {
        let mut reg = Self {
            providers: Vec::new(),
        };
        // Register vendor specific providers first
        reg.register(Box::new(dell::DellAwccProvider::new()));
        reg.register(Box::new(asus::AsusProvider::new()));
        reg.register(Box::new(lenovo::LenovoProvider::new()));
        reg.register(Box::new(hp::HpProvider::new()));
        reg.register(Box::new(framework::FrameworkSystem76Provider::new()));
        // Register universal generic fallback last
        reg.register(Box::new(generic::GenericLinuxProvider::new()));
        reg
    }

    pub fn register(&mut self, provider: Box<dyn HardwareProvider>) {
        self.providers.push(provider);
    }

    pub fn find_best_provider<'a>(
        &'a self,
        sysfs: &Path,
    ) -> Option<(&'a dyn HardwareProvider, ProviderMatch)> {
        let mut best: Option<(&'a dyn HardwareProvider, ProviderMatch)> = None;
        for p in &self.providers {
            if let Some(m) = p.detect(sysfs) {
                if let Some((_, ref cur_best)) = best {
                    if m.priority > cur_best.priority {
                        best = Some((p.as_ref(), m));
                    }
                } else {
                    best = Some((p.as_ref(), m));
                }
            }
        }
        best
    }

    pub fn providers(&self) -> &[Box<dyn HardwareProvider>] {
        &self.providers
    }
}
