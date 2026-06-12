//! Heatr GTK4/libadwaita client.
//!
//! A GNOME desktop client for the heatr library. All device I/O runs as
//! async tasks on the GLib main loop; no worker threads are needed.

mod window;

use adw::prelude::*;
use gtk::{gio, glib};

const APP_ID: &str = "nz.eloque.heatr";

fn main() -> glib::ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing::Level::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let app = adw::Application::builder().application_id(APP_ID).build();

    app.connect_startup(|app| {
        let quit = gio::ActionEntry::builder("quit")
            .activate(|app: &adw::Application, _, _| app.quit())
            .build();
        let about = gio::ActionEntry::builder("about")
            .activate(|app: &adw::Application, _, _| show_about(app))
            .build();
        app.add_action_entries([quit, about]);

        app.set_accels_for_action("app.quit", &["<Ctrl>q"]);
        app.set_accels_for_action("window.close", &["<Ctrl>w"]);
        app.set_accels_for_action("win.refresh", &["<Ctrl>r", "F5"]);
    });

    app.connect_activate(|app| {
        window::build(app).present();
    });

    app.run()
}

fn show_about(app: &adw::Application) {
    let dialog = adw::AboutDialog::builder()
        .application_name("Heatr")
        .application_icon(APP_ID)
        .version(env!("CARGO_PKG_VERSION"))
        .comments(
            "Client for insect bite healers like heat-it.\n\nNOT A CERTIFIED MEDICAL PRODUCT. WE ARE NOT LIABLE FOR ANY DAMAGE YOU DO TO YOURSELF",
        )
        .build();
    dialog.present(app.active_window().as_ref());
}
