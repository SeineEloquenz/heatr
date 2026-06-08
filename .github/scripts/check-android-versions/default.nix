{ writeShellApplication
, python3 }:

writeShellApplication {
  name = "check-android-versions";
  runtimeInputs = [ python3 ];

  text = ''
    exec python3 ${./check_android_versions.py} "$@"
  '';
}
