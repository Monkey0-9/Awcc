# Specification: Universal Linux Hardware Platform & Dynamic Discovery

## Purpose
Provide a modular, vendor-agnostic architecture for hardware performance and thermal control across Linux distributions and PC manufacturers.

## Requirements

### Requirement: Multi-Vendor Hardware Provider Abstraction
The core engine SHALL implement dynamic hardware discovery via modular provider implementations conforming to the `HardwareProvider` interface.

#### Scenario: Hardware provider detection
- **WHEN** the system starts up or executes discovery
- **THEN** it probes DMI vendor, product, and active sysfs driver trees to bind the highest-priority matching hardware provider (Dell, ASUS, Lenovo, HP, Acer, MSI, System76, Framework, or Generic).

#### Scenario: Unsupported hardware graceful fallback
- **WHEN** a system does not match any specialized vendor driver
- **THEN** it binds to `GenericLinuxProvider` and exposes only standard ACPI platform profiles and standard hwmon PWM nodes, returning explicit capability flags.

### Requirement: Strict Security and Safety Contract
The platform SHALL operate without requiring world-writable file permissions and SHALL enforce non-root unprivileged telemetry reading.

#### Scenario: Unprivileged telemetry
- **WHEN** an unprivileged user executes `awccctl status` or `awcc-lux-dashboard.py`
- **THEN** all telemetry is read from standard read-only sysfs nodes without sudo prompts.

#### Scenario: Privileged actuation isolation
- **WHEN** a user requests fan boost or profile change
- **THEN** the privileged actuation is executed through the daemon broker or restricted sudoers entry, with sysfs nodes maintaining 0644 root permissions.
