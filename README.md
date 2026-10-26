# AWCC Linux (`awcc-linux`)

Enterprise fan control, platform profile orchestration, and real-time tachometer telemetry suite for Dell G-Series & Alienware laptops on Linux.

**Primary Target Platform**: Dell G15 5530 (i7-13650HX, RTX 3050 6GB) on Red Hat Enterprise Linux 10.2 / Fedora / CentOS Stream (Kernel 6.12+).

---

## Architecture Overview

```
                      /usr/local/bin/rhel-performance (Boost 255/255)
                                    |
                                 awccctl
                        (CLI & Telemetry Engine)
                                    |
                                  awccd
                        (Policy & Safety Daemon)
                                    |
                sysfs (/sys/class/hwmon/hwmonX [0644])
                                    |
                 alienware-wmi.ko (Signed Kernel Module)
                                    |
                      WMAX WMI Node (PNP0C14:0b)
                                    |
                     Dell Embedded Controller (EC)
```

---

## Security Model & Privilege Boundary
- **Secure Root Controls (0644)**: System fan boost nodes (`fan[1-2]_boost`) and platform profile switches are restricted to root. World-writable permissions (`chmod 0666`) are strictly forbidden.
- **Unprivileged Telemetry**: Unprivileged users can freely inspect real-time tachometer RPM, temperatures, and current profiles via `awccctl status`, `awccctl detect`, or the cybernetic terminal dashboard.
- **Privileged Actuation**: Changing profiles or commanding fan boosts requires `sudo` or the systemd background daemon (`awccd.service`). For convenience with desktop hotkeys (such as F9), an optional `/etc/sudoers.d/awcc-linux` rule permits wheel users to run `awccctl` fan and profile commands without a password prompt.

---

## Quick Start & Verification

### 1. Build and Sign the Driver (if not yet installed)
```bash
cd kernel
make
./scripts/sign-module.sh
```

### 2. Load Driver
```bash
sudo insmod kernel/alienware-wmi.ko
# Or install system-wide via DKMS / installer:
sudo ./bin/install-system.sh
```

### 3. Check System Diagnostics
```bash
awccctl doctor
```

### 4. Telemetry Status
```bash
# Formatted tabular display
awccctl status

# Machine-readable JSON
awccctl status --json
```

### 5. Actuate True Maximum Performance & Fans
```bash
sudo ./bin/rhel-performance
```
*Applies 100% boost (`255/255`) on all controllable fans and verifies tachometer acceleration up to rated hardware ceiling (~4,800 RPM).*

---

## Operational Modes

| Mode | Shortcut | Platform Profile | Fan Boost | RPM Ceiling | Description |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Battery Saver** | `awcc-battery` | `quiet` | `10/255` (~4%) | ~1,000 RPM | Silent whisper, restricts thermal headroom for maximum battery runtime |
| **Quiet** | `awcc-quiet` | `quiet` | `20/255` (~8%) | ~1,450 RPM | Low-noise acoustic silence for study and office environments |
| **Balance** | `awcc-balance` | `balanced` | `40/255` (~16%) | ~1,980 RPM | Daily productivity balance with moderate cooling |
| **Mid** | `awcc-mid` | `performance` | `65/255` (~25%) | ~3,100 RPM | Sustained compilation, light gaming, content creation |
| **High** | `awcc-high` | `performance` | `85/255` (~33%) | ~4,200 RPM | Heavy gaming and sustained computational workloads |
| **Ultra (G-Mode)**| `awcc-ultra` / `rhel-performance` | `performance` | `255/255` (100%) | ~4,800 RPM | True maximum fan speed and peak performance headroom |

---

## Cybernetic Terminal Dashboard (TUI Cockpit)

Launch the real-time ANSI terminal cockpit for live tachometer graphs, temperatures, clock speeds, and keyboard shortcuts:

```bash
python3 bin/awcc-lux-dashboard.py
# Or snapshot one-liner:
python3 bin/awcc-lux-dashboard.py --snapshot
```

*Note: The dashboard is a terminal user interface (TUI). A native GTK4/Libadwaita desktop GUI is planned for Phase 4 of the roadmap.*

---

## Hardware Game Shift Hotkey (F9)

To bind the Dell F9 Game Shift key in GNOME desktop environments:
```bash
./bin/setup-f9-keybind.sh
```
To cleanly remove the binding without affecting your other desktop shortcuts:
```bash
./bin/uninstall-f9-keybind.sh
```

---

## License & Third-Party Provenance
- **Kernel Module (`kernel/`)**: GNU General Public License v2.0 or later ([GPL-2.0-or-later](LICENSES/GPL-2.0.txt)), derived from upstream `alienware-wmi` patches by Kurt Borja and Mario Limonciello.
- **Userspace Rust Crates and Scripts (`crates/`, `bin/`)**: [MIT License](LICENSES/MIT.txt).
- Full file-by-file inventory is documented in [docs/provenance-and-licenses.md](docs/provenance-and-licenses.md).
