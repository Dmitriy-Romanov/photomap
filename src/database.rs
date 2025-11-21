use anyhow::Result;
use serde::Serialize;
use std::sync::{Arc, RwLock};

// Structure to store metadata for each photo in database
#[derive(Debug, Clone)]
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
}
