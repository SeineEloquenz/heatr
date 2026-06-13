{
  lib,
  rustPlatform,
  clippy,
  rustfmt,
  cmake,
  pkg-config,
  wrapGAppsHook4,
  glib,
  gtk4,
  libadwaita,
  ...
}:

rustPlatform.buildRustPackage {

  pname = "heatr-gtk";
  version = "0.0.1";

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../crates
    ];
  };

  nativeBuildInputs = [
    cmake
    pkg-config
    rustfmt
    clippy
    wrapGAppsHook4
  ];

  buildInputs = [
    glib
    gtk4
    libadwaita
  ];

  RUST_SRC_PATH = "${rustPlatform.rustLibSrc}";

  cargoBuildFlags = [
    "-p"
    "heatr-gtk"
  ];

  cargoTestFlags = [
    "-p"
    "heatr"
    "-p"
    "heatr-gtk"
  ];

  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  postInstall = ''
    install -Dm644 crates/heatr-gtk/data/nz.eloque.heatr.desktop \
      $out/share/applications/nz.eloque.heatr.desktop
    install -Dm644 crates/heatr-gtk/data/nz.eloque.heatr.metainfo.xml \
      $out/share/metainfo/nz.eloque.heatr.metainfo.xml
    install -Dm644 crates/heatr-gtk/data/icons/hicolor/scalable/apps/nz.eloque.heatr.svg \
      $out/share/icons/hicolor/scalable/apps/nz.eloque.heatr.svg
  '';

  meta = {
    description = "GTK4 client for heat-based USB insect bite healers";
    license = lib.licenses.gpl3Plus;
    mainProgram = "heatr-gtk";
  };
}
