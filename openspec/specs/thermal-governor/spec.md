# Specification: Thermal & Fan Governor Engine

## Purpose
Regulate system acoustics and thermal dissipation through a canonical 5-tier platform profile matrix (with 20% fan boost increments) and autonomous closed-loop hysteresis damping.

## Requirements

### Requirement: Canonical 5-Tier Platform Profile Matrix
The platform SHALL support five canonical operational modes (`battery`, `quiet`, `balanced`, `performance`, `ultra`) with 20% fan boost increments up to 255/255 (100% max RPM), alongside legacy compatibility aliases (`balance`, `mid`, `high`).

#### Scenario: Set profile mode
- **WHEN** a user or client requests actuation of a platform profile (`battery`, `quiet`, `balanced`, `performance`, `ultra`, etc.)
- **THEN** the platform sets the underlying ACPI platform profile and configures the corresponding 20% quantized fan boost level (51, 102, 153, 204, 255).

#### Scenario: Maximum fan boost
- **WHEN** the `ultra` or max fan boost mode is requested on a supported machine
- **THEN** fan boost is set to 255/255 (100% Max RPM) and readback verification is executed.

### Requirement: Autonomous Daemon Hysteresis
The `awccd` daemon SHALL execute closed-loop temperature monitoring with anti-hunt hysteresis.

#### Scenario: Thermal spike response
- **WHEN** package temperature exceeds 75°C
- **THEN** the daemon initiates fan ramp-up within 100ms.

#### Scenario: Temperature reduction damping
- **WHEN** package temperature decreases below the profile threshold
- **THEN** the daemon enforces a 3-5 second hysteresis delay before down-ramping fan speed to prevent acoustic hunting.
