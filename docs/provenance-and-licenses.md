# Software Provenance and Licensing Inventory

This document defines the licensing, upstream provenance, and copyright attribution for every component within the AWCC Linux repository.

| Subsystem / Path | Primary Files | License | Upstream Origin / Attribution |
| :--- | :--- | :--- | :--- |
| **Kernel Subsystem** | `kernel/src/*.c`, `kernel/src/*.h` | `GPL-2.0-or-later` | Upstream Linux Kernel `alienware-wmi` driver authored by Kurt Borja <kuurtb@gmail.com> and Mario Limonciello <mario.limonciello@outlook.com>. |
| **Kernel Build / Packaging** | `kernel/Makefile`, `kernel/dkms.conf`, `kernel/scripts/*` | `GPL-2.0-or-later` | Upstream Linux DKMS integration scripts. |
| **Core Rust Library** | `crates/awcc-core/` | `MIT` | Original userspace telemetry and ACPI abstraction library for AWCC Linux. |
| **CLI Management Utility** | `crates/awccctl/` | `MIT` | Original Rust CLI tool implementing diagnostic, status, profile, and fan actuation. |
| **Autonomous Daemon** | `crates/awccd/` | `MIT` | Original Rust thermal governor daemon implementing hysteresis curves and fail-safe restoration. |
| **Shell Integration Scripts** | `bin/*.sh`, `bin/rhel-*`, `bin/awcc-*` | `MIT` | Ergonomic shell wrappers for desktop shortcuts and system initialization. |
| **Cybernetic TUI Dashboard** | `bin/awcc-lux-dashboard.py` | `MIT` | Terminal UI dashboard for real-time telemetry observation and mode selection. |
| **Systemd Service** | `systemd/awccd.service` | `MIT` | Hardened systemd unit configuration. |
| **Distribution Packaging** | `packaging/rhel/awcc-linux.spec`, `packaging/arch/PKGBUILD` | `GPL-2.0-or-later AND MIT` | RPM and Arch packaging specifications. |

## Third-Party Notice
The `alienware-wmi` kernel module interacts with Dell proprietary ACPI WMI objects (`WMAX` GUID `A70591CE-A997-11DA-B012-B622A1EF5492`). It does not contain any proprietary binary blobs or reverse-engineered firmware microcode.
