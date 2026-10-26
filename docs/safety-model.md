# AWCC Linux Fan Control Security Architecture & Threat Model

## 1. Trust Boundaries & Privileges
- **Unprivileged Users**: Can query live telemetry, platform profile choices, and diagnostic reports through `awccctl detect`, `awccctl status`, and `awccctl doctor`.
- **Privileged Scope**: Writing to `/sys/class/hwmon/hwmonX/fan*_boost` and `/sys/class/platform-profile/platform-profile-*/profile` requires root or systemd unit privilege (`awccd.service`).
- **No World-Writable Sysfs Controls**: All hardware control nodes are kept at standard root permissions (`0644`). Setting `chmod 0666` or creating world-writable udev rules is strictly prohibited to prevent unauthorized local processes from altering laptop thermal profiles or disabling fans.
- **Controlled Desktop Delegation**: Desktop shortcut actuation (e.g. F9 Game Shift) may be enabled via a bounded sudoers drop-in (`/etc/sudoers.d/awcc-linux`) allowing members of the local `wheel` group to execute `/usr/local/bin/awccctl fan *` and `/usr/local/bin/awccctl profile *` without password prompts.

## 2. Mitigation of Firmware / Hardware Damage
- **No Direct Port I/O or Raw EC Writes**: Direct I/O writes (`inb`/`outb` or probing memory registers `0x66`/`0x68`) are completely avoided. All commands pass through ACPI WMI WMAX methods checked and executed by OEM firmware.
- **Fail-Safe Restoration**: On any abnormal exit (`SIGTERM`, `SIGINT`, system suspend), `awccd` resets boost values to `0`, handing thermal management back to the Dell BIOS EC fan curve.
- **Bounded Inputs**: Boost values are strictly validated and clamped to `0..255`. Profiles are validated against reported driver choices before actuation.

## 3. Sensor Failure & Telemetry Safety Policy
- **Never Treat Missing Sensors as 0°C**: If CPU core/package or GPU temperature cannot be read (e.g. driver glitch, absent sensor), the governor daemon does NOT assume the system is at 0°C (which would suppress cooling). Instead, it logs an alert and leaves fan control authority to the Dell BIOS firmware.
- **Anti-Hunt Hysteresis**: Prevents rapid thermal oscillation by damping ramp-downs while allowing instantaneous ramp-ups during sudden temperature spikes.
