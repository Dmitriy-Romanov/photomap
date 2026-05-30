use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoMetadata {
    pub filename: String,
    pub relative_path: String,
    pub datetime: String,
    pub lat: f64,
    pub lng: f64,
    pub file_path: String,
    pub is_heic: bool,
}

#[derive(Serialize, Debug, Clone, Deserialize)]
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
    pub location: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CachedDatabase {
    pub version: u32,
    pub source_paths: Vec<String>,
    pub photos: Vec<PhotoMetadata>,
}

#[derive(Clone)]
pub struct Database {
    photos: Arc<RwLock<HashMap<String, PhotoMetadata>>>,
}

impl Database {
    pub fn new() -> Result<Self> {
        Ok(Database {
            photos: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub fn clear_all_photos(&self) -> Result<()> {
        let mut photos = self.photos.write().unwrap();
        photos.clear();
        Ok(())
    }

    pub fn insert_photo(&self, photo: &PhotoMetadata) -> Result<()> {
        let mut photos = self.photos.write().unwrap();
        photos.insert(photo.relative_path.clone(), photo.clone());
        Ok(())
    }

    pub fn insert_photos_batch(&self, new_photos: &[PhotoMetadata]) -> Result<usize> {
        if new_photos.is_empty() {
            return Ok(0);
        }
        let mut photos = self.photos.write().unwrap();
        for photo in new_photos {
            photos.insert(photo.relative_path.clone(), photo.clone());
        }
        Ok(new_photos.len())
    }

    pub fn get_all_photos(&self) -> Result<Vec<PhotoMetadata>> {
        let photos = self.photos.read().unwrap();
        let mut result: Vec<_> = photos.values().cloned().collect();
        result.sort_by(|a, b| b.datetime.cmp(&a.datetime));
        Ok(result)
    }

    pub fn get_photos_count(&self) -> Result<usize> {
        let photos = self.photos.read().unwrap();
        Ok(photos.len())
    }

    pub fn get_photo_by_relative_path(&self, relative_path: &str) -> Result<Option<PhotoMetadata>> {
        let photos = self.photos.read().unwrap();
        Ok(photos.get(relative_path).cloned())
    }

    pub fn save_to_disk(&self, source_paths: &[String]) -> Result<()> {
        let photos = self.photos.read().unwrap();
        let cache = CachedDatabase {
            version: 1,
            source_paths: source_paths.to_vec(),
            photos: photos.values().cloned().collect(),
        };
        let app_dir = crate::utils::get_app_data_dir();
        crate::utils::ensure_directory_exists(&app_dir)?;
        let cache_path = app_dir.join("photos_v1.bin");
        let file = std::fs::File::create(cache_path)?;
        bincode::serialize_into(file, &cache)?;
        Ok(())
    }

    pub fn load_from_disk(&self, expected_paths: &[String]) -> Result<bool> {
        let app_dir = crate::utils::get_app_data_dir();
        let old_cache_path = app_dir.join("photos.bin");
        if old_cache_path.exists() {
            eprintln!("🗑️ Removing old cache format (photos.bin)");
            let _ = std::fs::remove_file(&old_cache_path);
        }
        let old_db_path = app_dir.join("photos.db");
        if old_db_path.exists() {
            eprintln!("🗑️ Removing old SQLite database (photos.db)");
            let _ = std::fs::remove_file(&old_db_path);
        }
        let cache_path = app_dir.join("photos_v1.bin");
        if !cache_path.exists() {
            return Ok(false);
        }
        let file = std::fs::File::open(&cache_path)?;
        let cache: CachedDatabase = match bincode::deserialize_from(file) {
            Ok(c) => c,
            Err(_) => {
                eprintln!("⚠️ Cache format incompatible or corrupted");
                eprintln!("🗑️ Deleting invalid cache file");
                let _ = std::fs::remove_file(&cache_path);
                return Ok(false);
            }
        };
        if cache.version != 1 {
            eprintln!("⚠️ Cache version mismatch (found {}, expected 1)", cache.version);
            eprintln!("🗑️ Deleting outdated cache file");
            let _ = std::fs::remove_file(&cache_path);
            return Ok(false);
        }
        if cache.source_paths != expected_paths {
            return Ok(false);
        }
        let mut photos = self.photos.write().unwrap();
        *photos = cache.photos.into_iter().map(|p| (p.relative_path.clone(), p)).collect();
        Ok(true)
    }
}
