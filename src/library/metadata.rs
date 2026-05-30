use std::path::PathBuf;
use std::time::UNIX_EPOCH;

use crate::library::models::Track;

pub fn track_from_path(path: PathBuf) -> Option<Track> {
    let metadata = std::fs::metadata(&path).ok()?;
    let folder = path.parent()?.to_path_buf();
    let title = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Sin titulo")
        .to_string();
    let album = folder
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned);
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_lowercase();
    let modified = metadata
        .modified()
        .ok()
        .and_then(|mtime| mtime.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default();
    let mut track = Track {
        title,
        artist: None,
        album,
        path,
        folder,
        extension,
        cover_path: None,
        size: metadata.len(),
        modified,
    };
    track.cover_path = crate::library::cover::resolve_track_cover(&track);
    Some(track)
}
