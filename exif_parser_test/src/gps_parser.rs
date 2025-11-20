/// Custom GPS parser for malformed EXIF files
/// This module implements direct GPS IFD reading to handle files with broken IFD chains
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// EXIF byte order
#[derive(Debug, Clone, Copy)]
enum ByteOrder {
    LittleEndian,
    BigEndian,
}

/// Read GPS coordinates directly from EXIF data, bypassing broken IFD chains
pub fn extract_gps_from_malformed_exif(path: &Path) -> Option<(f64, f64)> {
    let mut file = File::open(path).ok()?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).ok()?;
    
    // Find EXIF marker in JPEG (0xFFE1)
    let exif_start = find_exif_segment(&buffer)?;
    
    // Parse TIFF header
    // APP1 structure: FF E1 [2 bytes length] "Exif\0\0" [TIFF data]
    let tiff_start = exif_start + 4 + 6; // Skip marker (2) + length (2) + "Exif\0\0" (6)
    if tiff_start + 8 > buffer.len() {
        return None;
    }
    
    // Determine byte order
    let byte_order = match &buffer[tiff_start..tiff_start + 2] {
        b"II" => ByteOrder::LittleEndian,
        b"MM" => ByteOrder::BigEndian,
        _ => return None,
    };
    
    // Verify TIFF magic number (42)
    let magic = read_u16(&buffer[tiff_start + 2..tiff_start + 4], byte_order);
    if magic != 42 {
        return None;
    }
    
    // Read offset to first IFD
    let ifd0_offset = read_u32(&buffer[tiff_start + 4..tiff_start + 8], byte_order) as usize;
    
    // Try to find GPS IFD offset in IFD0
    if let Some(gps_ifd_offset) = find_gps_ifd_offset(&buffer, tiff_start, ifd0_offset, byte_order) {
        // Read GPS data from GPS IFD
        return parse_gps_ifd(&buffer, tiff_start, gps_ifd_offset, byte_order);
    }
    
    None
}

/// Find EXIF segment in JPEG
fn find_exif_segment(data: &[u8]) -> Option<usize> {
    if data.len() < 4 || &data[0..2] != b"\xFF\xD8" {
        return None; // Not a JPEG
    }
    
    let mut pos = 2;
    while pos + 4 < data.len() {
        if data[pos] != 0xFF {
            return None;
        }
        
        let marker = data[pos + 1];
        let length = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
        
        // Check for APP1 (EXIF) marker
        if marker == 0xE1 && pos + 10 < data.len() && &data[pos + 4..pos + 10] == b"Exif\0\0" {
            return Some(pos);
        }
        
        pos += 2 + length;
    }
    
    None
}

/// Find GPS IFD offset in IFD0
fn find_gps_ifd_offset(data: &[u8], tiff_start: usize, ifd_offset: usize, byte_order: ByteOrder) -> Option<usize> {
    let ifd_pos = tiff_start + ifd_offset;
    if ifd_pos + 2 > data.len() {
        return None;
    }
    
    let num_entries = read_u16(&data[ifd_pos..ifd_pos + 2], byte_order) as usize;
    let mut pos = ifd_pos + 2;
    
    for _ in 0..num_entries {
        if pos + 12 > data.len() {
            break;
        }
        
        let tag = read_u16(&data[pos..pos + 2], byte_order);
        
        // GPS IFD Pointer tag (0x8825)
        if tag == 0x8825 {
            let gps_offset = read_u32(&data[pos + 8..pos + 12], byte_order) as usize;
            return Some(gps_offset);
        }
        
        pos += 12;
    }
    
    None
}

/// Parse GPS IFD and extract coordinates
fn parse_gps_ifd(data: &[u8], tiff_start: usize, gps_offset: usize, byte_order: ByteOrder) -> Option<(f64, f64)> {
    let gps_pos = tiff_start + gps_offset;
    if gps_pos + 2 > data.len() {
        return None;
    }
    
    let num_entries = read_u16(&data[gps_pos..gps_pos + 2], byte_order) as usize;
    let mut pos = gps_pos + 2;
    
    let mut lat: Option<f64> = None;
    let mut lat_ref: Option<char> = None;
    let mut lon: Option<f64> = None;
    let mut lon_ref: Option<char> = None;
    
    for _ in 0..num_entries {
        if pos + 12 > data.len() {
            break;
        }
        
        let tag = read_u16(&data[pos..pos + 2], byte_order);
        let format = read_u16(&data[pos + 2..pos + 4], byte_order);
        let count = read_u32(&data[pos + 4..pos + 8], byte_order);
        let value_offset = read_u32(&data[pos + 8..pos + 12], byte_order);
        
        match tag {
            1 => { // GPSLatitudeRef
                if format == 2 && count >= 1 {
                    lat_ref = Some(data[pos + 8] as char);
                }
            }
            2 => { // GPSLatitude
                if format == 5 && count == 3 {
                    lat = read_gps_coordinate(data, tiff_start, value_offset as usize, byte_order);
                }
            }
            3 => { // GPSLongitudeRef
                if format == 2 && count >= 1 {
                    lon_ref = Some(data[pos + 8] as char);
                }
            }
            4 => { // GPSLongitude
                if format == 5 && count == 3 {
                    lon = read_gps_coordinate(data, tiff_start, value_offset as usize, byte_order);
                }
            }
            _ => {}
        }
        
        pos += 12;
    }
    
    // Combine coordinates with references
    let mut final_lat = lat?;
    let mut final_lon = lon?;
    
    if lat_ref == Some('S') {
        final_lat = -final_lat;
    }
    if lon_ref == Some('W') {
        final_lon = -final_lon;
    }
    
    Some((final_lat, final_lon))
}

/// Read GPS coordinate (degrees, minutes, seconds) and convert to decimal
fn read_gps_coordinate(data: &[u8], tiff_start: usize, offset: usize, byte_order: ByteOrder) -> Option<f64> {
    let pos = tiff_start + offset;
    if pos + 24 > data.len() {
        return None;
    }
    
    // Read 3 rational values (degrees, minutes, seconds)
    let deg_num = read_u32(&data[pos..pos + 4], byte_order) as f64;
    let deg_den = read_u32(&data[pos + 4..pos + 8], byte_order) as f64;
    
    let min_num = read_u32(&data[pos + 8..pos + 12], byte_order) as f64;
    let min_den = read_u32(&data[pos + 12..pos + 16], byte_order) as f64;
    
    let sec_num = read_u32(&data[pos + 16..pos + 20], byte_order) as f64;
    let sec_den = read_u32(&data[pos + 20..pos + 24], byte_order) as f64;
    
    if deg_den == 0.0 || min_den == 0.0 || sec_den == 0.0 {
        return None;
    }
    
    let degrees = deg_num / deg_den;
    let minutes = min_num / min_den;
    let seconds = sec_num / sec_den;
    
    Some(degrees + minutes / 60.0 + seconds / 3600.0)
}

/// Read u16 with specified byte order
fn read_u16(data: &[u8], byte_order: ByteOrder) -> u16 {
    match byte_order {
        ByteOrder::LittleEndian => u16::from_le_bytes([data[0], data[1]]),
        ByteOrder::BigEndian => u16::from_be_bytes([data[0], data[1]]),
    }
}

/// Read u32 with specified byte order
fn read_u32(data: &[u8], byte_order: ByteOrder) -> u32 {
    match byte_order {
        ByteOrder::LittleEndian => u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
        ByteOrder::BigEndian => u32::from_be_bytes([data[0], data[1], data[2], data[3]]),
    }
}
