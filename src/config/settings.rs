use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub last_music_dir: Option<PathBuf>,
    pub volume: f64,
    pub window_width: Option<i32>,
    pub window_height: Option<i32>,
    pub compact_mode: bool,
    pub favorite_tracks: Vec<PathBuf>,
    pub hidden_folders: Vec<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            last_music_dir: None,
            volume: 0.8,
            window_width: None,
            window_height: None,
            compact_mode: false,
            favorite_tracks: Vec::new(),
            hidden_folders: Vec::new(),
        }
    }
}

impl Settings {
    pub fn load(path: &std::path::Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }
}
