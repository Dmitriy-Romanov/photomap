use anyhow::{Context, Result};
use ignore::Walk;
use exif::{In, Reader, Tag, Value};
use rayon::prelude::*;
use serde::Serialize;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

// Structure to store metadata for each photo.
// `Serialize` is needed for JSON conversion.
#[derive(Serialize, Debug)]
struct ImageMetadata {
    filename: String,
    path: String,       // Relative path to original file
    thumbnail: String,  // Relative path to thumbnail
    lat: f64,
    lng: f64,
    datetime: String,   // Date and time from EXIF (DD.MM.YYYY HH:MM)
}

const THUMBNAIL_DIR: &str = ".thumbnails";
const THUMBNAIL_SIZE: u32 = 700;
const OUTPUT_FILE: &str = "geodata.js";
const MAP_HTML_FILE: &str = "map.html";

// Embedded HTML for the map
const MAP_HTML_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>PhotoMap</title>
    <link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.4/dist/leaflet.css" />
    <link rel="stylesheet" href="https://unpkg.com/leaflet.markercluster@1.5.3/dist/MarkerCluster.css" />
    <link rel="stylesheet" href="https://unpkg.com/leaflet.markercluster@1.5.3/dist/MarkerCluster.Default.css" />
    <style>
        body { margin: 0; padding: 0; }
        #map { height: 100vh; width: 100vw; }
        .popup-image {
            max-width: 700px;
            max-height: 700px;
            width: auto;
            height: auto;
            display: block;
        }
        .leaflet-popup-content {
            width: 720px !important;
            padding: 12px !important;
            margin: 0 !important;
        }
    </style>
</head>
<body>
    <div id="map"></div>

    <script src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js"></script>
    <script src="https://unpkg.com/leaflet.markercluster@1.5.3/dist/leaflet.markercluster.js"></script>

    <script>
        // Initialize map centered on world view
        const map = L.map('map').setView([20, 0], 2);

        // Add tile layer
        L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
            attribution: '© OpenStreetMap contributors'
        }).addTo(map);

        // Load photo data
        fetch('./geodata.js')
            .then(response => response.json())
            .then(data => {
                if (data.photos && data.photos.length > 0) {
                    // Create marker cluster group
                    const markerClusterGroup = L.markerClusterGroup({
                        chunkedLoading: true,
                        spiderfyOnMaxZoom: true,
                        showCoverageOnHover: true,
                        zoomToBoundsOnClick: true,
                        maxClusterRadius: 80
                    });

                    // Create bounds for fitting all markers
                    const bounds = L.latLngBounds();

                    // Add markers for each photo
                    data.photos.forEach(photo => {
                        const marker = L.marker([photo.lat, photo.lng]);

                        // Create popup content with image and metadata
                        const popupContent = `
                            <div style="text-align: center;">
                                <img src="${photo.thumbnail}" class="popup-image" alt="${photo.filename}" />
                                <div style="margin-top: 8px; font-size: 12px;">
                                    <strong>${photo.filename}</strong><br>
                                    ${photo.datetime}
                                </div>
                            </div>
                        `;

                        marker.bindPopup(popupContent);
                        markerClusterGroup.addLayer(marker);

                        // Extend bounds to include all markers
                        bounds.extend([photo.lat, photo.lng]);
                    });

                    // Add marker cluster group to map
                    map.addLayer(markerClusterGroup);

                    // Fit map to show all markers
                    map.fitBounds(bounds);

                } else {
                    // If no data, show message
                    L.popup()
                     .setLatLng(map.getCenter())
                     .setContent('No photos with GPS data found. Run photomap_processor to create them.')
                     .openOn(map);
                }
            })
            .catch(error => {
                console.error('Error loading photo data:', error);
                L.popup()
                 .setLatLng(map.getCenter())
                 .setContent('Error loading photo data. Make sure geodata.js exists.')
                 .openOn(map);
            });
    </script>

</body>
</html>"#;

fn main() -> Result<()> {
    println!("🗺️  PhotoMap Processor starting...");

    // 0. Create map.html if it doesn't exist
    if !std::path::Path::new(MAP_HTML_FILE).exists() {
        println!("📄 Creating map.html...");
        create_map_html()?;
        println!("✅ map.html created in current directory: {}", MAP_HTML_FILE);
    } else {
        println!("📄 map.html already exists in current directory: {}", MAP_HTML_FILE);
    }

    // 1. Create thumbnails directory if it doesn't exist
    fs::create_dir_all(THUMBNAIL_DIR)
        .with_context(|| format!("Failed to create thumbnails directory: {}", THUMBNAIL_DIR))?;

    // 2. Get list of all files in current directory and subdirectories
    println!("🔍 Scanning current directory and subdirectories...");
    let current_dir = std::env::current_dir()?;
    println!("📂 Current directory: {}", current_dir.display());

    // Create walker for current directory with limitations
    let walker = Walk::new(&current_dir);
    let files: Vec<PathBuf> = walker
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|e| {
            // Check that file is in current directory or its subdirectories
            e.path().starts_with(&current_dir)
        })
        .filter(|e| {
            // Exclude system directories and hidden files
            let path = e.path();
            if let Some(components) = path.components().collect::<Vec<_>>().get(1..) {
                for component in components {
                    if let Some(name) = component.as_os_str().to_str() {
                        if name.starts_with('.') || name == "node_modules" || name == "target" || name == ".git" {
                            return false;
                        }
                    }
                }
            }
            true
        })
        .filter(|e| e.file_type().map_or(false, |ft| ft.is_file()))
        .map(|e| e.into_path())
        .collect();
    println!("✅ Found {} files in current directory. Starting processing...", files.len());

    // 3. Process files in parallel using Rayon
    let photo_data: Vec<ImageMetadata> = files
        .par_iter() // <-- Parallelism magic!
        .filter_map(|path| process_file(path).ok()) // Filter out files that couldn't be processed
        .collect();

    println!("✅ Processing complete. Found {} photos with GPS data.", photo_data.len());

    // 4. Write result to geodata.js
    write_geodata_js(&photo_data)?;

    println!(
        "🎉 Done! Data saved to '{}' in current directory.",
        OUTPUT_FILE
    );
    println!("🌐 To view the map, open in browser: {}", std::env::current_dir()?.join(MAP_HTML_FILE).display());
    println!("💡 Or run: open {}", MAP_HTML_FILE);

    // Wait for user input before closing
    pause_and_wait_for_input()?;

    Ok(())
}

// Helper functions remain the same but with English comments and messages
// [The rest of the functions would continue here...]