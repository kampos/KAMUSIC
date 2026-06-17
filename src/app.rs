use adw::prelude::*;
use gtk::gio;

use crate::ui::window::MainWindow;

pub const APP_ID: &str = "org.kampos.kamusic";

pub fn run() {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    app.connect_activate(|app| {
        if let Some(window) = app.active_window() {
            window.present();
            return;
        }
        let window = MainWindow::new(app, Vec::new());
        window.present();
    });

    app.connect_open(|app, files, _hint| {
        let initial_files = files
            .iter()
            .filter_map(|file| file.path())
            .collect::<Vec<_>>();

        if let Some(window) = app.active_window() {
            window.present();
            if let Some(main_window) = window.downcast_ref::<adw::ApplicationWindow>() {
                MainWindow::open_files_in_window(main_window, initial_files);
            }
        } else {
            let window = MainWindow::new(app, initial_files);
            window.present();
        }
    });

    app.run();
}
