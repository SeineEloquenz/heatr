//! Main application window

use std::cell::Cell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use heatr::{Duration, Generation, HeatingPhase, HeatingStatus, Preferences, SkinSensitivity};
use tracing::error;

use crate::device::{self, DeviceView};
use crate::session::{self, Outcome};

const PAGE_NO_DEVICE: &str = "no-device";
const PAGE_READY: &str = "ready";
const PAGE_RUNNING: &str = "running";

/// Peak raw ADC temperature reading, used to scale the progress bar.
const TEMPERATURE_MAX: f64 = 225.0;

/// Builds the main window and triggers an initial device scan.
pub fn build(app: &adw::Application) -> adw::ApplicationWindow {
    let ui = Rc::new(Ui::new(app));

    let refresh = gio::ActionEntry::builder("refresh")
        .activate({
            let ui = Rc::clone(&ui);
            move |_: &adw::ApplicationWindow, _, _| ui.refresh()
        })
        .build();
    ui.window.add_action_entries([refresh]);

    ui.start_button.connect_clicked({
        let ui = Rc::clone(&ui);
        move |_| ui.start_session()
    });
    ui.stop_button.connect_clicked({
        let ui = Rc::clone(&ui);
        move |_| ui.request_stop()
    });

    ui.refresh();
    ui.window.clone()
}

/// All widgets that change with application state.
struct Ui {
    window: adw::ApplicationWindow,
    toast_overlay: adw::ToastOverlay,
    banner: adw::Banner,
    stack: gtk::Stack,
    device_row: adw::ActionRow,
    duration_row: adw::ComboRow,
    user_row: adw::ComboRow,
    sensitive_row: adw::SwitchRow,
    start_button: gtk::Button,
    phase_label: gtk::Label,
    temperature_bar: gtk::LevelBar,
    stop_button: gtk::Button,
    /// True while a session task is in flight (re-entrancy guard).
    running: Cell<bool>,
    /// Shared with the running session task; set to request cancellation.
    stop: Rc<Cell<bool>>,
}

impl Ui {
    fn new(app: &adw::Application) -> Self {
        // -- Header bar --------------------------------------------------
        let header = adw::HeaderBar::new();

        let refresh_button = gtk::Button::from_icon_name("view-refresh-symbolic");
        refresh_button.set_tooltip_text(Some("Scan for bite healers"));
        refresh_button.set_action_name(Some("win.refresh"));
        header.pack_start(&refresh_button);

        let menu = gio::Menu::new();
        menu.append(Some("_About Heatr"), Some("app.about"));
        menu.append(Some("_Quit"), Some("app.quit"));
        let menu_button = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .menu_model(&menu)
            .primary(true)
            .tooltip_text("Main Menu")
            .build();
        header.pack_end(&menu_button);

        // -- Safety banner (revealed while a device is in use) -----------
        let banner = adw::Banner::new("Not a certified medical product — not safe for use on skin");

        // -- Pages --------------------------------------------------------
        let no_device_page = build_no_device_page();

        let device_row = adw::ActionRow::builder().title("No device").build();
        let duration_row = adw::ComboRow::builder()
            .title("Duration")
            .model(&gtk::StringList::new(&["Short", "Medium", "Long"]))
            .build();
        let user_row = adw::ComboRow::builder()
            .title("User")
            .model(&gtk::StringList::new(&["Child", "Adult"]))
            .build();
        let sensitive_row = adw::SwitchRow::builder()
            .title("Sensitive Skin")
            .active(true)
            .build();
        let start_button = gtk::Button::builder()
            .label("Start")
            .halign(gtk::Align::Center)
            .css_classes(["pill", "suggested-action"])
            .build();
        let ready_page = build_ready_page(
            &device_row,
            &duration_row,
            &user_row,
            &sensitive_row,
            &start_button,
        );

        let phase_label = gtk::Label::builder().css_classes(["title-2"]).build();
        let temperature_bar = gtk::LevelBar::for_interval(0.0, TEMPERATURE_MAX);
        let stop_button = gtk::Button::builder()
            .label("Stop")
            .halign(gtk::Align::Center)
            .css_classes(["pill", "destructive-action"])
            .build();
        let running_page = build_running_page(&phase_label, &temperature_bar, &stop_button);

        // -- Window assembly ----------------------------------------------
        let stack = gtk::Stack::builder()
            .vexpand(true)
            .transition_type(gtk::StackTransitionType::Crossfade)
            .build();
        stack.add_named(&no_device_page, Some(PAGE_NO_DEVICE));
        stack.add_named(&ready_page, Some(PAGE_READY));
        stack.add_named(&running_page, Some(PAGE_RUNNING));

        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.append(&banner);
        content.append(&stack);

        let toast_overlay = adw::ToastOverlay::new();
        toast_overlay.set_child(Some(&content));

        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&toast_overlay));

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Heatr")
            .default_width(420)
            .default_height(640)
            .content(&toolbar)
            .build();

        Self {
            window,
            toast_overlay,
            banner,
            stack,
            device_row,
            duration_row,
            user_row,
            sensitive_row,
            start_button,
            phase_label,
            temperature_bar,
            stop_button,
            running: Cell::new(false),
            stop: Rc::new(Cell::new(false)),
        }
    }

    /// Scans for bite healers and switches to the matching page.
    fn refresh(self: &Rc<Self>) {
        // Don't yank the page out from under a running session.
        if self.running.get() {
            return;
        }
        let ui = Rc::clone(self);
        glib::spawn_future_local(async move {
            match device::discover().await {
                Ok(Some(found)) => ui.show_ready(&found),
                Ok(None) => ui.show_no_device(),
                Err(e) => {
                    error!("Device discovery failed: {e}");
                    ui.toast(&format!("Device discovery failed: {e}"));
                    ui.show_no_device();
                }
            }
        });
    }

    fn show_no_device(&self) {
        self.banner.set_revealed(false);
        self.stack.set_visible_child_name(PAGE_NO_DEVICE);
    }

    fn show_ready(&self, device: &DeviceView) {
        self.device_row.set_title(&device.product);
        let mut subtitle = device.vendor.clone();
        if let Some(serial) = &device.serial {
            subtitle.push_str(&format!(" · S/N {serial}"));
        }
        if !device.supported {
            subtitle.push_str("\nThis model is not supported by heatr");
        }
        self.device_row.set_subtitle(&subtitle);
        self.start_button.set_sensitive(device.supported);

        self.banner.set_revealed(true);
        self.stack.set_visible_child_name(PAGE_READY);
    }

    /// Reads the session preferences from the input rows.
    fn read_preferences(&self) -> Preferences {
        let duration = match self.duration_row.selected() {
            0 => Duration::Short,
            1 => Duration::Medium,
            _ => Duration::Long,
        };
        let generation = match self.user_row.selected() {
            0 => Generation::Child,
            _ => Generation::Adult,
        };
        let skin_sensitivity = if self.sensitive_row.is_active() {
            SkinSensitivity::Sensitive
        } else {
            SkinSensitivity::Regular
        };
        Preferences {
            duration,
            generation,
            skin_sensitivity,
        }
    }

    /// Starts a heating session and switches to the running page.
    fn start_session(self: &Rc<Self>) {
        if self.running.get() {
            return;
        }
        self.running.set(true);
        self.stop.set(false);

        let prefs = self.read_preferences();
        self.show_running();

        let ui = Rc::clone(self);
        let stop = Rc::clone(&self.stop);
        glib::spawn_future_local(async move {
            let outcome = session::run(prefs, stop, |status| ui.update_progress(status)).await;
            ui.finish_session(outcome);
        });
    }

    /// Requests cancellation of the running session.
    fn request_stop(&self) {
        self.stop.set(true);
        self.stop_button.set_sensitive(false);
        self.phase_label.set_label("Stopping…");
    }

    /// Switches to the running page and resets its widgets.
    fn show_running(&self) {
        self.phase_label.set_label("Preheating…");
        self.temperature_bar.set_value(0.0);
        self.stop_button.set_sensitive(true);
        self.banner.set_revealed(true);
        self.stack.set_visible_child_name(PAGE_RUNNING);
    }

    /// Reflects a status poll on the running page.
    fn update_progress(&self, status: &HeatingStatus) {
        let label = match status.phase {
            HeatingPhase::Heating => "Preheating…",
            HeatingPhase::Applying => "Apply to skin",
            HeatingPhase::Done => "Done",
        };
        self.phase_label.set_label(label);
        self.temperature_bar.set_value(status.temperature as f64);
    }

    /// Reports the session outcome and returns to device selection.
    fn finish_session(self: &Rc<Self>, outcome: Outcome) {
        self.running.set(false);
        match outcome {
            Outcome::Completed => self.toast("Session complete"),
            Outcome::Stopped => self.toast("Session stopped"),
            Outcome::Failed(message) => {
                error!("Session failed: {message}");
                self.toast(&message);
            }
        }
        // Re-scan so the page reflects whether the device is still present.
        self.refresh();
    }

    fn toast(&self, message: &str) {
        self.toast_overlay.add_toast(adw::Toast::new(message));
    }
}

fn build_no_device_page() -> gtk::Widget {
    let refresh_button = gtk::Button::builder()
        .label("Refresh")
        .halign(gtk::Align::Center)
        .css_classes(["pill", "suggested-action"])
        .action_name("win.refresh")
        .build();

    adw::StatusPage::builder()
        .icon_name("drive-removable-media-symbolic")
        .title("No Bite Healer Found")
        .description("Plug in a supported USB bite healer and refresh")
        .child(&refresh_button)
        .build()
        .upcast()
}

fn build_ready_page(
    device_row: &adw::ActionRow,
    duration_row: &adw::ComboRow,
    user_row: &adw::ComboRow,
    sensitive_row: &adw::SwitchRow,
    start_button: &gtk::Button,
) -> gtk::Widget {
    let device_group = adw::PreferencesGroup::builder().title("Device").build();
    device_group.add(device_row);

    let session_group = adw::PreferencesGroup::builder().title("Session").build();
    session_group.add(duration_row);
    session_group.add(user_row);
    session_group.add(sensitive_row);

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(24)
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(12)
        .margin_end(12)
        .build();
    content.append(&device_group);
    content.append(&session_group);
    content.append(start_button);

    let clamp = adw::Clamp::builder()
        .maximum_size(420)
        .child(&content)
        .build();

    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&clamp)
        .build()
        .upcast()
}

fn build_running_page(
    phase_label: &gtk::Label,
    temperature_bar: &gtk::LevelBar,
    stop_button: &gtk::Button,
) -> gtk::Widget {
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(24)
        .valign(gtk::Align::Center)
        .margin_start(24)
        .margin_end(24)
        .build();
    content.append(phase_label);
    content.append(temperature_bar);
    content.append(stop_button);

    adw::Clamp::builder()
        .maximum_size(420)
        .child(&content)
        .build()
        .upcast()
}
