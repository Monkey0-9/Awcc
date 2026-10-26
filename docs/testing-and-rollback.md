# AWCC Linux Testing, Verification, and Rollback Guide

## 1. Safety Principles & Rollback Path
The project does not alter stock RHEL kernel binaries or overwrite BIOS flash memory. 

### To Unload the Driver:
```bash
sudo rmmod alienware-wmi
```

### To Completely Remove the Kernel Module:
```bash
sudo rm -f /lib/modules/$(uname -r)/extra/alienware-wmi.ko*
sudo depmod -a
```

### To Uninstall the CLI and Daemons:
```bash
rm -f ~/.local/bin/awccctl ~/.local/bin/awccd ~/.local/bin/rhel-*
```

---

## 2. Hardware Verification Protocol
1. Run `awccctl doctor` to ensure the WMAX WMI bus (`PNP0C14:0b`) is detected.
2. Load driver via `sudo insmod Awcc/kernel/alienware-wmi.ko`.
3. Check `awccctl status` to confirm `fan1_boost` and `fan2_boost` appear.
4. Run `rhel-ultra` to actuate Game Shift (G-Mode) and maximum boost (255).
5. Verify fans ramp from ~2000 RPM up to ~3500+ RPM.
6. Run `rhel-balanced` to verify boost returns to 0 and fans decelerate.
