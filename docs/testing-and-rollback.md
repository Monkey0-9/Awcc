# AWCC Linux Testing, Verification, and Rollback Guide

## 1. Safety Principles & Clean Rollback Path
The project does not alter stock Linux kernel binaries or write to BIOS flash memory. All changes to thermal settings or fan profiles can be cleanly undone.

### Automatic Clean Uninstallation:
```bash
sudo ./bin/install-system.sh --uninstall
# Or run the clean uninstallation utility directly:
sudo ./bin/uninstall-system.sh
```

### Manual Rollback Steps:
1. Revert fans to firmware control:
```bash
awccctl fan release
```
2. Stop and remove the systemd service:
```bash
sudo systemctl stop awccd.service 2>/dev/null || true
sudo systemctl disable awccd.service 2>/dev/null || true
sudo rm -f /etc/systemd/system/awccd.service
sudo systemctl daemon-reload
```
3. Remove user binaries:
```bash
rm -f ~/.local/bin/awcc* ~/.local/bin/gmode
```
4. Unload the kernel driver (if loaded on Dell hardware):
```bash
sudo modprobe -r alienware-wmi 2>/dev/null || sudo rmmod alienware-wmi 2>/dev/null || true
```

---

## 2. Hardware Verification Protocol
1. Run `awccctl doctor` to ensure the platform and capabilities are detected.
2. Check `awccctl status` to confirm active sensors, profiles, and fans appear.
3. Run `awcc-ultra` (or `awccctl ultra`) to actuate Game Shift and maximum boost (255/255).
4. Verify turbines ramp from idle up to rated ceiling (~4,800\(en5,029 RPM).
5. Run `awcc-balanced` (or `awccctl balanced`) to verify fans decelerate to quiet acoustic levels.
6. Run `awccctl track` to stream real-time tachometer RPM and thermal feedback.
