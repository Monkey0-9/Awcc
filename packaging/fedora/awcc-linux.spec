Name:           awcc-linux
Version:        0.3.0
Release:        1%{?dist}
Summary:        Universal Linux Hardware Performance, Fan Control & Telemetry Suite
License:        GPL-2.0-or-later AND MIT
URL:            https://github.com/Monkey0-9/Awcc
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  gcc make cargo rust openssl-devel desktop-file-utils
Requires:       python3 python3-gobject gtk4 libadwaita libnotify libcanberra-gtk3 systemd

%description
Universal Linux hardware performance, fan control, platform profile management,
and telemetry monitoring suite supporting Dell Alienware, ASUS, Lenovo, HP,
Acer, MSI, Framework, System76, and standard Linux desktop motherboards. Includes CLI,
responsive terminal dashboard with TrueColor color grading, and modern GTK4 GUI.

%prep
%setup -q

%build
cargo build --release --manifest-path Cargo.toml

%install
rm -rf $RPM_BUILD_ROOT

# Binaries
install -d $RPM_BUILD_ROOT%{_bindir}
install -m 0755 target/release/awccctl $RPM_BUILD_ROOT%{_bindir}/awccctl
install -m 0755 target/release/awccd $RPM_BUILD_ROOT%{_bindir}/awccd
install -m 0755 bin/awcc-gui $RPM_BUILD_ROOT%{_bindir}/awcc-gui
install -m 0755 bin/awcc-lux-dashboard.py $RPM_BUILD_ROOT%{_bindir}/awcc-lux-dashboard.py

# Shell helpers & symlinks
install -m 0755 bin/awcc-mode $RPM_BUILD_ROOT%{_bindir}/awcc-mode
install -m 0755 bin/awcc-track $RPM_BUILD_ROOT%{_bindir}/awcc-track
install -m 0755 bin/awcc-monitor $RPM_BUILD_ROOT%{_bindir}/awcc-monitor
install -m 0755 bin/awcc-gameshift-toggle.sh $RPM_BUILD_ROOT%{_bindir}/awcc-gameshift-toggle
ln -sf %{_bindir}/awcc-gameshift-toggle $RPM_BUILD_ROOT%{_bindir}/gmode

for m in battery quiet balance mid high ultra; do
    if [ -f "bin/awcc-${m}" ]; then
        install -m 0755 "bin/awcc-${m}" "$RPM_BUILD_ROOT%{_bindir}/awcc-${m}"
    fi
done

# Systemd Unit
install -d $RPM_BUILD_ROOT%{_unitdir}
install -m 0644 systemd/awccd.service $RPM_BUILD_ROOT%{_unitdir}/awccd.service

# Desktop Entry & Icon
install -d $RPM_BUILD_ROOT%{_datadir}/applications
install -m 0644 packaging/awcc-gui.desktop $RPM_BUILD_ROOT%{_datadir}/applications/awcc-gui.desktop

install -d $RPM_BUILD_ROOT%{_datadir}/icons/hicolor/scalable/apps
install -m 0644 packaging/icons/awcc-gui.svg $RPM_BUILD_ROOT%{_datadir}/icons/hicolor/scalable/apps/awcc-gui.svg

# PolicyKit Policy
install -d $RPM_BUILD_ROOT%{_datadir}/polkit-1/actions
install -m 0644 packaging/io.github.monkey0_9.awcc.policy $RPM_BUILD_ROOT%{_datadir}/polkit-1/actions/io.github.monkey0_9.awcc.policy

%check
desktop-file-validate $RPM_BUILD_ROOT%{_datadir}/applications/awcc-gui.desktop || true

%post
%systemd_post awccd.service

%preun
%systemd_preun awccd.service

%postun
%systemd_postun_with_restart awccd.service

%files
%{_bindir}/awccctl
%{_bindir}/awccd
%{_bindir}/awcc-gui
%{_bindir}/awcc-lux-dashboard.py
%{_bindir}/awcc-mode
%{_bindir}/awcc-gameshift-toggle
%{_bindir}/gmode
%{_bindir}/awcc-*
%{_unitdir}/awccd.service
%{_datadir}/applications/awcc-gui.desktop
%{_datadir}/icons/hicolor/scalable/apps/awcc-gui.svg
%{_datadir}/polkit-1/actions/io.github.monkey0_9.awcc.policy

%changelog
* Mon Oct 26 2026 AWCC Linux Team <support@awcc-linux.org> - 0.3.0-1
- Universal multi-vendor architecture (Dell, ASUS, Lenovo, HP, Framework, Desktop)
- Dynamic capabilities discovery and verification tiers
- Modern GTK4 / Libadwaita desktop GUI
- Responsive cybernetic terminal dashboard with auto-scaling
- Autonomous thermal daemon with hysteresis damping
