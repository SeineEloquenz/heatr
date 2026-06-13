# NixOS module exposing programs.heatr-cli and programs.heatr-gtk.
#
# Enabling either installs the matching package and the udev rules needed to
# access the device without root.
{ self }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cliCfg = config.programs.heatr-cli;
  gtkCfg = config.programs.heatr-gtk;
  heatrPkgs = self.packages.${pkgs.stdenv.hostPlatform.system};
in
{
  options.programs.heatr-cli.enable =
    lib.mkEnableOption "the heatr command-line client for USB insect bite healers";

  options.programs.heatr-gtk.enable =
    lib.mkEnableOption "the heatr GTK desktop client for USB insect bite healers";

  config = lib.mkIf (cliCfg.enable || gtkCfg.enable) {
    environment.systemPackages =
      lib.optional cliCfg.enable heatrPkgs.cli
      ++ lib.optional gtkCfg.enable heatrPkgs.gtk;

    # Grant non-root access to the bite-healer USB devices.
    services.udev.packages = [ heatrPkgs.udev-rules ];
  };
}
