# AWCC Linux Compatibility & Hardware Verification Matrix

This matrix clearly distinguishes between hardware physically tested and validated by the maintainers versus models with upstream quirk definitions that remain community-supported and untested locally.

| Model Identifier | DMI Product Name | DMI Sys Vendor | WMI GUID | Status | Verification Evidence / Notes |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Dell G15 5530** | `Dell G15 5530` | `Dell Inc.` | `A70591CE-A997-11DA-B012-B622A1EF5492` | **Physically Verified** | Verified on RHEL 10.2 (BIOS 1.34.0, Kernel 6.12.0-211.68.1.el10_2.x86_64). Dual-fan boost (0-255), dynamic hwmon tachometer range (0 to 4,800 RPM), platform-profile class device. |
| **Dell G15 5515** | `Dell G15 5515` | `Dell Inc.` | `A70591CE-A997-11DA-B012-B622A1EF5492` | Untested / Community Supported | Upstream DMI quirk present in `alienware-wmi-wmax.c`. No local physical verification report. |
| **Dell G16 7630** | `Dell G16 7630` | `Dell Inc.` | `A70591CE-A997-11DA-B012-B622A1EF5492` | Untested / Community Supported | Upstream DMI quirk present in `alienware-wmi-wmax.c`. No local physical verification report. |
| **Alienware m16 R1** | `Alienware m16 R1` | `Alienware` | `A70591CE-A997-11DA-B012-B622A1EF5492` | Untested / Community Supported | Upstream thermal WMI method support. No local physical verification report. |
| **Alienware x14 R1** | `Alienware x14 R1` | `Alienware` | `A70591CE-A997-11DA-B012-B622A1EF5492` | Untested / Community Supported | Upstream thermal WMI method support. No local physical verification report. |

## Verification Criteria for "Physically Verified" Status
A model is only marked **Physically Verified** when the following criteria are met and archived:
1. Exact DMI product name, vendor, and BIOS version recorded.
2. Kernel version, module loading logs (`dmesg`), and sysfs tree archived.
3. Verified tachometer acceleration: rotor speed ramps linearly with boost `0..255` and matches reported hardware limits (`fanN_max`).
4. Safe return to firmware control: boost returns to `0` and fans decelerate under thermal idle.
