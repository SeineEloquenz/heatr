{ stdenvNoCC, ... }:

stdenvNoCC.mkDerivation {
  pname = "heatr-udev-rules";
  version = "0.2.0";

  src = ./.;

  dontBuild = true;

  installPhase = ''
    mkdir -p $out/lib/udev/rules.d
    cp *.rules $out/lib/udev/rules.d/
  '';
}
