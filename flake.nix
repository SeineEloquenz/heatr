{
  description = "heatr";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      forAllSystems =
        f:
        builtins.listToAttrs (
          map (system: {
            name = system;
            value = f system;
          }) systems
        );
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          default = pkgs.callPackage ./nix/cli.nix { };
          cli = pkgs.callPackage ./nix/cli.nix { };
          gtk = pkgs.callPackage ./nix/gtk.nix { };
          udev-rules = pkgs.callPackage ./nix/udev { };
        }
      );

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/heatr";
        };
      });

      devShells = forAllSystems (
        system:
        import ./shell.nix {
          inherit system nixpkgs rust-overlay;
        }
      );

      nixosModules.default =
        { pkgs, ... }:
        {
          services.udev.packages = [ self.packages.${pkgs.system}.udev-rules ];
        };
    };
}
