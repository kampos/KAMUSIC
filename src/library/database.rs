use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};

use crate::library::models::{Library, Track};

pub struct LibraryDatabase {
    conn: Connection,
}

impl LibraryDatabase {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tracks (
                path TEXT PRIMARY KEY,
                folder TEXT NOT NULL,
                title TEXT NOT NULL,
                artist TEXT,
                album TEXT,
                extension TEXT NOT NULL,
                cover_path TEXT,
                size INTEGER NOT NULL,
                modified INTEGER NOT NULL
            );",
        )?;
        Ok(Self { conn })
    }

    pub fn replace_library(&self, library: &Library) -> anyhow::Result<()> {
        self.conn.execute("DELETE FROM tracks", [])?;
        for track in &library.tracks {
            self.conn.execute(
                "INSERT OR REPLACE INTO tracks
                (path, folder, title, artist, album, extension, cover_path, size, modified)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    track.path.to_string_lossy(),
                    track.folder.to_string_lossy(),
                    track.title,
                    track.artist,
                    track.album,
                    track.extension,
                    track
                        .cover_path
                        .as_ref()
                        .map(|p| p.to_string_lossy().to_string()),
                    track.size as i64,
                    track.modified,
                ],
            )?;
        }
        Ok(())
    }

    pub fn load(&self) -> anyhow::Result<Library> {
        let mut stmt = self.conn.prepare(
            "SELECT path, folder, title, artist, album, extension, cover_path, size, modified
             FROM tracks ORDER BY folder, path",
        )?;
        let tracks = stmt
            .query_map([], |row| {
                Ok(Track {
                    path: PathBuf::from(row.get::<_, String>(0)?),
                    folder: PathBuf::from(row.get::<_, String>(1)?),
                    title: row.get(2)?,
                    artist: row.get(3)?,
                    album: row.get(4)?,
                    extension: row.get(5)?,
                    cover_path: row.get::<_, Option<String>>(6)?.map(PathBuf::from),
                    size: row.get::<_, i64>(7)? as u64,
                    modified: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Library { root: None, tracks })
    }
}
