#!/usr/bin/env bash
# ==============================================================================
# tests/distro_packaging_syntax_test.sh
# Syntax and validation test for multi-distribution packaging & service files.
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Running Multi-Distribution Packaging Syntax Tests ==="
PASSED=0
FAILED=0

assert_ok() {
    local desc="$1"
    local status="$2"
    if [[ "$status" -eq 0 ]]; then
        echo -e "\e[32m[PASS]\e[0m ${desc}"
        PASSED=$((PASSED + 1))
    else
        echo -e "\e[31m[FAIL]\e[0m ${desc}"
        FAILED=$((FAILED + 1))
    fi
}

# 1. RPM Specs parsing (Fedora, openSUSE, RHEL)
if command -v rpmspec >/dev/null 2>&1; then
    rpmspec -P "${REPO_DIR}/packaging/fedora/awcc-linux.spec" >/dev/null 2>&1
    assert_ok "Fedora RPM Spec syntax valid (rpmspec -P)" $?

    rpmspec -P "${REPO_DIR}/packaging/opensuse/awcc-linux.spec" >/dev/null 2>&1
    assert_ok "openSUSE RPM Spec syntax valid (rpmspec -P)" $?

    rpmspec -P "${REPO_DIR}/packaging/rhel/awcc-linux.spec" >/dev/null 2>&1
    assert_ok "RHEL RPM Spec syntax valid (rpmspec -P)" $?
else
    # Fallback to grep validation of required sections
    for spec in "${REPO_DIR}/packaging/fedora/awcc-linux.spec" "${REPO_DIR}/packaging/opensuse/awcc-linux.spec" "${REPO_DIR}/packaging/rhel/awcc-linux.spec"; do
        if grep -q "%description" "$spec" && grep -q "%files" "$spec"; then
            assert_ok "RPM Spec has required sections: $(basename "$spec")" 0
        else
            assert_ok "RPM Spec has required sections: $(basename "$spec")" 1
        fi
    done
fi

# 2. Debian packaging files
if grep -q "Source: awcc-linux" "${REPO_DIR}/packaging/debian/control" && grep -q "Package: awcc-linux" "${REPO_DIR}/packaging/debian/control"; then
    assert_ok "Debian control file structure valid" 0
else
    assert_ok "Debian control file structure valid" 1
fi

if make -n -f "${REPO_DIR}/packaging/debian/rules" >/dev/null 2>&1; then
    assert_ok "Debian rules makefile syntax valid" 0
else
    assert_ok "Debian rules makefile syntax valid" 1
fi

# 3. Arch Linux PKGBUILD syntax
if bash -n "${REPO_DIR}/packaging/arch/PKGBUILD"; then
    assert_ok "Arch Linux PKGBUILD syntax valid" 0
else
    assert_ok "Arch Linux PKGBUILD syntax valid" 1
fi

# 4. NixOS definitions
if [[ -f "${REPO_DIR}/packaging/nixos/default.nix" && -f "${REPO_DIR}/packaging/nixos/flake.nix" ]]; then
    if grep -q "buildRustPackage" "${REPO_DIR}/packaging/nixos/default.nix" && grep -q "flake-utils" "${REPO_DIR}/packaging/nixos/flake.nix"; then
        assert_ok "NixOS default.nix and flake.nix structure valid" 0
    else
        assert_ok "NixOS default.nix and flake.nix structure valid" 1
    fi
else
    assert_ok "NixOS files exist" 1
fi

# 5. Init service scripts
if bash -n "${REPO_DIR}/systemd/openrc-awccd.initd"; then
    assert_ok "OpenRC init script syntax valid" 0
else
    assert_ok "OpenRC init script syntax valid" 1
fi

if bash -n "${REPO_DIR}/systemd/runit-awccd.run"; then
    assert_ok "Runit run script syntax valid" 0
else
    assert_ok "Runit run script syntax valid" 1
fi

# 6. Management and recovery scripts
for script in "${REPO_DIR}/bin/install-system.sh" "${REPO_DIR}/bin/uninstall-system.sh" "${REPO_DIR}/bin/doctor-recovery.sh"; do
    if bash -n "$script"; then
        assert_ok "Script syntax valid: $(basename "$script")" 0
    else
        assert_ok "Script syntax valid: $(basename "$script")" 1
    fi
done

echo "=== Summary: ${PASSED} Passed, ${FAILED} Failed ==="
if [[ "$FAILED" -gt 0 ]]; then
    exit 1
fi
