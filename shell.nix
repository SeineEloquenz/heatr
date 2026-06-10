{
  system,
  nixpkgs,
  rust-overlay,
}:

let
  buildToolsVersion = "37.0.0";
  ndkVersion = "29.0.14206865";

  buildToolsVersions = [ buildToolsVersion ];
  platformVersions = [ "37" ];
  ndkVersions = [ ndkVersion ];

  pkgs = import nixpkgs {
    inherit system;

    overlays = [ rust-overlay.overlays.default ];

    config.allowUnfree = true;
    config.android_sdk.accept_license = true;

  };

  jdk = pkgs.jdk25;

  androidSdk = pkgs.androidenv.composeAndroidPackages {
    inherit buildToolsVersions platformVersions ndkVersions;
    includeNDK = true;
    includeEmulator = true;
    includeSystemImages = true;
    systemImageTypes = [ "google_apis_playstore" ];
    abiVersions = [ "x86_64" ];
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
      jdk
      pkgs.cargo-ndk
      pkgs.gradle
      androidSdk.androidsdk

      # GTK4/libadwaita client
      pkgs.pkg-config
      pkgs.glib
      pkgs.gtk4
      pkgs.libadwaita
    ];

    env = {
      ANDROID_HOME = "${androidSdk.androidsdk}/libexec/android-sdk";

      ANDROID_NDK_ROOT = "${androidSdk.androidsdk}/libexec/android-sdk/ndk/${ndkVersion}";

      JAVA_HOME = "${jdk}";

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
