# Specification: Thermal & Fan Governor Engine

## Purpose
Regulate system acoustics and thermal dissipation through a 6-tier platform profile matrix and autonomous closed-loop hysteresis damping.

## Requirements

### Requirement: 6-Tier Platform Profile Matrix
The platform SHALL support six distinct operational modes (`battery`, `quiet`, `balance`, `mid`, `high`, `ultra`) balancing acoustic noise against cooling performance.

#### Scenario: Set profile mode
- **WHEN** a user or client requests actuation of a platform profile (`quiet`, `balance`, `ultra`, etc.)
- **THEN** the platform sets the underlying ACPI platform profile and configures the corresponding fan boost level.

#### Scenario: Maximum fan boost
- **WHEN** the `ultra` or max fan boost mode is requested on a supported machine
- **THEN** fan boost is set to 255/255 and readback verification is executed.

### Requirement: Autonomous Daemon Hysteresis
The `awccd` daemon SHALL execute closed-loop temperature monitoring with anti-hunt hysteresis.

#### Scenario: Thermal spike response
- **WHEN** package temperature exceeds 75°C
- **THEN** the daemon initiates fan ramp-up within 100ms.

#### Scenario: Temperature reduction damping
- **WHEN** package temperature decreases below the profile threshold
- **THEN** the daemon enforces a 3-5 second hysteresis delay before down-ramping fan speed to prevent acoustic hunting.
