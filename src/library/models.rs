use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Track {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub path: PathBuf,
    pub folder: PathBuf,
    pub extension: String,
    pub cover_path: Option<PathBuf>,
    pub size: u64,
    pub modified: i64,
}

impl Track {
    pub fn display_artist_album(&self) -> String {
        match (&self.artist, &self.album) {
            (Some(artist), Some(album)) => format!("{artist} - {album}"),
            (Some(artist), None) => artist.clone(),
            (None, Some(album)) => album.clone(),
            (None, None) => self
                .folder
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Sin album")
                .to_string(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Library {
    pub root: Option<PathBuf>,
    pub tracks: Vec<Track>,
}

impl Library {
    pub fn folders(&self) -> Vec<PathBuf> {
        let mut folders = self
            .tracks
            .iter()
            .map(|track| track.folder.clone())
            .collect::<Vec<_>>();
        folders.sort();
        folders.dedup();
        folders
    }

    pub fn tracks_in_folder(&self, folder: &PathBuf) -> Vec<Track> {
        self.tracks
            .iter()
            .filter(|track| &track.folder == folder)
            .cloned()
            .collect()
    }

    pub fn search(&self, query: &str) -> Vec<Track> {
        let needle = query.trim().to_lowercase();
        if needle.is_empty() {
            return self.tracks.clone();
        }

        self.tracks
            .iter()
            .filter(|track| {
                track.title.to_lowercase().contains(&needle)
                    || track
                        .artist
                        .as_deref()
                        .unwrap_or_default()
                        .to_lowercase()
                        .contains(&needle)
                    || track
                        .album
                        .as_deref()
                        .unwrap_or_default()
                        .to_lowercase()
                        .contains(&needle)
                    || track
                        .path
                        .to_string_lossy()
                        .to_lowercase()
                        .contains(&needle)
                    || track
                        .folder
                        .to_string_lossy()
                        .to_lowercase()
                        .contains(&needle)
            })
            .cloned()
            .collect()
    }
}
