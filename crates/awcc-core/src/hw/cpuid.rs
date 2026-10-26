//! Low-level x86_64 CPUID hardware interrogation.
//!
//! Directly queries CPUID to extract vendor, model, stepping, brand string,
//! Digital Thermal Sensor (DTS) support, and Hardware P-States (HWP/Speed Shift).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CpuHardwareFeatures {
    pub vendor_id: String,
    pub brand_string: String,
    pub family: u32,
    pub model: u32,
    pub stepping: u32,
    pub digital_thermal_sensor: bool,
    pub turbo_boost: bool,
    pub hwp_supported: bool,
    pub hwp_epp_supported: bool,
    pub energy_bias_preference: bool,
    pub aperf_mperf_supported: bool,
    pub rdtscp_supported: bool,
}

#[cfg(target_arch = "x86_64")]
pub fn probe_cpu_features() -> CpuHardwareFeatures {
    use core::arch::x86_64::__cpuid;

    let leaf0 = __cpuid(0);
    let max_leaf = leaf0.eax;
    let mut vendor_bytes = [0u8; 12];
    vendor_bytes[0..4].copy_from_slice(&leaf0.ebx.to_le_bytes());
    vendor_bytes[4..8].copy_from_slice(&leaf0.edx.to_le_bytes());
    vendor_bytes[8..12].copy_from_slice(&leaf0.ecx.to_le_bytes());
    let vendor_id = String::from_utf8_lossy(&vendor_bytes).trim().to_string();

    let (family, model, stepping) = if max_leaf >= 1 {
        let leaf1 = __cpuid(1);
        let stepping = leaf1.eax & 0x0F;
        let base_model = (leaf1.eax >> 4) & 0x0F;
        let base_family = (leaf1.eax >> 8) & 0x0F;
        let ext_model = (leaf1.eax >> 16) & 0x0F;
        let ext_family = (leaf1.eax >> 20) & 0xFF;

        let family = if base_family == 15 {
            base_family + ext_family
        } else {
            base_family
        };

        let model = if base_family == 6 || base_family == 15 {
            (ext_model << 4) | base_model
        } else {
            base_model
        };

        (family, model, stepping)
    } else {
        (0, 0, 0)
    };

    let (dts, turbo, hwp, hwp_epp, energy_bias, aperf_mperf) = if max_leaf >= 6 {
        let leaf6 = __cpuid(6);
        let dts = (leaf6.eax & 0x01) != 0;
        let turbo = (leaf6.eax & 0x02) != 0;
        let hwp = (leaf6.eax & 0x40) != 0;
        let hwp_epp = (leaf6.eax & 0x200) != 0;
        let energy_bias = (leaf6.ecx & 0x08) != 0;
        let aperf_mperf = (leaf6.ecx & 0x01) != 0;
        (dts, turbo, hwp, hwp_epp, energy_bias, aperf_mperf)
    } else {
        (false, false, false, false, false, false)
    };

    let leaf_ext0 = __cpuid(0x8000_0000);
    let max_ext_leaf = leaf_ext0.eax;

    let rdtscp_supported = if max_ext_leaf >= 0x8000_0001 {
        let leaf_ext1 = __cpuid(0x8000_0001);
        (leaf_ext1.edx & (1 << 27)) != 0
    } else {
        false
    };

    let brand_string = if max_ext_leaf >= 0x8000_0004 {
        let mut brand_bytes = [0u8; 48];
        for i in 0..3 {
            let leaf = __cpuid(0x8000_0002 + i);
            let offset = (i as usize) * 16;
            brand_bytes[offset..offset + 4].copy_from_slice(&leaf.eax.to_le_bytes());
            brand_bytes[offset + 4..offset + 8].copy_from_slice(&leaf.ebx.to_le_bytes());
            brand_bytes[offset + 8..offset + 12].copy_from_slice(&leaf.ecx.to_le_bytes());
            brand_bytes[offset + 12..offset + 16].copy_from_slice(&leaf.edx.to_le_bytes());
        }
        let raw = String::from_utf8_lossy(&brand_bytes);
        raw.trim_matches('\0').trim().to_string()
    } else {
        String::new()
    };

    CpuHardwareFeatures {
        vendor_id,
        brand_string,
        family,
        model,
        stepping,
        digital_thermal_sensor: dts,
        turbo_boost: turbo,
        hwp_supported: hwp,
        hwp_epp_supported: hwp_epp,
        energy_bias_preference: energy_bias,
        aperf_mperf_supported: aperf_mperf,
        rdtscp_supported,
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub fn probe_cpu_features() -> CpuHardwareFeatures {
    CpuHardwareFeatures::default()
}
