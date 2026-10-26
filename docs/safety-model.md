# AWCC Linux Fan Control Security Architecture & Threat Model

## 1. Trust Boundaries & Privileges
- **Unprivileged Users**: Can query live telemetry, platform profile choices, and doctor diagnostics through `awccctl detect`, `awccctl status`, and `awccctl doctor`.
- **Privileged Scope**: Writing to `/sys/devices/platform/.../fan[1-2]_boost` and `/sys/firmware/acpi/platform_profile` requires root or systemd unit privilege (`awccd.service`).

## 2. Mitigation of Firmware / Hardware Damage
- **No Direct Port I/O or Raw EC Writes**: Direct I/O writes (`inb`/`outb` or probing memory registers `0x66`/`0x68`) are completely avoided. All commands pass through ACPI WMI WMAX methods checked and executed by OEM firmware.
- **Fail-Safe Restoration**: On any abnormal exit (`SIGTERM`, `SIGINT`, system suspend), `awccd` resets boost values to `0`, handing thermal management back to the Dell BIOS EC fan curve.
- **Clamping**: Boost values are strictly clamped to `0..255`.
