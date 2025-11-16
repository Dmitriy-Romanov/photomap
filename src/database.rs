use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use serde::{Serialize};

// Structure to store metadata for each photo in database
#[derive(Debug, Clone)]
pub struct PhotoMetadata {
    pub filename: String,
    pub relative_path: String,  // Relative path from photos directory (e.g., "folder/IMG_0001.JPG")
    pub datetime: String,
    pub lat: f64,
    pub lng: f64,
    pub file_path: String,
    pub is_heic: bool,
}

// Structure for API responses
#[derive(Serialize, Debug)]
pub struct ImageMetadata {
    pub filename: String,
    pub relative_path: String,
    pub url: String,
    pub fallback_url: String,
    pub marker_icon: String,
    pub lat: f64,
    pub lng: f64,
    pub datetime: String,
    pub file_path: String,
    pub is_heic: bool,
}

// Database connection wrapper
#[derive(Clone)]
pub struct Database {
    // SQLite connections aren't thread-safe, so we'll create connections per thread
    pub db_path: String,
}

impl Database {
    pub fn new() -> Result<Self> {
        let db_path = Self::database_path();
        let db = Database {
            db_path,
        };
        db.init_tables()?;
        Ok(db)
    }

    fn get_app_data_dir() -> std::path::PathBuf {
        // Cross-platform application data directory
        if cfg!(target_os = "macos") {
            let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            let mut path = std::path::PathBuf::from(home_dir);
            path.push("Library");
            path.push("Application Support");
            path.push("PhotoMap");
            path
        } else if cfg!(target_os = "windows") {
            // Use %APPDATA%/PhotoMap on Windows
            if let Ok(appdata) = std::env::var("APPDATA") {
                let mut path = std::path::PathBuf::from(appdata);
                path.push("PhotoMap");
                path
            } else {
                // Fallback to current directory
                std::path::PathBuf::from(".").join("PhotoMap")
            }
        } else {
            // Linux and others: use ~/.local/share/PhotoMap or ~/.photomap
            let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            let mut path = std::path::PathBuf::from(home_dir);

            // Try XDG_DATA_HOME first
            if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
                path = std::path::PathBuf::from(xdg_data);
            } else {
                path.push(".local");
                path.push("share");
            }
            path.push("PhotoMap");
            path
        }
    }

    pub fn database_path() -> String {
        let mut app_dir = Self::get_app_data_dir();

        // Create directory if it doesn't exist
        if !app_dir.exists() {
            let _ = std::fs::create_dir_all(&app_dir);
        }

        app_dir.push("photomap.db");
        app_dir.to_string_lossy().to_string()
    }

    pub fn init_tables(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)
            .with_context(|| "Failed to open database for initialization")?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS photos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                filename TEXT NOT NULL,
                relative_path TEXT NOT NULL UNIQUE,
                datetime TEXT NOT NULL,
                lat REAL NOT NULL,
                lng REAL NOT NULL,
                file_path TEXT NOT NULL,
                is_heic BOOLEAN NOT NULL DEFAULT 0,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).with_context(|| "Failed to create photos table")?;

        // Create indexes for performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_photos_lat_lng ON photos(lat, lng)",
            [],
        ).with_context(|| "Failed to create coordinates index")?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_photos_relative_path ON photos(relative_path)",
            [],
        ).with_context(|| "Failed to create relative_path index")?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_photos_filename ON photos(filename)",
            [],
        ).with_context(|| "Failed to create filename index")?;

        Ok(())
    }

    pub fn clear_all_photos(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)
            .with_context(|| "Failed to open database for clearing")?;

        conn.execute("DELETE FROM photos", params![])
            .with_context(|| "Failed to clear photos table")?;

        Ok(())
    }

    pub fn insert_photo(&self, photo: &PhotoMetadata) -> Result<()> {
        let conn = Connection::open(&self.db_path)
            .with_context(|| "Failed to open database for insert")?;

        conn.execute(
            "INSERT OR REPLACE INTO photos (filename, relative_path, datetime, lat, lng, file_path, is_heic)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                photo.filename,
                photo.relative_path,
                photo.datetime,
                photo.lat,
                photo.lng,
                photo.file_path,
                photo.is_heic
            ],
        ).with_context(|| format!("Failed to insert photo: {}", photo.relative_path))?;

        Ok(())
    }

    pub fn get_all_photos(&self) -> Result<Vec<PhotoMetadata>> {
        let conn = Connection::open(&self.db_path)
            .with_context(|| "Failed to open database for query")?;

        let mut stmt = conn.prepare(
            "SELECT filename, relative_path, datetime, lat, lng, file_path, is_heic FROM photos ORDER BY datetime DESC"
        )?;

        let photos = stmt.query_map([], |row| {
            Ok(PhotoMetadata {
                filename: row.get(0)?,
                relative_path: row.get(1)?,
                datetime: row.get(2)?,
                lat: row.get(3)?,
                lng: row.get(4)?,
                file_path: row.get(5)?,
                is_heic: row.get(6)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(photos)
    }

    pub fn get_photos_count(&self) -> Result<usize> {
        let conn = Connection::open(&self.db_path)
            .with_context(|| "Failed to open database for count")?;

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM photos",
            [],
            |row| row.get(0)
        )?;
        Ok(count as usize)
    }
}