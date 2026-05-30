use std::path::Path;

use gst::prelude::*;
use gstreamer as gst;
use gtk::gdk;

pub struct GstBackend {
    playbin: gst::Element,
    video_sink: gst::Element,
}

impl GstBackend {
    pub fn new() -> anyhow::Result<Self> {
        let playbin = gst::ElementFactory::make("playbin").build()?;
        let video_sink = gst::ElementFactory::make("gtk4paintablesink").build()?;
        playbin.set_property("video-sink", &video_sink);
        Ok(Self {
            playbin,
            video_sink,
        })
    }

    pub fn set_file(&self, path: &Path) -> anyhow::Result<()> {
        let uri = url::Url::from_file_path(path)
            .map_err(|_| anyhow::anyhow!("No se pudo convertir la ruta a URI"))?;
        self.playbin.set_property("uri", uri.as_str());
        Ok(())
    }

    pub fn set_uri(&self, uri: &str) -> anyhow::Result<()> {
        self.playbin.set_property("uri", uri);
        Ok(())
    }

    pub fn play(&self) -> anyhow::Result<()> {
        self.playbin.set_state(gst::State::Playing)?;
        Ok(())
    }

    pub fn pause(&self) -> anyhow::Result<()> {
        self.playbin.set_state(gst::State::Paused)?;
        Ok(())
    }

    pub fn stop(&self) -> anyhow::Result<()> {
        self.playbin.set_state(gst::State::Null)?;
        Ok(())
    }

    pub fn set_volume(&self, volume: f64) {
        self.playbin.set_property("volume", volume.clamp(0.0, 1.5));
    }

    pub fn video_paintable(&self) -> Option<gdk::Paintable> {
        self.video_sink
            .property::<Option<gdk::Paintable>>("paintable")
    }
}
