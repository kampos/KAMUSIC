use std::path::PathBuf;
use std::time::Duration;

use serde::Deserialize;

const RADIO_API: &str = "https://de1.api.radio-browser.info";

#[derive(Clone, Debug)]
pub enum OnlineKind {
    Radio { stream_url: String },
}

#[derive(Clone, Debug)]
pub struct OnlineItem {
    pub title: String,
    pub subtitle: String,
    pub cover_path: Option<PathBuf>,
    pub favicon_url: Option<String>,
    pub kind: OnlineKind,
}

#[derive(Clone, Debug, Deserialize)]
struct RadioStation {
    name: String,
    url_resolved: String,
    favicon: Option<String>,
    country: Option<String>,
    tags: Option<String>,
    bitrate: Option<u32>,
}

#[derive(Clone, Copy)]
struct RadioPreset {
    name: &'static str,
    query: &'static str,
}

const RADIO_PRESETS: &[RadioPreset] = &[
    RadioPreset {
        name: "LOS 40",
        query: "LOS 40",
    },
    RadioPreset {
        name: "Cadena SER",
        query: "Cadena SER",
    },
    RadioPreset {
        name: "Cadena 100",
        query: "Cadena 100",
    },
    RadioPreset {
        name: "COPE",
        query: "COPE",
    },
    RadioPreset {
        name: "Europa FM",
        query: "Europa FM",
    },
    RadioPreset {
        name: "Onda Cero",
        query: "Onda Cero",
    },
    RadioPreset {
        name: "Kiss FM",
        query: "Kiss FM",
    },
    RadioPreset {
        name: "LOS 40 Classic",
        query: "LOS 40 Classic",
    },
    RadioPreset {
        name: "Rock FM",
        query: "Rock FM",
    },
    RadioPreset {
        name: "Radiolé",
        query: "Radiolé",
    },
];

pub fn search_radio(query: &str) -> anyhow::Result<Vec<OnlineItem>> {
    let catalog = load_spanish_radio_catalog()?;
    let query = query.trim().to_lowercase();

    if query.is_empty() {
        return Ok(catalog);
    }

    Ok(catalog
        .into_iter()
        .filter(|item| {
            item.title.to_lowercase().contains(&query)
                || item.subtitle.to_lowercase().contains(&query)
        })
        .collect())
}

fn load_spanish_radio_catalog() -> anyhow::Result<Vec<OnlineItem>> {
    let client = http_client()?;
    let mut items = Vec::new();

    for preset in RADIO_PRESETS {
        if let Some(item) = fetch_radio_station(&client, preset) {
            items.push(item);
        }
    }

    Ok(items)
}

fn fetch_radio_station(
    client: &reqwest::blocking::Client,
    preset: &RadioPreset,
) -> Option<OnlineItem> {
    let response = client
        .get(format!("{RADIO_API}/json/stations/search"))
        .query(&[
            ("hidebroken", "true"),
            ("limit", "5"),
            ("countrycode", "ES"),
            ("name", preset.query),
        ])
        .send()
        .ok()?;

    let stations = response.json::<Vec<RadioStation>>().ok()?;
    let station = stations
        .iter()
        .find(|station| {
            station
                .name
                .to_lowercase()
                .contains(&preset.query.to_lowercase())
        })
        .cloned()
        .or_else(|| stations.first().cloned())?;

    let subtitle = build_radio_subtitle(&station);
    let cover_path = station
        .favicon
        .as_deref()
        .and_then(|favicon| download_thumbnail(client, favicon, "radio"));

    Some(OnlineItem {
        title: preset.name.to_string(),
        subtitle,
        cover_path,
        favicon_url: station.favicon.clone(),
        kind: OnlineKind::Radio {
            stream_url: station.url_resolved,
        },
    })
}

fn build_radio_subtitle(station: &RadioStation) -> String {
    let country = station
        .country
        .clone()
        .unwrap_or_else(|| "Spain".to_string());
    let tags = station.tags.clone().unwrap_or_default();
    let bitrate = station
        .bitrate
        .map(|bitrate| format!("{bitrate} kbps"))
        .unwrap_or_else(|| "Streaming".to_string());

    match (tags.is_empty(), country.is_empty()) {
        (true, true) => bitrate,
        (true, false) => format!("{country} · {bitrate}"),
        (false, true) => format!("{tags} · {bitrate}"),
        (false, false) => format!("{country} · {tags} · {bitrate}"),
    }
}

pub(crate) fn http_client() -> anyhow::Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder()
        .user_agent("KAMUSIC/0.1")
        .timeout(Duration::from_secs(12))
        .build()?)
}

pub(crate) fn download_thumbnail(
    client: &reqwest::blocking::Client,
    url: &str,
    prefix: &str,
) -> Option<PathBuf> {
    let cache_dir = crate::util::paths::cache_dir().join("covers").join(prefix);
    std::fs::create_dir_all(&cache_dir).ok()?;
    let cache_path = cache_dir.join(format!("{}.jpg", stable_hash(url)));
    if cache_path.is_file() {
        return Some(cache_path);
    }

    let bytes = client.get(url).send().ok()?.bytes().ok()?;
    std::fs::write(&cache_path, bytes).ok()?;
    Some(cache_path)
}

fn stable_hash(value: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
