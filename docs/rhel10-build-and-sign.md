# RHEL 10 Build, DKMS, and Secure Boot Guide

## 1. Prerequisites
- `kernel-devel` matching `$(uname -r)`
- `gcc`, `make`, `openssl`, `cargo`, `rustc`
- `dkms` (optional, for automated rebuilds across kernel updates)

## 2. Building and Signing
```bash
cd kernel
make
./scripts/sign-module.sh
```

## 3. Secure Boot MOK Enrollment (If Secure Boot is Enabled)
If `mokutil --sb-state` reports Secure Boot is active:
```bash
sudo mokutil --import kernel/certs/mok.der
```
Enter a temporary password, reboot the laptop, select **Enroll MOK** at the blue MOKManager screen, enter the password, and continue boot. The driver is now trusted by the UEFI firmware.
