# Hardware Telemetry Submission Guide

This guide explains how to capture and submit physical hardware verification evidence to elevate a machine to **Tier 1: Physically Verified** in the [Compatibility Matrix](compatibility-matrix.md).

---

## What Qualifies for Tier 1 Status?

To be certified as **Tier 1: Physically Verified**, hardware must meet four strict criteria:
1. **DMI Identity:** Exact vendor, product name, and BIOS firmware version recorded.
2. **Kernel Telemetry:** Kernel release, active kernel driver stack, and sysfs nodes documented.
3. **Tachometer Acceleration Proof:** Measured fan RPM increases in response to boost / performance mode and matches hardware limits.
4. **Firmware Release Proof:** Fans reliably decelerate back to quiet BIOS firmware curves upon release.

---

## Step-by-Step Evidence Capture

Run the following commands in terminal:

### 1. Capture Machine Identity & Capabilities
```bash
# Generate capability contract
awccctl capabilities --json > /tmp/capabilities.json

# Capture live telemetry snapshot
awccctl status --json > /tmp/status.json

# Run diagnostic health suite
awccctl doctor > /tmp/doctor.txt
```

### 2. Capture DMI & Kernel Details
```bash
cat << 'EOF' > /tmp/dmi_info.txt
--- DMI Identity ---
Vendor:       $(cat /sys/devices/virtual/dmi/id/sys_vendor 2>/dev/null)
Product:      $(cat /sys/devices/virtual/dmi/id/product_name 2>/dev/null)
BIOS Version: $(cat /sys/devices/virtual/dmi/id/bios_version 2>/dev/null)
Chassis Type: $(cat /sys/devices/virtual/dmi/id/chassis_type 2>/dev/null)
Kernel:       $(uname -r)
Distro:       $(grep PRETTY_NAME /etc/os-release | cut -d= -f2)
EOF
```

### 3. Verify Tachometer Response
Run the controlled calibration verification ladder:
```bash
awccctl verify --step-duration 5 > /tmp/verification_ladder.txt
```
*(If manual fan boost is unsupported on your hardware, note whether switching profiles via `awccctl profile performance` alters fan speeds).*

---

## Submitting Your Report

Create a new GitHub Issue titled:
`[Hardware Verification] <Vendor> <Product Name>`

Attach or paste the contents of:
- `/tmp/capabilities.json`
- `/tmp/status.json`
- `/tmp/doctor.txt`
- `/tmp/verification_ladder.txt`

Once validated by the maintainers, your machine will be added to the official Tier 1 verified hardware roster!
