# AWCC Linux Fan Control (`awcc-linux`)
High-performance, secure fan control, platform profile orchestration, and real-time tachometer telemetry suite for Dell G-Series & Alienware laptops on Enterprise Linux.

Primary Target Platform: **Dell G15 5530** running **Red Hat Enterprise Linux 10.2** (Kernel 6.12+).

---

## Architecture Overview

```
                      /usr/local/bin/rhel-performance
                                    |
                                 awccctl
                       (CLI & Telemetry Engine)
                                    |
                                  awccd
                       (Policy & Safety Daemon)
                                    |
                    sysfs (/sys/class/hwmon/hwmonX)
                                    |
                 alienware-wmi.ko (Backported & Signed)
                                    |
                      WMAX WMI Node (PNP0C14:0b)
                                    |
                     Dell Embedded Controller (EC)
```

---

## Quick Start & Verification

### 1. Build and Sign the Driver
```bash
cd Awcc/kernel
make
./scripts/sign-module.sh
```

### 2. Load Driver
```bash
sudo insmod alienware-wmi.ko
# or copy to /lib/modules/$(uname -r)/extra/ and run depmod -a
```

### 3. Check System Diagnostics
```bash
cargo run --release --bin awccctl -- doctor
```

### 4. Telemetry Status
```bash
# Tabular mode
cargo run --release --bin awccctl -- status

# Machine-readable JSON mode
cargo run --release --bin awccctl -- status --json
```

### 5. Actuate Performance Mode & Fans
```bash
./bin/rhel-performance
```

---

## Safety Features
- **No Raw EC Writes**: All communication goes through the Dell ACPI WMI WMAX interface.
- **Fail-Safe Cleanup**: The `awccd` daemon traps `SIGINT`/`SIGTERM` and sleep signals, resetting `fan[1-2]_boost` to `0` to return control to BIOS EC fan curves.
- **Tachometer Readback**: Clearly differentiates requested boost (`0-255`), readback boost, and measured tachometer RPM.

## Unified Operational Modes (`battery`, `quiet`, `balance`, `high`, `ultra`)

You can now toggle between complete power & fan profiles instantly using single-word commands from anywhere in your terminal:

- **`battery`**: Restricts power envelope and keeps fans silent (boost = 0).
- **`quiet`**: Prioritizes acoustic silence (boost = 0, quiet platform profile).
- **`balance`**: Everyday balance of CPU/GPU clocks and moderate airflow (boost = 80).
- **`high`**: High performance with sustained strong airflow (boost = 180).
- **`ultra`**: Dell Game Shift (G-Mode) with maximum cooling & fan speed (boost = 255, ~5,000 RPM).

## Dedicated `awcc-*` Command Suite

You can run any of the dedicated `awcc-*` commands directly from your terminal:

- **`awcc-battery`**: Boost `26/255` (~10% step) + `quiet` power profile
- **`awcc-quiet`**: Boost `51/255` (20% step) + `quiet` profile
- **`awcc-balance`**: Boost `102/255` (40% step) + `balanced` profile
- **`awcc-mid`**: Boost `153/255` (60% step) + `performance` profile
- **`awcc-high`**: Boost `204/255` (80% step) + `performance` profile
- **`awcc-ultra`**: Boost `255/255` (100% max step) + Dell Game Shift (G-Mode) (~5,000 RPM)

## Real-Time Live Monitor Dashboard

Launch the dynamic full-screen terminal monitor by running:

```bash
awcc-monitor
# or simply:
monitor
```

It displays:
- Real-time Fan 1 (CPU) & Fan 2 (GPU) boost gauges & physical tachometer RPMs.
- Active platform profile and Dell Game-Shift (G-Mode) status.
- Live temperature telemetry for Intel CPU package, motherboard VRMs, and NVIDIA GPU die.

## Hardware F9 Game-Shift (G-Mode) Key Integration

On Dell G15 laptops, the **F9** key (marked with the 'G' Game-Shift logo) toggles G-Mode:

- **When G-Mode is OFF**: Pressing **`F9`** immediately activates **G-Mode Ultra Performance & Max Fans (~5,000 RPM)**.
- **When G-Mode is ON**: Pressing **`F9`** deactivates G-Mode and restores **Balanced** mode.

You can also trigger it from the terminal via:
```bash
awcc-gameshift-toggle
# or simply:
gmode
```
