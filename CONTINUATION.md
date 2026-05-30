# Continuation Notes

Last updated: 2026-05-29

## Current State

KAMUSIC is compiling and the current snap build is at `0.1.11`.

Current identifiers:

- Visible name: `KAMUSIC`
- Cargo package / binary: `kamusic`
- App ID: `org.kampos.kamusic`
- Snap name: `kamusic`

## What Is Implemented

- GTK4 + Libadwaita main window with the redesigned layout.
- Left navigation with:
  - `Música local`
  - `Música en YouTube`
  - `Radio online`
- Playlist column now opens a folder picker from:
  - the top folder button
  - the `+` button in `PLAYLISTS`
- Local music scan:
  - recursive folder scan
  - supported audio extensions: `mp3`, `flac`, `ogg`, `opus`, `wav`, `m4a`, `aac`
  - background scan so the UI does not block
  - default folder detection now prefers `SNAP_REAL_HOME`, then `HOME`, then `/home/kampos/Música`
- Library model:
  - tracks stored with path, folder, title, album, extension, size, modified time, and cover path
  - search across title, artist, album, folder, and file path
- Local persistence:
  - JSON settings in XDG config
  - SQLite index in XDG data
- Playback:
  - GStreamer `playbin`
  - play, pause, stop, next, previous
  - volume control
- Online playback:
  - YouTube search via Invidious search API
  - YouTube playback now resolves the stream URL through Invidious `/api/v1/videos/:id`
  - radio preset list for typical Spanish stations
- Cover art:
  - local folder art detection (`cover.jpg`, `folder.png`, etc.)
  - app icon and fallback cover use `data/org.kampos.kamusic.svg`
  - YouTube thumbnails and radio favicons are cached locally
- Snap packaging:
  - `snap/snapcraft.yaml`
  - `command-chain/desktop-launch`
  - compiled GSettings schemas staged into the snap
  - `dbus` session slot for `org.kampos.kamusic`

## Important Files

- App entry: [src/main.rs](/home/kampos/Desarrollo_Software/KAMUSIC/src/main.rs)
- App bootstrap: [src/app.rs](/home/kampos/Desarrollo_Software/KAMUSIC/src/app.rs)
- Main UI: [src/ui/window.rs](/home/kampos/Desarrollo_Software/KAMUSIC/src/ui/window.rs)
- Audio backend: [src/audio/player.rs](/home/kampos/Desarrollo_Software/KAMUSIC/src/audio/player.rs)
- GStreamer backend: [src/audio/gst_backend.rs](/home/kampos/Desarrollo_Software/KAMUSIC/src/audio/gst_backend.rs)
- MPRIS bridge: [src/mpris.rs](/home/kampos/Desarrollo_Software/KAMUSIC/src/mpris.rs)
- Scanner: [src/library/scanner.rs](/home/kampos/Desarrollo_Software/KAMUSIC/src/library/scanner.rs)
- Metadata helpers: [src/library/metadata.rs](/home/kampos/Desarrollo_Software/KAMUSIC/src/library/metadata.rs)
- Cover helpers: [src/library/cover.rs](/home/kampos/Desarrollo_Software/KAMUSIC/src/library/cover.rs)
- Online backend: [src/library/online.rs](/home/kampos/Desarrollo_Software/KAMUSIC/src/library/online.rs)
- Storage: [src/library/database.rs](/home/kampos/Desarrollo_Software/KAMUSIC/src/library/database.rs)
- Settings paths: [src/util/paths.rs](/home/kampos/Desarrollo_Software/KAMUSIC/src/util/paths.rs)
- Toast escaping: [src/util/errors.rs](/home/kampos/Desarrollo_Software/KAMUSIC/src/util/errors.rs)
- Snap config: [snap/snapcraft.yaml](/home/kampos/Desarrollo_Software/KAMUSIC/snap/snapcraft.yaml)
- Snap launcher: [snap/command-chain/desktop-launch](/home/kampos/Desarrollo_Software/KAMUSIC/snap/command-chain/desktop-launch)

## Verified Commands

- `CARGO_HOME=/tmp/cargo cargo check`
- `CARGO_HOME=/tmp/cargo cargo build --release`
- `glib-compile-schemas prime/usr/share/glib-2.0/schemas`
- `snap pack prime /tmp --filename=kamusic_0.1.11_amd64.snap`
- `snap info /tmp/kamusic_0.1.11_amd64.snap`

## Runtime Notes

- The installed snap revision in the user machine can still lag behind the freshly built artifact.
- Current snap artifacts present in the repo root:
  - `kamusic_0.1.11_amd64.snap`
  - older revisions from `0.1.0` through `0.1.10`
- In the snap environment, the following warnings were seen repeatedly:
  - `Gdk-WARNING ... ListActivatableNames ... AccessDenied`
  - `GLib-GIO-WARNING ... /proc/self/mountinfo: Permission denied`
- The app is now set up to avoid the earlier GSettings abort by shipping compiled schemas.
- YouTube playback previously failed because `rustube` broke against the current YouTube response; the current code no longer depends on it.

## Known Gaps / Next Work

- We still need a real end-to-end test of YouTube playback inside the current snap revision `0.1.11`.
- The scan/index pipeline still uses a simple full refresh instead of incremental diffs.
- Track duration, progress bar, and seek support are not fully implemented.
- The queue UI is still basic and not exposed as a dedicated panel.
- Embedded cover-art extraction from audio tags is not implemented yet.
- Radio selection works, but the information panel can still be refined further.

## Safe Next Steps

1. Install and run `kamusic_0.1.11_amd64.snap` and verify YouTube playback with a known working video.
2. Confirm the local library still scans `~/Música` automatically inside the snap.
3. If YouTube still fails, inspect the exact `stream_url` returned by Invidious and whether GStreamer accepts it.
4. Add seek/progress updates from GStreamer bus and expose the timeline in the player bar.
5. Improve metadata extraction and embedded cover-art support.
