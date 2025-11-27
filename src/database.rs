use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

// Structure to store metadata for each photo in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoMetadata {
    pub filename: String,
    pub relative_path: String, // Relative path from photos directory (e.g., "folder/IMG_0001.JPG")
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

// Structure for disk persistence
#[derive(Serialize, Deserialize)]
pub struct CachedDatabase {
    pub version: u32,  // Cache format version
    pub source_paths: Vec<String>,  // Multiple folder paths
    pub photos: Vec<PhotoMetadata>,
}

// Database connection wrapper
#[derive(Clone)]
pub struct Database {
    // In-memory storage for photos
    photos: Arc<RwLock<Vec<PhotoMetadata>>>,
}

impl Database {
    pub fn new() -> Result<Self> {
        Ok(Database {
            photos: Arc::new(RwLock::new(Vec::new())),
        })
    }

    pub fn clear_all_photos(&self) -> Result<()> {
        let mut photos = self.photos.write().unwrap();
        photos.clear();
        Ok(())
    }

    pub fn insert_photo(&self, photo: &PhotoMetadata) -> Result<()> {
        let mut photos = self.photos.write().unwrap();
        // Check if photo already exists (by relative_path) to mimic "INSERT OR REPLACE"
        if let Some(existing) = photos.iter_mut().find(|p| p.relative_path == photo.relative_path) {
            *existing = photo.clone();
        } else {
            photos.push(photo.clone());
        }
        Ok(())
    }

    /// Insert multiple photos in a single transaction for better performance
    pub fn insert_photos_batch(&self, new_photos: &[PhotoMetadata]) -> Result<usize> {
        if new_photos.is_empty() {
            return Ok(0);
        }

        let mut photos = self.photos.write().unwrap();
        let mut inserted = 0;

        for photo in new_photos {
             // Check if photo already exists (by relative_path) to mimic "INSERT OR REPLACE"
            if let Some(existing) = photos.iter_mut().find(|p| p.relative_path == photo.relative_path) {
                *existing = photo.clone();
                inserted += 1;
            } else {
                photos.push(photo.clone());
                inserted += 1;
            }
        }

        Ok(inserted)
    }

    pub fn get_all_photos(&self) -> Result<Vec<PhotoMetadata>> {
        let photos = self.photos.read().unwrap();
        // Return cloned vector. In a real DB we'd query.
        // Sorting by datetime DESC as in original query
        let mut result = photos.clone();
        result.sort_by(|a, b| b.datetime.cmp(&a.datetime));
        Ok(result)
    }

    pub fn get_photos_count(&self) -> Result<usize> {
        let photos = self.photos.read().unwrap();
        Ok(photos.len())
    }

    /// Save the current database state to disk using bincode
    pub fn save_to_disk(&self, source_paths: &[String]) -> Result<()> {
        let photos = self.photos.read().unwrap();
        let cache = CachedDatabase {
            version: 1,  // Cache format version
            source_paths: source_paths.to_vec(),
            photos: photos.clone(),
        };
        
        let app_dir = crate::utils::get_app_data_dir();
        crate::utils::ensure_directory_exists(&app_dir)?;
        let cache_path = app_dir.join("photos_v1.bin");  // New versioned filename
        
        let file = std::fs::File::create(cache_path)?;
        bincode::serialize_into(file, &cache)?;
        
        Ok(())
    }

    /// Load database state from disk if source paths match (100%)
    pub fn load_from_disk(&self, expected_paths: &[String]) -> Result<bool> {
        let app_dir = crate::utils::get_app_data_dir();
        
        // Clean up old files (TODO: remove this in future versions)
        let old_cache_path = app_dir.join("photos.bin");
        if old_cache_path.exists() {
            eprintln!("🗑️  Removing old cache format (photos.bin)");
            let _ = std::fs::remove_file(&old_cache_path);
        }
        
        let old_db_path = app_dir.join("photos.db");
        if old_db_path.exists() {
            eprintln!("🗑️  Removing old SQLite database (photos.db)");
            let _ = std::fs::remove_file(&old_db_path);
        }
        
        // Use new versioned cache filename
        let cache_path = app_dir.join("photos_v1.bin");
        
        if !cache_path.exists() {
            return Ok(false);
        }
        
        let file = std::fs::File::open(&cache_path)?;
        let cache: CachedDatabase = match bincode::deserialize_from(file) {
            Ok(c) => c,
            Err(_) => {
                // Corrupted or incompatible cache (e.g., old format without version)
                eprintln!("⚠️  Cache format incompatible or corrupted");
                eprintln!("🗑️  Deleting invalid cache file");
                let _ = std::fs::remove_file(&cache_path);
                return Ok(false);
            }
        };
        
        // Check version - delete file if mismatch
        if cache.version != 1 {
            eprintln!("⚠️  Cache version mismatch (found {}, expected 1)", cache.version);
            eprintln!("🗑️  Deleting outdated cache file");
            let _ = std::fs::remove_file(&cache_path);
            return Ok(false);
        }
        
        // Check if paths match exactly (100% match)
        if cache.source_paths != expected_paths {
            return Ok(false);
        }

        let mut photos = self.photos.write().unwrap();
        *photos = cache.photos;
        Ok(true)
    }
}
