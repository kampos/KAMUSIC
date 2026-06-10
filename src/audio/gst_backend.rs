use std::path::Path;
use std::sync::mpsc::Sender;
use std::time::Duration;

use gst::prelude::*;
use gstreamer as gst;
use gtk::gdk;
use gtk::glib;

use crate::audio::player::PlayerEvent;

pub struct GstBackend {
    playbin: gst::Element,
    audio_sink: Option<gst::Element>,
    video_sink: Option<gst::Element>,
    _bus_watch: gst::bus::BusWatchGuard,
}

impl GstBackend {
    pub fn new(event_sender: Option<Sender<PlayerEvent>>) -> anyhow::Result<Self> {
        let playbin = gst::ElementFactory::make("playbin").build()?;

        // Intentar usar autoaudiosink que es el más compatible (especialmente con Pipewire)
        let audio_sink = gst::ElementFactory::make("autoaudiosink").build().ok();
        if let Some(audio_sink) = &audio_sink {
            playbin.set_property("audio-sink", audio_sink);
        }

        let video_sink = gst::ElementFactory::make("gtk4paintablesink").build().ok();
        if let Some(video_sink) = &video_sink {
            playbin.set_property("video-sink", video_sink);
        } else if let Ok(video_sink) = gst::ElementFactory::make("fakesink").build() {
            playbin.set_property("video-sink", &video_sink);
        }

        // Configurar el bus para capturar errores asíncronos
        let bus = playbin.bus().expect("El playbin debería tener un bus");
        let bus_watch = bus
            .add_watch(move |_, msg| {
                use gst::MessageView;
                match msg.view() {
                    MessageView::Error(err) => {
                        eprintln!(
                            "GStreamer Error from {:?}: {} ({:?})",
                            msg.src().map(|s| s.path_string()),
                            err.error(),
                            err.debug()
                        );
                    }
                    MessageView::Warning(warn) => {
                        eprintln!(
                            "GStreamer Warning from {:?}: {} ({:?})",
                            msg.src().map(|s| s.path_string()),
                            warn.error(),
                            warn.debug()
                        );
                    }
                    MessageView::Eos(_) => {
                        println!("GStreamer: Fin de la reproducción (EOS)");
                        if let Some(sender) = &event_sender {
                            let _ = sender.send(PlayerEvent::EndOfStream);
                        }
                    }
                    _ => (),
                }
                glib::ControlFlow::Continue
            })
            .expect("No se pudo añadir el watch al bus");

        // Establecer User-Agent para streams online (radio)
        playbin.connect("source-setup", false, |args| {
            let source = args[1].get::<gst::Element>().unwrap();
            if source.has_property("user-agent") {
                let _ = source.set_property("user-agent", "KAMUSIC/0.1");
            }
            None
        });

        Ok(Self {
            playbin,
            audio_sink,
            video_sink,
            _bus_watch: bus_watch,
        })
    }

    pub fn set_file(&self, path: &Path) -> anyhow::Result<()> {
        let uri = url::Url::from_file_path(path)
            .map_err(|_| anyhow::anyhow!("No se pudo convertir la ruta a URI"))?;
        self.playbin.set_property("uri", uri.as_str());
        Ok(())
    }

    pub fn set_uri(&self, uri: &str) -> anyhow::Result<()> {
        eprintln!("GStreamer: reproduciendo URI {uri}");
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

    pub fn position(&self) -> Option<Duration> {
        self.playbin
            .query_position::<gst::ClockTime>()
            .map(Into::into)
    }

    pub fn duration(&self) -> Option<Duration> {
        self.playbin
            .query_duration::<gst::ClockTime>()
            .map(Into::into)
    }

    pub fn seek(&self, position: Duration) -> anyhow::Result<()> {
        self.playbin.seek_simple(
            gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
            gst::ClockTime::from_nseconds(position.as_nanos().min(u128::from(u64::MAX)) as u64),
        )?;
        Ok(())
    }

    pub fn set_volume(&self, volume: f64) {
        self.playbin.set_property("volume", volume.clamp(0.0, 1.5));
    }

    pub fn video_paintable(&self) -> Option<gdk::Paintable> {
        self.video_sink
            .as_ref()
            .and_then(|sink| sink.property::<Option<gdk::Paintable>>("paintable"))
    }
}
