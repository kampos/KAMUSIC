mod app;
mod audio;
mod config;
mod library;
mod mpris;
mod ui;
mod util;

fn main() -> anyhow::Result<()> {
    unsafe {
        std::env::set_var("GSETTINGS_BACKEND", "memory");
        std::env::set_var("GSK_RENDERER", "cairo");
    }

    #[cfg(feature = "gst")]
    gstreamer::init()?;
    app::run();
    Ok(())
}
