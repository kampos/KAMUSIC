use std::path::Path;
use std::sync::mpsc::Sender;
use std::time::Duration;

#[cfg(feature = "gst")]
use crate::audio::gst_backend::GstBackend;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerEvent {
    EndOfStream,
}

#[cfg(feature = "gst")]
pub struct Player {
    backend: GstBackend,
}

#[cfg(not(feature = "gst"))]
pub struct Player;

impl Player {
    #[cfg(feature = "gst")]
    pub fn new_with_events(event_sender: Sender<PlayerEvent>) -> anyhow::Result<Self> {
        Ok(Self {
            backend: GstBackend::new(Some(event_sender))?,
        })
    }

    #[cfg(not(feature = "gst"))]
    pub fn new_with_events(_event_sender: Sender<PlayerEvent>) -> anyhow::Result<Self> {
        Ok(Self)
    }

    #[cfg(feature = "gst")]
    pub fn play_file(&self, path: &Path) -> anyhow::Result<()> {
        self.backend.stop()?;
        self.backend.set_file(path)?;
        self.backend.play()
    }

    #[cfg(feature = "gst")]
    pub fn play_uri(&self, uri: &str) -> anyhow::Result<()> {
        self.backend.stop()?;
        self.backend.set_uri(uri)?;
        self.backend.play()
    }

    #[cfg(not(feature = "gst"))]
    pub fn play_uri(&self, _uri: &str) -> anyhow::Result<()> {
        anyhow::bail!("compilado sin soporte GStreamer")
    }

    #[cfg(not(feature = "gst"))]
    pub fn play_file(&self, _path: &Path) -> anyhow::Result<()> {
        anyhow::bail!("compilado sin soporte GStreamer")
    }

    #[cfg(feature = "gst")]
    pub fn play(&self) -> anyhow::Result<()> {
        self.backend.play()
    }

    #[cfg(not(feature = "gst"))]
    pub fn play(&self) -> anyhow::Result<()> {
        anyhow::bail!("compilado sin soporte GStreamer")
    }

    #[cfg(feature = "gst")]
    pub fn pause(&self) -> anyhow::Result<()> {
        self.backend.pause()
    }

    #[cfg(not(feature = "gst"))]
    pub fn pause(&self) -> anyhow::Result<()> {
        anyhow::bail!("compilado sin soporte GStreamer")
    }

    #[cfg(feature = "gst")]
    pub fn stop(&self) -> anyhow::Result<()> {
        self.backend.stop()
    }

    #[cfg(not(feature = "gst"))]
    pub fn stop(&self) -> anyhow::Result<()> {
        Ok(())
    }

    #[cfg(feature = "gst")]
    pub fn set_volume(&self, volume: f64) {
        self.backend.set_volume(volume);
    }

    #[cfg(not(feature = "gst"))]
    pub fn set_volume(&self, _volume: f64) {}

    #[cfg(feature = "gst")]
    pub fn position(&self) -> Option<Duration> {
        self.backend.position()
    }

    #[cfg(not(feature = "gst"))]
    pub fn position(&self) -> Option<Duration> {
        None
    }

    #[cfg(feature = "gst")]
    pub fn duration(&self) -> Option<Duration> {
        self.backend.duration()
    }

    #[cfg(not(feature = "gst"))]
    pub fn duration(&self) -> Option<Duration> {
        None
    }

    #[cfg(feature = "gst")]
    pub fn seek(&self, position: Duration) -> anyhow::Result<()> {
        self.backend.seek(position)
    }

    #[cfg(not(feature = "gst"))]
    pub fn seek(&self, _position: Duration) -> anyhow::Result<()> {
        anyhow::bail!("compilado sin soporte GStreamer")
    }

    #[cfg(feature = "gst")]
    pub fn video_paintable(&self) -> Option<gtk::gdk::Paintable> {
        self.backend.video_paintable()
    }

    #[cfg(not(feature = "gst"))]
    pub fn video_paintable(&self) -> Option<gtk::gdk::Paintable> {
        None
    }
}
