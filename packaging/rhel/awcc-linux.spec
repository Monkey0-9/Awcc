Name:           awcc-linux
Version:        2.5.0
Release:        1%{?dist}
Summary:        Alienware Command Center (AWCC) & Game Shift Control Suite for Dell Laptops on Linux
License:        GPL-2.0-or-later AND MIT
URL:            https://github.com/Monkey0-9/Awcc
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
install -m 0755 bin/awcc-gui $RPM_BUILD_ROOT%{_bindir}/awcc-gui

# Shell wrappers and governors
install -m 0755 bin/awcc-mode.sh $RPM_BUILD_ROOT%{_bindir}/awcc-mode
install -m 0755 bin/awcc-track $RPM_BUILD_ROOT%{_bindir}/awcc-track
install -m 0755 bin/awcc-monitor $RPM_BUILD_ROOT%{_bindir}/awcc-monitor
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

# Desktop Entry, Icon, and PolicyKit Policy
install -d $RPM_BUILD_ROOT%{_datadir}/applications
install -m 0644 packaging/awcc-gui.desktop $RPM_BUILD_ROOT%{_datadir}/applications/awcc-gui.desktop
install -d $RPM_BUILD_ROOT%{_datadir}/icons/hicolor/scalable/apps
install -m 0644 packaging/icons/awcc-gui.svg $RPM_BUILD_ROOT%{_datadir}/icons/hicolor/scalable/apps/awcc-gui.svg
install -d $RPM_BUILD_ROOT%{_datadir}/polkit-1/actions
install -m 0644 packaging/io.github.monkey0_9.awcc.policy $RPM_BUILD_ROOT%{_datadir}/polkit-1/actions/io.github.monkey0_9.awcc.policy

# Shell Completions
install -d $RPM_BUILD_ROOT%{_datadir}/bash-completion/completions
install -m 0644 completions/bash/awccctl $RPM_BUILD_ROOT%{_datadir}/bash-completion/completions/awccctl
install -d $RPM_BUILD_ROOT%{_datadir}/zsh/site-functions
install -m 0644 completions/zsh/_awccctl $RPM_BUILD_ROOT%{_datadir}/zsh/site-functions/_awccctl
install -d $RPM_BUILD_ROOT%{_datadir}/fish/vendor_completions.d
install -m 0644 completions/fish/awccctl.fish $RPM_BUILD_ROOT%{_datadir}/fish/vendor_completions.d/awccctl.fish

# Man Pages
install -d $RPM_BUILD_ROOT%{_mandir}/man1
install -m 0644 man/man1/*.1 $RPM_BUILD_ROOT%{_mandir}/man1/

# Systemd Units
install -d $RPM_BUILD_ROOT%{_unitdir}
install -m 0644 systemd/awccd.service $RPM_BUILD_ROOT%{_unitdir}/awccd.service

# DKMS Driver Source
install -d $RPM_BUILD_ROOT/usr/src/alienware-wmi-%{version}
cp -r kernel/* $RPM_BUILD_ROOT/usr/src/alienware-wmi-%{version}/

%post
# Register and install DKMS module
dkms add -m alienware-wmi -v %{version} || true
dkms build -m alienware-wmi -v %{version} || true
dkms install -m alienware-wmi -v %{version} || true
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
%{_bindir}/awcc-gui
%{_bindir}/awcc-mode
%{_bindir}/awcc-track
%{_bindir}/awcc-monitor
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
%{_datadir}/applications/awcc-gui.desktop
%{_datadir}/icons/hicolor/scalable/apps/awcc-gui.svg
%{_datadir}/polkit-1/actions/io.github.monkey0_9.awcc.policy
%{_datadir}/bash-completion/completions/awccctl
%{_datadir}/zsh/site-functions/_awccctl
%{_datadir}/fish/vendor_completions.d/awccctl.fish
%{_mandir}/man1/*.1*
/usr/src/alienware-wmi-%{version}

%changelog
* Sun Oct 26 2026 AWCC Linux Enterprise Team <support@awcc-linux.org> - 2.5.0-1
- Complete 6-tier fan and power profile matrix (Battery, Quiet, Balance, Mid, High, Ultra)
- Direct integrated TrueColor Cybernetic Cockpit with East Asian width support
- Hardware Game Shift (F9) instantaneous async toggle (<120ms) with desktop OSD
- Hardened root/broker privilege boundary for fan controls
- Anti-hunt hysteresis damping governor in awccd daemon
