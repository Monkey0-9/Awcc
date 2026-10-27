Name:           awcc-linux
Version:        0.3.0
Release:        0
Summary:        Universal Linux Hardware Performance, Fan Control & Telemetry Suite
License:        GPL-2.0-or-later AND MIT
Group:          System/Hardware
URL:            https://github.com/Monkey0-9/Awcc
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  gcc
BuildRequires:  make
BuildRequires:  update-desktop-files
BuildRequires:  pkgconfig(systemd)

Requires:       python3
Requires:       python3-gobject
Requires:       python3-gobject-Gdk
Requires:       typelib-1_0-Gtk-4_0
Requires:       typelib-1_0-Adw-1
Requires:       libnotify-tools

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
# Binaries
install -d %{buildroot}%{_bindir}
install -m 0755 target/release/awccctl %{buildroot}%{_bindir}/awccctl
install -m 0755 target/release/awccd %{buildroot}%{_bindir}/awccd
install -m 0755 bin/awcc-gui %{buildroot}%{_bindir}/awcc-gui
install -m 0755 bin/awcc-lux-dashboard.py %{buildroot}%{_bindir}/awcc-lux-dashboard.py

# Shell helpers & symlinks
install -m 0755 bin/awcc-mode %{buildroot}%{_bindir}/awcc-mode
install -m 0755 bin/awcc-track %{buildroot}%{_bindir}/awcc-track
install -m 0755 bin/awcc-monitor %{buildroot}%{_bindir}/awcc-monitor
install -m 0755 bin/awcc-gameshift-toggle.sh %{buildroot}%{_bindir}/awcc-gameshift-toggle
ln -sf %{_bindir}/awcc-gameshift-toggle %{buildroot}%{_bindir}/gmode

for m in battery quiet balance balanced mid high performance ultra; do
    if [ -f "bin/awcc-${m}" ]; then
        install -m 0755 "bin/awcc-${m}" "%{buildroot}%{_bindir}/awcc-${m}"
    fi
done

# Systemd Unit
install -d %{buildroot}%{_unitdir}
install -m 0644 systemd/awccd.service %{buildroot}%{_unitdir}/awccd.service

# Desktop Entry & Icon
install -d %{buildroot}%{_datadir}/applications
install -m 0644 packaging/awcc-gui.desktop %{buildroot}%{_datadir}/applications/awcc-gui.desktop

install -d %{buildroot}%{_datadir}/icons/hicolor/scalable/apps
install -m 0644 packaging/icons/awcc-gui.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/awcc-gui.svg

# PolicyKit Policy & Rules
install -d %{buildroot}%{_datadir}/polkit-1/actions
install -m 0644 packaging/io.github.monkey0_9.awcc.policy %{buildroot}%{_datadir}/polkit-1/actions/io.github.monkey0_9.awcc.policy
install -d %{buildroot}%{_datadir}/polkit-1/rules.d
install -m 0644 packaging/90-awcc.rules %{buildroot}%{_datadir}/polkit-1/rules.d/90-awcc.rules

# Shell Completions
install -d %{buildroot}%{_datadir}/bash-completion/completions
install -m 0644 completions/bash/awccctl %{buildroot}%{_datadir}/bash-completion/completions/awccctl
install -d %{buildroot}%{_datadir}/zsh/site-functions
install -m 0644 completions/zsh/_awccctl %{buildroot}%{_datadir}/zsh/site-functions/_awccctl
install -d %{buildroot}%{_datadir}/fish/vendor_completions.d
install -m 0644 completions/fish/awccctl.fish %{buildroot}%{_datadir}/fish/vendor_completions.d/awccctl.fish

# Man Pages
install -d %{buildroot}%{_mandir}/man1
install -m 0644 man/man1/*.1 %{buildroot}%{_mandir}/man1/

%pre
%service_add_pre awccd.service

%post
%service_add_post awccd.service

%preun
%service_del_preun awccd.service

%postun
%service_del_postun awccd.service

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
%{_datadir}/polkit-1/rules.d/90-awcc.rules
%{_datadir}/bash-completion/completions/awccctl
%{_datadir}/zsh/site-functions/_awccctl
%{_datadir}/fish/vendor_completions.d/awccctl.fish
%{_mandir}/man1/*.1*

%changelog
* Mon Oct 26 2026 AWCC Linux Team <support@awcc-linux.org>
- Initial openSUSE package for version 0.3.0
