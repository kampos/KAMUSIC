use adw::prelude::*;
use gtk::gio;

use crate::ui::window::MainWindow;

pub const APP_ID: &str = "org.kampos.kamusic";

pub fn run() {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::NON_UNIQUE | gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    app.connect_activate(|app| {
        let window = MainWindow::new(app, Vec::new());
        window.present();
    });

    app.connect_open(|app, files, _hint| {
        let initial_files = files
            .iter()
            .filter_map(|file| file.path())
            .collect::<Vec<_>>();
        let window = MainWindow::new(app, initial_files);
        window.present();
    });

    app.run();
}
