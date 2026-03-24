use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

// Embed the compressed geodata binary
const GEODATA_BYTES: &[u8] = include_bytes!("geodata.bin.gz");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub country: String,
    pub admin1: String,
}

pub struct ReverseGeocoder {
    locations: Vec<GeoLocation>,
}

// Global singleton instance
static GEOCODER: OnceLock<ReverseGeocoder> = OnceLock::new();

impl ReverseGeocoder {
    pub fn new() -> Result<Self> {
        println!("🌍 Initializing Reverse Geocoder...");
        let start = std::time::Instant::now();

        // Decompress and deserialize
        let decoder = GzDecoder::new(GEODATA_BYTES);
        let locations: Vec<GeoLocation> = bincode::deserialize_from(decoder)
            .context("Failed to deserialize geodata")?;

        println!("✅ Geocoder initialized in {:?} with {} cities", start.elapsed(), locations.len());
        Ok(ReverseGeocoder { locations })
    }

    pub fn get() -> Option<&'static ReverseGeocoder> {
        GEOCODER.get()
    }

    pub fn init() {
        // Initialize in background or on first access
        let _ = GEOCODER.get_or_init(|| {
            match ReverseGeocoder::new() {
                Ok(g) => g,
                Err(e) => {
                    eprintln!("❌ Failed to initialize geocoder: {}", e);
                    panic!("Failed to initialize geocoder: {}", e);
                }
            }
        });
    }

    pub fn lookup(&self, lat: f64, lon: f64) -> Option<String> {
        // Simple linear search with squared euclidean distance
        // For ~163k cities this is fast enough (~1-2ms)
        let mut nearest: Option<&GeoLocation> = None;
        let mut nearest_dist_sq = f64::MAX;

        for loc in &self.locations {
            // Squared euclidean distance (faster than sqrt, sufficient for comparison)
            let d_lat = loc.lat - lat;
            let d_lon = loc.lon - lon;
            let dist_sq = d_lat * d_lat + d_lon * d_lon;

            if dist_sq < nearest_dist_sq {
                nearest_dist_sq = dist_sq;
                nearest = Some(loc);
            }
        }

        nearest.map(|loc| format!("{}, {}", loc.name, loc.country))
    }
}

// Public helper for easy access
pub fn get_location_name(lat: f64, lon: f64) -> Option<String> {
    if let Some(geocoder) = ReverseGeocoder::get() {
        geocoder.lookup(lat, lon)
    } else {
        // Try to init if not initialized (lazy)
        ReverseGeocoder::init();
        if let Some(geocoder) = ReverseGeocoder::get() {
            geocoder.lookup(lat, lon)
        } else {
            None
        }
    }
}
