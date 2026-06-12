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
}
