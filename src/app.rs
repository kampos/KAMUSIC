use adw::prelude::*;

use crate::ui::window::MainWindow;

pub const APP_ID: &str = "org.kampos.kamusic";

pub fn run() {
    let app = adw::Application::builder().application_id(APP_ID).build();

    app.connect_activate(|app| {
        let window = MainWindow::new(app);
        window.present();
    });

    app.run();
}
