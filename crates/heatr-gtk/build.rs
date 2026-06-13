//! Compiles the GSettings schema into `OUT_DIR` so that `cargo run` (an
//! uninstalled build) can find it without the schema being installed
//! system-wide. Installed builds use the schema from their data dir instead;
//! see `src/settings.rs`.

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=data/nz.eloque.heatr.gschema.xml");

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let status = Command::new("glib-compile-schemas")
        .args(["--targetdir", &out_dir, "data"])
        .status()
        .expect("failed to run glib-compile-schemas (is glib installed?)");
    assert!(status.success(), "glib-compile-schemas failed");
}
