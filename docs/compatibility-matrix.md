# Universal Linux Hardware Verification & Compatibility Matrix

This matrix classifies hardware into four strict verification tiers based on physical hardware evidence, kernel interface telemetry, and upstream driver integration.

---

## Verification Tiers Defined

| Tier | Classification | Definition | Control Capabilities |
| :--- | :--- | :--- | :--- |
| **Tier 1** | **Physically Verified** | Hardware physically tested and benchmarked by maintainers with logged tachometer acceleration, readback verification, and thermal sensor matrices. | Full 0-255 boost ladder, Game Shift (F9), platform profiles, discrete GPU/VRM/Ambient sensors. |
| **Tier 2** | **Community Supported** | Recognized vendor architectures with known kernel drivers (`asus-wmi`, `thinkpad_acpi`, `hp-wmi`, `cros_ec`, `system76_acpi`). | Platform profiles, discrete thermal policies, vendor-specific fan levels where exposed by kernel. |
| **Tier 3** | **Generic Linux Fallback** | Any desktop motherboard or portable PC exposing standard Linux sysfs interfaces (`/sys/class/platform-profile`, standard hwmon `fanX_input`, `pwmX`). | Standard Linux PWM fan control, ACPI platform profiles, CPU frequency governors. |
| **Tier 4** | **Unsupported** | Proprietary architectures lacking kernel drivers or locking fan controls inside undocumented EC registers. | Fails safe: manual fan overrides disabled, reports firmware management active without crashing. |

---

## Hardware Matrix

| Hardware Vendor & Model | DMI Product Name | DMI Sys Vendor | Active Kernel Driver | Verification Tier | Verified Capabilities / Evidence |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Dell G15 5530** | `Dell G15 5530` | `Dell Inc.` | `alienware-wmi` + `dell_smm` + `dell_ddv` | **Tier 1: Physically Verified** | Tested on RHEL 10.2 (BIOS 1.34.0, Kernel 6.12.0). Dual fan boost (0-255), 0 to 4,800 RPM tachometer ladder, Game Shift (F9) toggle, VRM/ambient sensors. |
| **Dell G15 5515** | `Dell G15 5515` | `Dell Inc.` | `alienware-wmi` | **Tier 2: Community Supported** | Upstream WMAX DMI quirk present in kernel. Supports platform profiles and fan boost. |
| **Dell G16 7630** | `Dell G16 7630` | `Dell Inc.` | `alienware-wmi` | **Tier 2: Community Supported** | Upstream WMAX DMI quirk present. Supports dual fan boost and platform profiles. |
| **Alienware m16 R1** | `Alienware m16 R1` | `Alienware` | `alienware-wmi` | **Tier 2: Community Supported** | Thermal WMI method v0 supported in upstream driver. |
| **Alienware x14 R1** | `Alienware x14 R1` | `Alienware` | `alienware-wmi` | **Tier 2: Community Supported** | Thermal WMI method v0 supported. Supports quad/dual fan readouts. |
| **ASUS ROG Zephyrus G14** | `ROG Zephyrus G14` | `ASUSTeK COMPUTER INC.` | `asus-nb-wmi` / `asus-wmi` | **Tier 2: Community Supported** | `throttle_thermal_policy` (0=Normal, 1=Overboost, 2=Silent), ACPI platform profiles. |
| **ASUS ROG Strix SCAR 16** | `ROG Strix G634` | `ASUSTeK COMPUTER INC.` | `asus-nb-wmi` / `asus-wmi` | **Tier 2: Community Supported** | Fan boost mode, thermal policies, ASUS EC sensors. |
| **Lenovo ThinkPad X1 Carbon (Gen 9-12)** | `ThinkPad X1 Carbon` | `LENOVO` | `thinkpad_acpi` | **Tier 2: Community Supported** | `/proc/acpi/ibm/fan` discrete levels (0-7, auto, disengaged), ACPI platform profiles. |
| **Lenovo Legion Pro 5 / 7** | `Legion Pro 5` / `7` | `LENOVO` | `ideapad_laptop` / `thinkpad_acpi` | **Tier 2: Community Supported** | ACPI platform profiles (quiet, balanced, performance), dual fan monitoring. |
| **HP Omen 16 / 17** | `OMEN by HP Laptop 16` | `HP` | `hp-wmi` / `hp-omen` | **Tier 2: Community Supported** | Platform profile switching, Omen thermal zone monitoring. |
| **Framework Laptop 13 / 16** | `Laptop 13` / `Laptop 16` | `Framework` | `cros_ec` / `acpi_thermal` | **Tier 2: Community Supported** | Platform profiles, battery charge thresholds, standard hwmon thermals. |
| **System76 Oryx Pro / Gazelle** | `Oryx Pro` / `Gazelle` | `System76` | `system76_acpi` / `system76_power` | **Tier 2: Community Supported** | Platform profiles, fan curve actuation via system76-power. |
| **MSI / ASUS / Gigabyte / ASRock Desktops** | Any Motherboard (e.g. `MS-7D91`) | Various OEMs | `nct6775`, `it87`, `coretemp`, `k10temp` | **Tier 3: Generic Linux Fallback** | Standard Linux PWM fan controls (`pwm1..N`), CPU/GPU temperature sensors, RAPL powercap. |
| **Apple Silicon (MacBook Pro/Air)** | `MacBookPro*` / `MacBookAir*` | `Apple Inc.` | Proprietary SMC | **Tier 4: Unsupported** | Safe degradation: reports firmware management active, manual overrides disabled. |

---

## Verification Criteria for Tier 1 Promotion

To elevate a Tier 2 or Tier 3 model to **Tier 1: Physically Verified**, submit telemetry adhering to [`docs/hardware-telemetry-submission.md`](hardware-telemetry-submission.md) satisfying all four verification proofs:
1. Exact DMI product name, vendor, and BIOS firmware revision logged.
2. Kernel release, loaded module stack (`dmesg`), and full sysfs tree export.
3. Verified tachometer acceleration: rotor speed ramps linearly with boost and reaches rated hardware limits.
4. Safe return to firmware control: boost cleanly reverts to firmware automatic curves upon daemon termination or reset.
