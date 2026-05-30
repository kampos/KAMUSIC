use adw::prelude::*;
use gtk::gio;

use crate::ui::window::MainWindow;

pub const APP_ID: &str = "org.kampos.kamusic";

pub fn run() {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_activate(|app| {
        let window = MainWindow::new(app);
        window.present();
    });

    app.run();
}
