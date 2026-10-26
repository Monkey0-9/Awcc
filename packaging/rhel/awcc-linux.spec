Name:           awcc-linux
Version:        2.5.0
Release:        1%{?dist}
Summary:        Alienware Command Center (AWCC) & Game Shift Control Suite for Dell Laptops on Linux
License:        GPL-2.0-only AND MIT
URL:            https://github.com/awcc-linux/awcc-linux
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  gcc make cargo rust openssl-devel
Requires:       dkms kernel-devel python3 libnotify libcanberra-gtk3

%description
Enterprise-grade fan control, Game Shift (G-Mode) hotkey management,
platform profile actuation, and TrueColor cybernetic telemetry cockpit
for Dell G-Series and Alienware laptops (specifically Dell G15 5530).

%prep
%setup -q

%build
cargo build --release --manifest-path Cargo.toml
make -C kernel

%install
rm -rf $RPM_BUILD_ROOT

# Binaries
install -d $RPM_BUILD_ROOT%{_bindir}
install -m 0755 target/release/awccctl $RPM_BUILD_ROOT%{_bindir}/awccctl
install -m 0755 target/release/awccd $RPM_BUILD_ROOT%{_bindir}/awccd

# Shell wrappers and governors
install -m 0755 bin/awcc-mode.sh $RPM_BUILD_ROOT%{_bindir}/awcc-mode
install -m 0755 bin/awcc-battery $RPM_BUILD_ROOT%{_bindir}/awcc-battery
install -m 0755 bin/awcc-quiet $RPM_BUILD_ROOT%{_bindir}/awcc-quiet
install -m 0755 bin/awcc-balance $RPM_BUILD_ROOT%{_bindir}/awcc-balance
install -m 0755 bin/awcc-mid $RPM_BUILD_ROOT%{_bindir}/awcc-mid
install -m 0755 bin/awcc-high $RPM_BUILD_ROOT%{_bindir}/awcc-high
install -m 0755 bin/awcc-ultra $RPM_BUILD_ROOT%{_bindir}/awcc-ultra
install -m 0755 bin/awcc-gameshift-toggle.sh $RPM_BUILD_ROOT%{_bindir}/awcc-gameshift-toggle
ln -sf %{_bindir}/awcc-gameshift-toggle $RPM_BUILD_ROOT%{_bindir}/gmode

# Telemetry Cockpit
install -m 0755 bin/awcc-lux-dashboard.py $RPM_BUILD_ROOT%{_bindir}/awcc-lux-dashboard.py

# Systemd Units
install -d $RPM_BUILD_ROOT%{_unitdir}
install -m 0644 systemd/awccd.service $RPM_BUILD_ROOT%{_unitdir}/awccd.service

# Udev Rules
install -d $RPM_BUILD_ROOT%{_udevrulesdir}
cat > $RPM_BUILD_ROOT%{_udevrulesdir}/99-awcc-hwmon.rules << 'EOF'
ACTION=="add", SUBSYSTEM=="hwmon", ATTR{name}=="alienware_wmi", RUN+="/bin/chmod 0666 /sys%p/fan1_boost /sys%p/fan2_boost"
ACTION=="add", SUBSYSTEM=="platform", KERNEL=="alienware-wmi", RUN+="/bin/chmod 0666 /sys/firmware/acpi/platform_profile"
EOF

# DKMS Driver Source
install -d $RPM_BUILD_ROOT/usr/src/alienware-wmi-%{version}
cp -r kernel/* $RPM_BUILD_ROOT/usr/src/alienware-wmi-%{version}/

%post
# Register and install DKMS module
dkms add -m alienware-wmi -v %{version} || true
dkms build -m alienware-wmi -v %{version} || true
dkms install -m alienware-wmi -v %{version} || true
udevadm control --reload-rules || true
udevadm trigger --subsystem-match=hwmon || true
%systemd_post awccd.service

%preun
%systemd_preun awccd.service
if [ $1 -eq 0 ]; then
    dkms remove -m alienware-wmi -v %{version} --all || true
fi

%postun
%systemd_postun_with_restart awccd.service

%files
%{_bindir}/awccctl
%{_bindir}/awccd
%{_bindir}/awcc-mode
%{_bindir}/awcc-battery
%{_bindir}/awcc-quiet
%{_bindir}/awcc-balance
%{_bindir}/awcc-mid
%{_bindir}/awcc-high
%{_bindir}/awcc-ultra
%{_bindir}/awcc-gameshift-toggle
%{_bindir}/gmode
%{_bindir}/awcc-lux-dashboard.py
%{_unitdir}/awccd.service
%{_udevrulesdir}/99-awcc-hwmon.rules
/usr/src/alienware-wmi-%{version}

%changelog
* Sun Oct 26 2026 AWCC Linux Enterprise Team <support@awcc-linux.org> - 2.5.0-1
- Complete 6-tier fan and power profile matrix (Battery, Quiet, Balance, Mid, High, Ultra)
- Direct integrated TrueColor Cybernetic Cockpit with East Asian width support
- Hardware Game Shift (F9) instantaneous async toggle (<120ms) with desktop OSD
- Unprivileged 0666 udev rules for zero-sudo daily operation
- Anti-hunt hysteresis damping governor in awccd daemon
