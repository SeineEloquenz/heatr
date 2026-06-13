//! Loads the application's GSettings, tolerating an uninstalled schema.
//!
//! `gio::Settings::new` aborts the process if its schema isn't installed, so
//! we look the schema up explicitly and fall back gracefully:
//!   1. the installed schema (found via `XDG_DATA_DIRS`, as set by
//!      `wrapGAppsHook4` for the packaged app),
//!   2. the copy `build.rs` compiled into `OUT_DIR` (for `cargo run`),
//!   3. nothing — the app still runs, just without persistence.

use std::path::Path;

use gtk::gio;

use crate::APP_ID;

/// Returns the app settings, or `None` if the schema can't be found.
pub fn load() -> Option<gio::Settings> {
    // 1. Installed schema.
    if let Some(source) = gio::SettingsSchemaSource::default()
        && source.lookup(APP_ID, true).is_some()
    {
        return Some(gio::Settings::new(APP_ID));
    }

    // 2. Dev fallback: schema compiled into OUT_DIR by build.rs.
    let dev_dir = Path::new(env!("OUT_DIR"));
    if let Ok(source) = gio::SettingsSchemaSource::from_directory(
        dev_dir,
        gio::SettingsSchemaSource::default().as_ref(),
        false,
    ) && let Some(schema) = source.lookup(APP_ID, true)
    {
        return Some(gio::Settings::new_full(
            &schema,
            None::<&gio::SettingsBackend>,
            None,
        ));
    }

    // 3. No schema: run without persistence.
    tracing::warn!("GSettings schema {APP_ID} not found; preferences won't persist");
    None
}
