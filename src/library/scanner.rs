use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::library::metadata::track_from_path;
use crate::library::models::Library;

const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "ogg", "opus", "wav", "m4a", "aac"];

pub fn default_music_dir() -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(home) = std::env::var_os("SNAP_REAL_HOME")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
    {
        candidates.push(home.join("Música"));
        candidates.push(home.join("Musica"));
        candidates.push(home.join("Music"));
        candidates.push(home.join("music"));
    }

    candidates.push(PathBuf::from("/home/kampos/Música"));

    candidates.into_iter().find(|path| path.is_dir())
}

pub fn scan(root: PathBuf) -> Library {
    let mut tracks = WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let path = entry.into_path();
            is_supported_audio(&path)
                .then(|| track_from_path(path))
                .flatten()
        })
        .collect::<Vec<_>>();

    tracks.sort_by(|a, b| a.folder.cmp(&b.folder).then(a.path.cmp(&b.path)));

    Library {
        root: Some(root),
        tracks,
    }
}

fn is_supported_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}
