{
  lib,
  rustPlatform,
  clippy,
  rustfmt,
  cmake,
  pkg-config,
  ...
}:

rustPlatform.buildRustPackage {

  pname = "heatr";
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
  ];

  RUST_SRC_PATH = "${rustPlatform.rustLibSrc}";

  cargoBuildFlags = [
    "-p"
    "heatr-cli"
  ];

  cargoTestFlags = [
    "-p"
    "heatr"
    "-p"
    "heatr-cli"
  ];

  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  meta = {
    description = "CLI for heat-based USB insect bite healers";
    license = lib.licenses.gpl3Plus;
    mainProgram = "heatr";
  };
}
