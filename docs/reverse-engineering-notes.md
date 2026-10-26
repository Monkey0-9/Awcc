# AWCC WMAX Reverse-Engineering & ACPI Protocol Analysis

Dell Alienware Command Center (AWCC) interacts with laptop hardware via ACPI WMI Object `WMAX` (GUID `A70591CE-A997-11DA-B012-B622A1EF5492`).

## Method Map
- `0x03` (`WMAX_METHOD_BRIGHTNESS`): Controls AlienFX LED brightness.
- `0x04` (`WMAX_METHOD_ZONE_CONTROL`): Sets per-zone RGB colors.
- `0x13` (`AWCC_METHOD_GET_FAN_SENSORS`): Queries fan IDs mapped to thermal zones.
- `0x14` (`AWCC_METHOD_THERMAL_INFORMATION`):
  - Operation `0x02`: System Description.
  - Operation `0x03`: Resource ID enumeration.
  - Operation `0x04`: Temperature query.
  - Operation `0x05`: Fan speed in RPM.
  - Operation `0x08`: Minimum fan RPM.
  - Operation `0x09`: Maximum rated RPM.
  - Operation `0x0B`: Active thermal profile.
  - Operation `0x0C`: Current fan boost level (0-255).
- `0x15` (`AWCC_METHOD_THERMAL_CONTROL`):
  - Operation `0x01`: Activate thermal profile.
  - Operation `0x02`: Set manual fan boost (0-255).
- `0x25` (`AWCC_METHOD_GAME_SHIFT_STATUS`):
  - Operation `0x01`: Toggle Game Shift / G-Mode (`0xAB`).
  - Operation `0x02`: Read Game Shift active state.
