use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::library::models::Track;

const COVER_NAMES: &[&str] = &[
    "cover.jpg",
    "cover.png",
    "folder.jpg",
    "folder.png",
    "front.jpg",
    "front.png",
    "album.jpg",
    "album.png",
];

pub fn find_folder_cover(folder: &Path) -> Option<PathBuf> {
    for name in COVER_NAMES {
        let candidate = folder.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    std::fs::read_dir(folder).ok()?.flatten().find_map(|entry| {
        let path = entry.path();
        let ext = path.extension()?.to_str()?.to_lowercase();
        matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp").then_some(path)
    })
}

pub fn resolve_track_cover(track: &Track) -> Option<PathBuf> {
    find_folder_cover(&track.folder)
}

pub fn download_itunes_cover(track: &Track) -> Option<PathBuf> {
    let query = build_query(track);
    if query.is_empty() {
        return None;
    }

    let cache_dir = crate::util::paths::cache_dir()
        .join("covers")
        .join("itunes");
    std::fs::create_dir_all(&cache_dir).ok()?;
    let cache_name = stable_hash(&query);
    let cache_path = cache_dir.join(format!("{cache_name}.jpg"));
    if cache_path.is_file() {
        return Some(cache_path);
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .ok()?;
    let response = client
        .get("https://itunes.apple.com/search")
        .query(&[
            ("term", query.as_str()),
            ("media", "music"),
            ("entity", "song"),
            ("limit", "1"),
            ("country", "US"),
        ])
        .send()
        .ok()?;
    let payload = response.json::<ItunesSearchResponse>().ok()?;
    let artwork_url = payload.results.first()?.artwork_url_100.as_ref()?;
    let bytes = client.get(artwork_url).send().ok()?.bytes().ok()?;
    std::fs::write(&cache_path, bytes).ok()?;
    Some(cache_path)
}

#[derive(Debug, Deserialize)]
struct ItunesSearchResponse {
    results: Vec<ItunesTrack>,
}

#[derive(Debug, Deserialize)]
struct ItunesTrack {
    #[serde(rename = "artworkUrl100")]
    artwork_url_100: Option<String>,
}

fn build_query(track: &Track) -> String {
    match (&track.artist, &track.album) {
        (Some(artist), Some(album)) => format!("{artist} {album}"),
        (Some(artist), None) => format!("{artist} {}", track.title),
        (None, Some(album)) => format!("{album} {}", track.title),
        (None, None) => track.title.clone(),
    }
}

fn stable_hash(value: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
