{
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

  src = ./.;

  nativeBuildInputs = [
    cmake
    pkg-config
    rustfmt
    clippy
  ];

  RUST_SRC_PATH = "${rustPlatform.rustLibSrc}";

  cargoLock = {
    lockFile = ./Cargo.lock;
  };
}
