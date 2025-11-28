use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
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
    tree: KdTree<f64, usize, [f64; 2]>,
}

// Global singleton instance
static GEOCODER: OnceLock<ReverseGeocoder> = OnceLock::new();

impl ReverseGeocoder {
    pub fn new() -> Result<Self> {
        println!("🌍 Initializing Reverse Geocoder...");
        let start = std::time::Instant::now();

        // 1. Decompress and Deserialize
        let decoder = GzDecoder::new(Cursor::new(GEODATA_BYTES));
        let locations: Vec<GeoLocation> = bincode::deserialize_from(decoder)
            .context("Failed to deserialize geodata")?;

        // 2. Build KD-Tree
        let mut tree = KdTree::new(2);
        for (i, loc) in locations.iter().enumerate() {
            tree.add([loc.lat, loc.lon], i)?;
        }

        println!("✅ Geocoder initialized in {:?} with {} cities", start.elapsed(), locations.len());
        Ok(ReverseGeocoder { locations, tree })
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
                    // Return a dummy/empty one or panic? 
                    // Better to panic or handle gracefully. 
                    // For now, let's panic since this is static data that should be valid.
                    panic!("Failed to initialize geocoder: {}", e);
                }
            }
        });
    }

    pub fn lookup(&self, lat: f64, lon: f64) -> Option<String> {
        // Find nearest neighbor
        // We use squared_euclidean for speed. For small distances on Earth, it's "okay" for finding nearest city.
        // For strict correctness we should use Haversine, but KdTree works with Euclidean.
        // Since we just want the NEAREST point, Euclidean on lat/lon is a reasonable approximation for "nearest city"
        // unless we are near poles or dateline, which is rare for photos.
        
        match self.tree.nearest(&[lat, lon], 1, &squared_euclidean) {
            Ok(nearest) => {
                if let Some((_dist, &index)) = nearest.first() {
                    let loc = &self.locations[index];
                    // Format: "Paris, FR"
                    return Some(format!("{}, {}", loc.name, loc.country));
                }
                None
            }
            Err(_) => None,
        }
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
