use std::cell::RefCell;
use std::rc::Rc;

use gtk::glib;
use mpris_server::{Metadata, PlaybackStatus, Player as MprisPlayer, TrackId};

use crate::library::models::Track;

#[derive(Clone, Default)]
pub struct MprisControls {
    player: Rc<RefCell<Option<Rc<MprisPlayer>>>>,
}

impl MprisControls {
    pub fn setup(
        &self,
        on_play: impl Fn() + 'static,
        on_pause: impl Fn() + 'static,
        on_stop: impl Fn() + 'static,
        on_next: impl Fn() + 'static,
        on_previous: impl Fn() + 'static,
    ) {
        let slot = Rc::clone(&self.player);
        glib::MainContext::default().spawn_local(async move {
            let on_play = Rc::new(on_play);
            let Ok(player) = MprisPlayer::builder("org.mpris.MediaPlayer2.kamusic")
                .identity("KAMUSIC")
                .desktop_entry("org.kampos.kamusic")
                .can_play(true)
                .can_pause(true)
                .can_go_next(true)
                .can_go_previous(true)
                .can_control(true)
                .build()
                .await
            else {
                return;
            };

            {
                let on_play = Rc::clone(&on_play);
                player.connect_play(move |_| on_play());
            }
            player.connect_play_pause(move |_| on_play());
            player.connect_pause(move |_| on_pause());
            player.connect_stop(move |_| on_stop());
            player.connect_next(move |_| on_next());
            player.connect_previous(move |_| on_previous());

            let player = Rc::new(player);
            slot.replace(Some(Rc::clone(&player)));
            player.run().await;
        });
    }

    pub fn set_playing(&self, track: &Track) {
        self.update_status(PlaybackStatus::Playing);
        self.update_metadata(track);
    }

    pub fn set_paused(&self) {
        self.update_status(PlaybackStatus::Paused);
    }

    pub fn set_stopped(&self) {
        self.update_status(PlaybackStatus::Stopped);
    }

    fn update_status(&self, status: PlaybackStatus) {
        if let Some(player) = self.player.borrow().as_ref().cloned() {
            glib::MainContext::default().spawn_local(async move {
                let _ = player.set_playback_status(status).await;
            });
        }
    }

    fn update_metadata(&self, track: &Track) {
        let Some(player) = self.player.borrow().as_ref().cloned() else {
            return;
        };

        let mut builder = Metadata::builder()
            .trackid(track_id_for(track))
            .title(track.title.clone());

        if let Some(album) = &track.album {
            builder = builder.album(album.clone());
        }
        if let Some(artist) = &track.artist {
            builder = builder.artist([artist.clone()]);
        }
        if let Ok(uri) = url::Url::from_file_path(&track.path) {
            builder = builder.url(uri.to_string());
        }
        if let Some(cover_path) = &track.cover_path {
            if let Ok(uri) = url::Url::from_file_path(cover_path) {
                builder = builder.art_url(uri.to_string());
            }
        }

        let metadata = builder.build();
        glib::MainContext::default().spawn_local(async move {
            let _ = player.set_metadata(metadata).await;
        });
    }
}

fn track_id_for(track: &Track) -> TrackId {
    let mut id = String::from("/org/kampos/KAMUSIC/track/");
    for byte in track.path.to_string_lossy().bytes() {
        use std::fmt::Write;
        let _ = write!(id, "{byte:02x}");
    }
    TrackId::try_from(id).unwrap_or(TrackId::NO_TRACK)
}
