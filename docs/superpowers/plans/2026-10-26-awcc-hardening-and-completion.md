# AWCC Linux Hardening & Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remediate all critical security vulnerabilities (world-writable sysfs), portability flaws (hardcoded user paths), semantic bugs (rhel-performance boost 204 vs 255), false success reporting, telemetry inaccuracies (hardcoded 3600 RPM max, 0°C fallback), and licensing/documentation discrepancies in the AWCC Linux suite.

**Architecture:** Refactor the Rust workspace (`awcc-core`, `awccctl`, `awccd`) to support dynamic sysfs root abstraction for mock hardware testing, genuine provider identification (`alienware-wmi` platform-profile class device), verified readback status, and safe failsafe thermal governors without world-writable permissions. Consolidate shell utilities into thin, robust clients with non-zero exit codes.

**Tech Stack:** Rust 2021 (workspace with `clap`, `colored`, `serde`, `serde_json`, `signal-hook`, `thiserror`), Linux sysfs / ACPI WMI, POSIX Bash, systemd, RPM packaging, GitHub Actions.

**Spec:** Repository Audit: `AWCC Linux Repository Audit & Completion Roadmap` (2026-10-26).

## Global Constraints
- No world-writable (`chmod 0666` or `0777`) sysfs permissions anywhere in scripts, udev rules, or specs.
- No hardcoded `/home/unknownuser` or user-specific home paths in any scripts, services, or binaries.
- Privileged actuation must be executed via `sudo`, root daemon, or authenticated broker; unprivileged users can only read telemetry.
- `rhel-performance` must request 255/255 (100% boost) on all controllable fans and verify readback.
- Missing sensor telemetry must never default to 0.0°C or 0 RPM; use `Option` types and explicit `Unavailable` states.
- Fan maximum RPM must never use a hardcoded constant (e.g. 3600); read `fanN_max` dynamically from sysfs or report `None`.
- G-Mode must be reported as genuine status or `NotExposed`/`Unknown`; never infer from `platform_profile == performance` or `boost >= 240`.
- All operations must return explicit exit codes: 0 for verified success, non-zero for any failure or readback mismatch.

## Review Focus
1. An unprivileged user attempts to write to sysfs fan controls directly: expected behavior is permission denied without corrupting system state.
2. Driver is unloaded or missing: CLI and daemon must immediately report `DriverNotFound` with non-zero exit code instead of crashing or panicking.
3. System sensor read fails or returns non-numeric data: daemon fails safe (releases to firmware or maintains cooling) rather than assuming 0°C and starving thermal headroom.
4. Fan boost write fails or firmware ignores write: CLI reports `ReadbackMismatch` with non-zero exit code instead of false success.
5. GNOME keybinding setup: appends to custom keybindings safely without clobbering existing shortcuts and provides safe rollback.

---

### Task 1: P0 Security — Remove World-Writable Sysfs Permissions

**Files:**
- Modify: `bin/install-system.sh:30-45`
- Modify: `bin/set-fan-speed.sh:20-30`
- Modify: `bin/awcc-mode:100-115`
- Modify: `bin/fix-permissions.sh:1-20`
- Modify: `packaging/rhel/awcc-linux.spec:50-56`
- Test: `tests/security_permissions_test.sh`

**Interfaces:**
- Consumes: `/sys/class/hwmon/hwmon*/fan*_boost`, `/sys/class/platform-profile/platform-profile-*/profile`
- Produces: Hardened udev rules (group ownership or root-only permissions) and removal of `chmod 0666`

- [ ] **Step 1: Write test script checking for world-writable sysfs rules**

```bash
cat << 'EOF' > tests/security_permissions_test.sh
#!/usr/bin/env bash
set -euo pipefail
# Fail if chmod 0666 or 666 appears in any executable or rule
if grep -rnE "chmod (0?666|777)" bin/ packaging/ systemd/ 2>/dev/null; then
    echo "FAIL: Found world-writable chmod instructions in repository!"
    exit 1
fi
echo "PASS: No world-writable permission patterns found."
EOF
chmod +x tests/security_permissions_test.sh
```

- [ ] **Step 2: Run test to verify it fails**

Run: `tests/security_permissions_test.sh`
Expected: FAIL with matches found in `bin/install-system.sh`, `bin/fix-permissions.sh`, `bin/set-fan-speed.sh`, `bin/awcc-mode`, and `packaging/rhel/awcc-linux.spec`.

- [ ] **Step 3: Remove all `chmod 0666` and world-writable udev rules**

- In `bin/install-system.sh`: Remove the `99-awcc-hwmon.rules` generation that runs `chmod 0666`. Privileged operations must be executed with `sudo` or via `awccd.service`.
- In `bin/fix-permissions.sh`: Replace `chmod 0666` with a diagnostic script that checks whether the current user has write access or informs the user to run privileged commands via `sudo`.
- In `bin/set-fan-speed.sh` and `bin/awcc-mode`: Remove `chmod 0666` invocations.
- In `packaging/rhel/awcc-linux.spec`: Remove `chmod 0666` udev rules.

- [ ] **Step 4: Run test to verify it passes**

Run: `tests/security_permissions_test.sh`
Expected: PASS with "No world-writable permission patterns found."

- [ ] **Step 5: Commit**

```bash
git add bin/ packaging/ tests/security_permissions_test.sh
git commit -m "fix(security): remove world-writable chmod 0666 sysfs rules"
```

---

### Task 2: P0 Portability — Remove Hardcoded Home-Directory Paths

**Files:**
- Modify: `bin/rhel-performance`
- Modify: `bin/rhel-balanced`
- Modify: `bin/rhel-ultra`
- Modify: `bin/rhel-quiet`
- Modify: `bin/rhel-mid`
- Modify: `bin/rhel-cool`
- Modify: `bin/load-driver.sh`
- Modify: `bin/install-system.sh`
- Modify: `bin/run-max-fans.sh`
- Modify: `bin/awcc-mode.sh`
- Modify: `bin/awcc-gameshift-toggle.sh`
- Modify: `bin/setup-f9-keybind.sh`
- Modify: `bin/bind-all-f9.sh`
- Test: `tests/portability_path_test.sh`

**Interfaces:**
- Consumes: Environment `PATH`, `$(cd "$(dirname "$0")/.." && pwd)`
- Produces: Portable scripts executable from any directory or user installation

- [ ] **Step 1: Write test script checking for hardcoded user paths**

```bash
cat << 'EOF' > tests/portability_path_test.sh
#!/usr/bin/env bash
set -euo pipefail
if grep -rn "/home/unknownuser" bin/ docs/ systemd/ packaging/ crates/ 2>/dev/null; then
    echo "FAIL: Found hardcoded user home path in codebase!"
    exit 1
fi
echo "PASS: No hardcoded home paths found."
EOF
chmod +x tests/portability_path_test.sh
```

- [ ] **Step 2: Run test to verify it fails**

Run: `tests/portability_path_test.sh`
Expected: FAIL with multiple matches in `bin/` scripts.

- [ ] **Step 3: Replace hardcoded paths with portable binary lookups and relative fallbacks**

- For `bin/rhel-*`: Find `awccctl` via `command -v awccctl` in `$PATH`, falling back to `$(dirname "$0")/../target/release/awccctl` or `$(dirname "$0")/../target/debug/awccctl`.
- For driver scripts (`bin/load-driver.sh`, `bin/run-max-fans.sh`, `bin/install-system.sh`): Use `REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"`.
- For keybinding scripts: Use `command -v awcc-gameshift-toggle` or `/usr/local/bin/awcc-gameshift-toggle` with `$HOME/.local/bin` fallback.

- [ ] **Step 4: Run test to verify it passes**

Run: `tests/portability_path_test.sh`
Expected: PASS with "No hardcoded home paths found."

- [ ] **Step 5: Commit**

```bash
git add bin/ tests/portability_path_test.sh
git commit -m "fix(portability): eliminate hardcoded home-directory paths"
```

---

### Task 3: P0 Max Boost Semantics — Correct `rhel-performance` to Request 255 Boost

**Files:**
- Modify: `bin/rhel-performance`
- Modify: `bin/set-profile-helper.sh`
- Test: `tests/test_performance_semantics.sh`

**Interfaces:**
- Consumes: `awccctl fan set --fan <n> --boost 255`
- Produces: Boost 255/255 on all controllable fans for `rhel-performance`

- [ ] **Step 1: Write verification test for `rhel-performance`**

```bash
cat << 'EOF' > tests/test_performance_semantics.sh
#!/usr/bin/env bash
set -euo pipefail
# Verify bin/rhel-performance passes boost 255, not 204
if grep -q "204" bin/rhel-performance; then
    echo "FAIL: bin/rhel-performance still passes 204 (80%) instead of 255 (100%)!"
    exit 1
fi
if ! grep -q "255" bin/rhel-performance; then
    echo "FAIL: bin/rhel-performance does not pass 255 boost!"
    exit 1
fi
echo "PASS: rhel-performance specifies 255 boost."
EOF
chmod +x tests/test_performance_semantics.sh
```

- [ ] **Step 2: Run test to verify it fails**

Run: `tests/test_performance_semantics.sh`
Expected: FAIL with `bin/rhel-performance still passes 204 (80%) instead of 255 (100%)!`

- [ ] **Step 3: Update `bin/rhel-performance` to request 255 boost**

Change `bin/rhel-performance` to invoke helper or `awccctl` with `performance` profile and boost `255` (`"MAX PERFORMANCE (Boost 255/255)"`).

- [ ] **Step 4: Run test to verify it passes**

Run: `tests/test_performance_semantics.sh`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add bin/rhel-performance tests/test_performance_semantics.sh
git commit -m "fix(fan): set rhel-performance boost to 255 for true maximum fan speed"
```

---

### Task 4: P0 Integrity — Eliminate False Success Messages and Propagate Exit Codes

**Files:**
- Modify: `bin/run-max-fans.sh`
- Modify: `bin/set-profile-helper.sh`
- Modify: `bin/stop-max-fans.sh`
- Modify: `crates/awccctl/src/main.rs:188-280`
- Test: `tests/test_error_exit_codes.sh`

**Interfaces:**
- Consumes: `Result<(), AwccError>` from `HardwareManager`
- Produces: Non-zero exit code upon any failed sysfs write, readback mismatch, or missing driver

- [ ] **Step 1: Write test verifying non-zero exit codes on failure**

```bash
cat << 'EOF' > tests/test_error_exit_codes.sh
#!/usr/bin/env bash
set -euo pipefail
# Verify that awccctl returns non-zero when given an invalid fan index
cargo build --bin awccctl
if ./target/debug/awccctl fan set --fan 99 --boost 100 2>/dev/null; then
    echo "FAIL: awccctl exited with 0 on invalid fan index!"
    exit 1
fi
echo "PASS: awccctl correctly exits non-zero on error."
EOF
chmod +x tests/test_error_exit_codes.sh
```

- [ ] **Step 2: Run test to verify behavior**

Run: `tests/test_error_exit_codes.sh`
Expected: PASS or verify where `main.rs` suppresses errors (e.g. `cmd_gmode` using `let _ = ...`, `cmd_fan_release` ignoring error).

- [ ] **Step 3: Refactor `crates/awccctl/src/main.rs` and shell helpers**

- In `cmd_gmode`: Check results of `set_platform_profile` and `write_fan_boost`. If any fail, print error and `std::process::exit(1)`.
- In `cmd_fan_release`: Check result of `write_fan_boost(fan, 0)`. If any fail, print error and `std::process::exit(1)`.
- In `bin/run-max-fans.sh`: Don't claim "maximum fan speed achieved and verified" unconditionally; report requested boost readback and current measured RPM honestly. If driver or files are missing, exit 1.
- In `bin/set-profile-helper.sh`: Propagate failure exit code if writing profile or boost fails.

- [ ] **Step 4: Run test to verify it passes**

Run: `tests/test_error_exit_codes.sh`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add bin/run-max-fans.sh bin/set-profile-helper.sh bin/stop-max-fans.sh crates/awccctl/src/main.rs tests/test_error_exit_codes.sh
git commit -m "fix(cli): propagate errors and eliminate false success messages"
```

---

### Task 5: P0 Licensing & Provenance — Add Root License and Document Origins

**Files:**
- Create: `LICENSE`
- Create: `LICENSES/GPL-2.0.txt`
- Create: `LICENSES/MIT.txt`
- Create: `docs/provenance-and-licenses.md`
- Modify: `packaging/rhel/awcc-linux.spec:5`

**Interfaces:**
- Consumes: SPDX identifiers in kernel files (`GPL-2.0-or-later`), Rust crate metadata (`MIT OR Apache-2.0`)
- Produces: Full legal compliance and provenance inventory

- [ ] **Step 1: Check existing SPDX headers in kernel code**

Run: `grep -rn "SPDX-License-Identifier" kernel/`
Expected: Shows GPL-2.0-or-later for alienware-wmi sources.

- [ ] **Step 2: Create root `LICENSE` file**

Add root `LICENSE` explaining the dual licensing structure:
- Kernel driver files: GNU General Public License v2.0 or later (GPL-2.0-or-later), originating from upstream Linux kernel drivers by Kurt Borja and Mario Limonciello.
- Rust crates and user utilities: MIT License.

- [ ] **Step 3: Add `docs/provenance-and-licenses.md`**

Document file-by-file licensing and upstream attribution.

- [ ] **Step 4: Verify license metadata in `packaging/rhel/awcc-linux.spec`**

Ensure `License: GPL-2.0-or-later AND MIT` matches the file inventory.

- [ ] **Step 5: Commit**

```bash
git add LICENSE LICENSES/ docs/provenance-and-licenses.md packaging/rhel/awcc-linux.spec
git commit -m "docs(legal): add root LICENSE, license texts, and provenance documentation"
```

---

### Task 6: P1 Dynamic Platform Profile Provider Discovery

**Files:**
- Modify: `crates/awcc-core/src/discovery.rs`
- Modify: `crates/awcc-core/src/error.rs`
- Test: `crates/awcc-core/tests/mock_hardware_tests.rs`

**Interfaces:**
- Consumes: `/sys/class/platform-profile/platform-profile-*/name`, `/sys/class/platform-profile/platform-profile-*/choices`, `/sys/class/platform-profile/platform-profile-*/profile`
- Produces: `HardwareManager::find_platform_profile_node() -> Option<PathBuf>`, dynamic choices validation, verified readback

- [ ] **Step 1: Add unit test in `mock_hardware_tests.rs` for dynamic platform profile discovery**

```rust
#[test]
fn test_dynamic_platform_profile_discovery() {
    let temp_dir = tempfile::tempdir().unwrap();
    let pp_dir = temp_dir.path().join("sys/class/platform-profile");
    fs::create_dir_all(pp_dir.join("platform-profile-0")).unwrap();
    fs::create_dir_all(pp_dir.join("platform-profile-1")).unwrap();

    fs::write(pp_dir.join("platform-profile-0/name"), "dell-pc\n").unwrap();
    fs::write(pp_dir.join("platform-profile-1/name"), "alienware-wmi\n").unwrap();
    fs::write(pp_dir.join("platform-profile-1/choices"), "quiet balanced performance custom\n").unwrap();
    fs::write(pp_dir.join("platform-profile-1/profile"), "balanced\n").unwrap();

    let node = HardwareManager::find_platform_profile_node_at(temp_dir.path()).unwrap();
    assert_eq!(node, pp_dir.join("platform-profile-1"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test mock_hardware_tests`
Expected: FAIL (method not defined or compilation failure).

- [ ] **Step 3: Implement dynamic platform-profile discovery and profile setting in `discovery.rs`**

- Search `/sys/class/platform-profile/platform-profile-*` for `name` == `alienware-wmi`.
- Fall back to standard `/sys/firmware/acpi/platform_profile` if class device is absent.
- Read available choices; validate requested profile against choices before writing; read back and verify.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test mock_hardware_tests`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/awcc-core/ tests/
git commit -m "feat(core): discover platform-profile class device by provider identity"
```

---

### Task 7: P1 Genuine Sensor Telemetry & Dynamic Fan Max

**Files:**
- Modify: `crates/awcc-core/src/discovery.rs`
- Modify: `crates/awcc-core/src/telemetry.rs`
- Modify: `crates/awccd/src/main.rs`
- Test: `crates/awcc-core/tests/mock_hardware_tests.rs`

**Interfaces:**
- Consumes: `fanN_label`, `fanN_min`, `fanN_max`, `fanN_input`, `tempN_label`, `tempN_input`
- Produces: Non-zero, truthful `Option<u32>` for max RPM, `Option<f32>` for temperatures, failsafe handling in daemon

- [ ] **Step 1: Write test verifying genuine sensor telemetry and dynamic fan max**

```rust
#[test]
fn test_dynamic_fan_max_and_label_parsing() {
    let temp_dir = tempfile::tempdir().unwrap();
    let hwmon_dir = temp_dir.path().join("sys/class/hwmon/hwmon7");
    fs::create_dir_all(&hwmon_dir).unwrap();
    fs::write(hwmon_dir.join("name"), "alienware_wmi\n").unwrap();
    fs::write(hwmon_dir.join("fan1_label"), "CPU Fan\n").unwrap();
    fs::write(hwmon_dir.join("fan1_max"), "4800\n").unwrap();
    fs::write(hwmon_dir.join("fan1_input"), "2100\n").unwrap();
    fs::write(hwmon_dir.join("fan1_boost"), "50\n").unwrap();

    let fans = HardwareManager::read_fans_at(temp_dir.path(), &hwmon_dir);
    assert_eq!(fans[0].label, "CPU Fan");
    assert_eq!(fans[0].reported_max_rpm, Some(4800));
    assert_eq!(fans[0].current_rpm, 2100);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test mock_hardware_tests`
Expected: FAIL.

- [ ] **Step 3: Implement dynamic fan and temperature parsing**

- In `discovery.rs`:
  - Read `fanN_label` from sysfs (e.g. "CPU Fan", "GPU Fan") instead of hardcoding.
  - Read `fanN_max` (e.g. 4800) instead of hardcoding 3600.
  - Read `tempN_label` and map accordingly.
  - Represent temperatures with `Option<f32>`: if sensor is absent, do not default to 0.0°C!
- In `awccd/src/main.rs`:
  - If temperatures cannot be read (all `None`), fail safe: log warning and do not suppress fan boost.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test mock_hardware_tests`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/awcc-core/ crates/awccd/ crates/awcc-core/tests/
git commit -m "feat(telemetry): parse dynamic fan limits and labels; eliminate fake 0°C readings"
```

---

### Task 8: P1 G-Mode State Honesty

**Files:**
- Modify: `crates/awcc-core/src/discovery.rs`
- Modify: `crates/awcc-core/src/telemetry.rs`
- Modify: `crates/awccctl/src/main.rs`
- Modify: `bin/awcc-gameshift-toggle.sh`
- Test: `crates/awcc-core/tests/mock_hardware_tests.rs`

**Interfaces:**
- Consumes: ACPI WMI Game Shift status interface (if present)
- Produces: `GModeStatus` enum (`Active`, `Inactive`, `NotExposed`, `Unsupported`)

- [ ] **Step 1: Write test verifying G-Mode is not inferred from performance profile**

```rust
#[test]
fn test_gmode_not_inferred_from_performance_profile() {
    let status = HardwareManager::check_gmode_status(/* mock with no gmode sysfs */);
    assert_eq!(status, GModeStatus::NotExposed);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test mock_hardware_tests`
Expected: FAIL.

- [ ] **Step 3: Implement `GModeStatus` enum and remove false inferences**

- Stop equating `profile == "performance"` with G-Mode.
- In `bin/awcc-gameshift-toggle.sh`: Remove boost >= 240 inference. Toggle profile between `performance` (with boost 255) and `balanced` (with boost 0), and report explicit target state.
- In `cmd_doctor` and `cmd_status`: Display G-Mode as `Not Exposed by Driver Interface (Actuated via Performance Profile + 255 Boost)`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test mock_hardware_tests`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/awcc-core/ crates/awccctl/ bin/awcc-gameshift-toggle.sh
git commit -m "fix(gmode): report truthful G-Mode status instead of inferring from profile"
```

---

### Task 9: P1 Rust Crate Hardening & Comprehensive Error Handling

**Files:**
- Modify: `crates/awcc-core/src/error.rs`
- Modify: `crates/awcc-core/src/discovery.rs`
- Modify: `crates/awccctl/src/main.rs`
- Modify: `crates/awccd/src/main.rs`
- Test: `crates/awcc-core/tests/unit_tests.rs`

**Interfaces:**
- Consumes: Strict error types
- Produces: Zero panics, proper exit codes, and configurable sysfs root

- [ ] **Step 1: Write tests for error types and invalid inputs**

- [ ] **Step 2: Run test to verify it fails**

- [ ] **Step 3: Implement strict error handling and `--sysfs-root` support for testing**

- Add `AwccError::InvalidProfile`, `AwccError::SensorUnavailable`, `AwccError::PermissionDenied`.
- Support `--sysfs-root` in `HardwareManager` for deterministic test fixtures.
- Return exit code `1` on hardware/IO failures, exit code `2` on usage errors.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --workspace`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/awcc-core/ crates/awccctl/ crates/awccd/
git commit -m "refactor(crates): harden error handling and add sysfs root abstraction"
```

---

### Task 10: P1 Shell Consolidation & Safe GNOME Keybindings

**Files:**
- Modify: `bin/awcc-mode`
- Modify: `bin/setup-f9-keybind.sh`
- Create: `bin/uninstall-f9-keybind.sh`
- Test: `tests/shell_syntax_check.sh`

**Interfaces:**
- Consumes: `gsettings` custom keybindings list
- Produces: Safe append without overwriting user shortcuts; reversible uninstall script

- [ ] **Step 1: Write test checking shell syntax across all scripts**

```bash
cat << 'EOF' > tests/shell_syntax_check.sh
#!/usr/bin/env bash
set -euo pipefail
for s in bin/*.sh bin/awcc-* bin/rhel-*; do
    if [[ -f "$s" ]]; then
        bash -n "$s"
    fi
done
echo "PASS: All shell scripts passed bash -n syntax check."
EOF
chmod +x tests/shell_syntax_check.sh
```

- [ ] **Step 2: Run test to verify syntax**

Run: `tests/shell_syntax_check.sh`
Expected: PASS.

- [ ] **Step 3: Update `setup-f9-keybind.sh` to safely append and provide unbinder**

- Read current custom keybindings list via `gsettings get org.gnome.settings-daemon.plugins.media-keys custom-keybindings`.
- Append a new keybinding path without wiping the existing array.
- Create `bin/uninstall-f9-keybind.sh` to cleanly remove the added binding and restore original list.

- [ ] **Step 4: Run test to verify syntax**

Run: `tests/shell_syntax_check.sh`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add bin/ tests/shell_syntax_check.sh
git commit -m "fix(scripts): safe non-destructive F9 keybinding registration and rollback"
```

---

### Task 11: P1 Mock Hardware & CI Suite

**Files:**
- Create: `crates/awcc-core/tests/mock_hardware_tests.rs`
- Create: `.github/workflows/ci.yml`
- Test: `cargo test --workspace`

**Interfaces:**
- Consumes: In-memory/temporary mock `/sys` trees
- Produces: Complete automated test suite covering missing files, permission errors, and arbitrary hwmon indexes

- [ ] **Step 1: Create mock hardware test suite in `mock_hardware_tests.rs`**

- Test arbitrary hwmon indexes (`hwmon0` through `hwmon15`).
- Test missing or unreadable `fan1_boost`.
- Test readback mismatch.
- Test missing temperature sensors.

- [ ] **Step 2: Run `cargo test --workspace`**

Run: `cargo test --workspace`
Expected: PASS.

- [ ] **Step 3: Create GitHub Actions workflow `.github/workflows/ci.yml`**

- Runs `cargo test --workspace`.
- Runs `cargo clippy --workspace -- -D warnings`.
- Runs `cargo fmt --check`.
- Runs `tests/security_permissions_test.sh`, `tests/portability_path_test.sh`, `tests/shell_syntax_check.sh`.

- [ ] **Step 4: Verify CI checks locally**

Run: `cargo clippy --workspace -- -D warnings && cargo fmt --check`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/awcc-core/tests/ .github/workflows/ci.yml
git commit -m "ci: add automated CI workflow and mock hardware test suite"
```

---

### Task 12: Documentation Truthfulness & RPM Spec Hardening

**Files:**
- Modify: `README.md`
- Modify: `docs/compatibility-matrix.md`
- Modify: `docs/safety-model.md`
- Modify: `packaging/rhel/awcc-linux.spec`
- Test: `tests/doc_claims_verification.sh`

**Interfaces:**
- Consumes: Verified hardware facts from G15 5530 testbed
- Produces: Honest, accurate documentation and spec file

- [ ] **Step 1: Update README.md to eliminate overclaims**

- Clarify that the cybernetic cockpit is an advanced terminal UI (TUI), not a desktop GUI.
- Record accurate fan RPM limits (up to ~4800 RPM observed on G15 5530 hardware, not arbitrary numbers).
- Clearly explain the privileged execution model (via `sudo` or `awccd.service`, no world-writable sysfs).

- [ ] **Step 2: Update `docs/compatibility-matrix.md` and `docs/safety-model.md`**

- Distinguish physically verified hardware (Dell G15 5530) from community/unverified models.
- Document threat model without world-writable permissions.

- [ ] **Step 3: Fix `packaging/rhel/awcc-linux.spec`**

- Set `URL: https://github.com/Monkey0-9/Awcc`.
- Ensure DKMS errors are not silently ignored with `|| true`.
- Remove `chmod 0666` rules.

- [ ] **Step 4: Run all test scripts**

Run: `tests/security_permissions_test.sh && tests/portability_path_test.sh && tests/test_performance_semantics.sh && cargo test --workspace`
Expected: All PASS.

- [ ] **Step 5: Commit**

```bash
git add README.md docs/ packaging/rhel/awcc-linux.spec
git commit -m "docs: align documentation and RPM spec with verified hardware facts and security model"
```
