{ lib
, rustPlatform
, pkg-config
, openssl
, python3
, wrapGAppsHook4
, gtk4
, libadwaita
, libnotify
}:

rustPlatform.buildRustPackage rec {
  pname = "awcc-linux";
  version = "0.3.0";

  src = ../..;

  cargoLock = {
    lockFile = ../../Cargo.lock;
  };

  nativeBuildInputs = [
    pkg-config
    wrapGAppsHook4
  ];

  buildInputs = [
    openssl
    gtk4
    libadwaita
    libnotify
    (python3.withPackages (ps: with ps; [ pygobject3 ]))
  ];

  postInstall = ''
    install -Dm755 bin/awcc-gui $out/bin/awcc-gui
    install -Dm755 bin/awcc-lux-dashboard.py $out/bin/awcc-lux-dashboard.py
    install -Dm755 bin/awcc-mode $out/bin/awcc-mode
    install -Dm755 bin/awcc-track $out/bin/awcc-track
    install -Dm755 bin/awcc-gameshift-toggle.sh $out/bin/awcc-gameshift-toggle
    ln -s awcc-gameshift-toggle $out/bin/gmode

    install -Dm644 packaging/awcc-gui.desktop $out/share/applications/awcc-gui.desktop
    install -Dm644 packaging/icons/awcc-gui.svg $out/share/icons/hicolor/scalable/apps/awcc-gui.svg
    install -Dm644 packaging/io.github.monkey0_9.awcc.policy $out/share/polkit-1/actions/io.github.monkey0_9.awcc.policy
    install -Dm644 packaging/90-awcc.rules $out/share/polkit-1/rules.d/90-awcc.rules
    install -Dm644 systemd/awccd.service $out/lib/systemd/system/awccd.service
  '';

  meta = with lib; {
    description = "Universal Linux Hardware Performance, Fan Control & Telemetry Suite";
    homepage = "https://github.com/Monkey0-9/Awcc";
    license = with licenses; [ gpl2Plus mit ];
    platforms = platforms.linux;
  };
}
