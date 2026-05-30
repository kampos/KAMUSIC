use gtk::glib;

pub fn show_toast(overlay: &adw::ToastOverlay, message: impl Into<String>) {
    let message = message.into();
    overlay.add_toast(adw::Toast::new(&glib::markup_escape_text(&message)));
}
