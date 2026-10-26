# AWCC Linux Architecture Specification

## Layered Model
1. **Presentation & CLI Layer**: `awccctl` CLI and `/usr/local/bin/rhel-*` helper scripts.
2. **Policy & Thermal Governor Layer**: `awccd` daemon running closed-loop thermal curves with anti-hunt hysteresis ($3^\circ\text{C}$).
3. **Kernel Subsystem Layer**: `alienware-wmi.ko` exposing `/sys/class/hwmon/hwmonX/fan[1-2]_boost` and `/sys/firmware/acpi/platform_profile`.
4. **Firmware & Hardware Layer**: Dell Embedded Controller (Microchip MEC152x) communicating over ACPI WMAX GUID `0x15` on `PNP0C14:0b`.
