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
          default = pkgs.callPackage ./default.nix { };
          udev-rules = pkgs.callPackage ./udev { };
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
        let
          buildToolsVersion = "36.0.0";
          ndkVersion = "27.3.13750724";
          buildToolsVersions = [ buildToolsVersion ];
          platformVersions = [ "36" ];
          ndkVersions = [ ndkVersion ];

          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
            config.allowUnfree = true;
            config.android_sdk.accept_license = true;
          };

          androidSdk = pkgs.androidenv.composeAndroidPackages {
            inherit buildToolsVersions platformVersions ndkVersions;
            includeNDK = true;
          };

          rustToolchain = pkgs.rust-bin.stable.latest.default.override {
            extensions = [ "rust-src" ];
            targets = [
              "aarch64-linux-android"
              "armv7-linux-androideabi"
              "x86_64-linux-android"
            ];
          };
        in
        {
          default = pkgs.mkShell {
            packages = [
              rustToolchain
              pkgs.cargo-ndk
              pkgs.jdk21
              pkgs.gradle
              androidSdk.androidsdk
            ];

            env = {
              ANDROID_HOME = "${androidSdk.androidsdk}/libexec/android-sdk";
              ANDROID_NDK_ROOT = "${androidSdk.androidsdk}/libexec/android-sdk/ndk/${ndkVersion}";
              JAVA_HOME = "${pkgs.jdk17}";
              # aapt2 bundled in the AGP Maven artifact is a generic-Linux binary
              # that NixOS cannot run. Override it with the Nix-patched copy.
              GRADLE_OPTS = "-Dorg.gradle.project.android.aapt2FromMavenOverride=${androidSdk.androidsdk}/libexec/android-sdk/build-tools/${buildToolsVersion}/aapt2";
            };

            shellHook = ''
                            cat > "$PWD/android/local.properties" <<EOF
              sdk.dir=${androidSdk.androidsdk}/libexec/android-sdk
              EOF
            '';
          };
        }
      );

      nixosModules.default =
        { pkgs, ... }:
        {
          services.udev.packages = [ self.packages.${pkgs.system}.udev-rules ];
        };
    };
}
