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
    pub lng: f64,
    pub country: String,
}

pub struct ReverseGeocoder {
    locations: Vec<GeoLocation>,
}

// Global singleton instance — wrapped in Option so failures are stored as None
static GEOCODER: OnceLock<Option<ReverseGeocoder>> = OnceLock::new();

impl ReverseGeocoder {
    pub fn new() -> Result<Self> {
        println!("🌍 Initializing Reverse Geocoder...");
        let start = std::time::Instant::now();

        // Decompress and deserialize
        let decoder = GzDecoder::new(GEODATA_BYTES);
        use bincode::Options;
        let locations: Vec<GeoLocation> = bincode::options()
            .with_limit(20 * 1024 * 1024)
            .deserialize_from(decoder)
            .context("Failed to deserialize geodata")?;

        println!(
            "✅ Geocoder initialized in {:?} with {} cities",
            start.elapsed(),
            locations.len()
        );
        Ok(ReverseGeocoder { locations })
    }

    pub fn get() -> Option<&'static ReverseGeocoder> {
        GEOCODER.get().and_then(|opt| opt.as_ref())
    }

    pub fn init() {
        // Initialize in background or on first access — skip on corrupt/missing geodata
        let _ = GEOCODER.get_or_init(|| match ReverseGeocoder::new() {
            Ok(g) => Some(g),
            Err(e) => {
                eprintln!("⚠️ Skipping reverse geocoder: {}", e);
                None
            }
        });
    }

    pub fn lookup(&self, lat: f64, lng: f64) -> Option<String> {
        // Simple linear search with squared euclidean distance
        // For the embedded city set this is fast enough (~1-2ms)
        let mut nearest: Option<&GeoLocation> = None;
        let mut nearest_dist_sq = f64::MAX;

        for loc in &self.locations {
            // Squared euclidean distance (faster than sqrt, sufficient for comparison)
            let d_lat = loc.lat - lat;
            let d_lng = loc.lng - lng;
            let dist_sq = d_lat * d_lat + d_lng * d_lng;

            if dist_sq < nearest_dist_sq {
                nearest_dist_sq = dist_sq;
                nearest = Some(loc);
            }
        }

        nearest.map(|loc| format!("{}, {}", loc.name, loc.country))
    }
}

// Public helper for easy access
pub fn get_location_name(lat: f64, lng: f64) -> Option<String> {
    if let Some(geocoder) = ReverseGeocoder::get() {
        geocoder.lookup(lat, lng)
    } else {
        // Try to init if not initialized (lazy)
        ReverseGeocoder::init();
        if let Some(geocoder) = ReverseGeocoder::get() {
            geocoder.lookup(lat, lng)
        } else {
            None
        }
    }
}
