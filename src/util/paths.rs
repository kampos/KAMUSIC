use std::path::PathBuf;

pub fn config_file() -> PathBuf {
    writable_dir("config").join("settings.json")
}

pub fn database_file() -> PathBuf {
    writable_dir("data").join("library.sqlite3")
}

pub fn cache_dir() -> PathBuf {
    writable_dir("cache").join("covers")
}

fn writable_dir(kind: &str) -> PathBuf {
    if let Some(dir) = project_dirs(kind) {
        return dir;
    }

    std::env::var_os("KAMUSIC_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".kamusic"))
        .join(kind)
}

fn project_dirs(kind: &str) -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "kampos", "KAMUSIC").and_then(|dirs| {
        let dir = match kind {
            "config" => dirs.config_dir().to_path_buf(),
            "data" => dirs.data_dir().to_path_buf(),
            "cache" => dirs.cache_dir().to_path_buf(),
            _ => return None,
        };

        std::fs::create_dir_all(&dir).ok()?;
        Some(dir)
    })
}
