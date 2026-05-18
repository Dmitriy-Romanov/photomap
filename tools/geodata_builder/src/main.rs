use anyhow::{bail, Context, Result};
use flate2::{write::GzEncoder, Compression, GzBuilder};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeoLocation {
    name: String,
    lat: f64,
    lng: f64,
    country: String,
    admin1: String,
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        let program = args
            .first()
            .map(String::as_str)
            .unwrap_or("geodata_builder");
        eprintln!("Usage: {program} <geonames-cities1000.txt> <output-geodata.bin.gz>");
        eprintln!();
        eprintln!("Example:");
        eprintln!("  {program} cities1000.txt ../../src/geodata.bin.gz");
        bail!("expected input and output paths");
    }

    let input_path = Path::new(&args[1]);
    let output_path = Path::new(&args[2]);

    let locations = read_geonames(input_path)?;
    write_geodata(output_path, &locations)?;

    println!(
        "Wrote {} locations to {}",
        locations.len(),
        output_path.display()
    );

    Ok(())
}

fn read_geonames(path: &Path) -> Result<Vec<GeoLocation>> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut locations = Vec::new();
    let mut skipped = 0usize;

    for (line_index, line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("reading line {}", line_index + 1))?;

        match parse_geonames_line(&line) {
            Some(location) => locations.push(location),
            None => skipped += 1,
        }
    }

    if locations.is_empty() {
        bail!("no valid GeoNames rows found in {}", path.display());
    }

    if skipped > 0 {
        eprintln!("Skipped {skipped} malformed or non-finite row(s)");
    }

    Ok(locations)
}

fn parse_geonames_line(line: &str) -> Option<GeoLocation> {
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() < 11 {
        return None;
    }

    let name = fields[1].trim();
    let lat: f64 = fields[4].parse().ok()?;
    let lng: f64 = fields[5].parse().ok()?;
    let country = fields[8].trim();
    let admin1 = fields[10].trim();

    if name.is_empty() || country.is_empty() || !lat.is_finite() || !lng.is_finite() {
        return None;
    }

    Some(GeoLocation {
        name: name.to_string(),
        lat,
        lng,
        country: country.to_string(),
        admin1: admin1.to_string(),
    })
}

fn write_geodata(path: &Path, locations: &[GeoLocation]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }

    let file = File::create(path).with_context(|| format!("creating {}", path.display()))?;
    let writer = BufWriter::new(file);
    let encoder: GzEncoder<BufWriter<File>> =
        GzBuilder::new().mtime(0).write(writer, Compression::best());

    let mut encoder = encoder;
    bincode::serialize_into(&mut encoder, locations).context("serializing geodata")?;
    encoder.finish()?.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::GzDecoder;

    #[test]
    fn parses_geonames_city_row() {
        let line = "2950159\tBerlin\tBerlin\tBerlin\t52.52437\t13.41053\tP\tPPLC\tDE\t\t16\t\t\t\t3426354\t\t74\tEurope/Berlin\t2024-01-01";

        let location = parse_geonames_line(line).expect("valid GeoNames row");

        assert_eq!(location.name, "Berlin");
        assert_eq!(location.country, "DE");
        assert_eq!(location.admin1, "16");
        assert_eq!(location.lat, 52.52437);
        assert_eq!(location.lng, 13.41053);
    }

    #[test]
    fn writes_geodata_in_runtime_format() {
        let locations = vec![GeoLocation {
            name: "Berlin".to_string(),
            lat: 52.52437,
            lng: 13.41053,
            country: "DE".to_string(),
            admin1: "16".to_string(),
        }];

        let path = env::temp_dir().join(format!(
            "photomap-geodata-test-{}.bin.gz",
            std::process::id()
        ));

        write_geodata(&path, &locations).expect("write geodata");

        let file = File::open(&path).expect("open generated geodata");
        let decoder = GzDecoder::new(file);
        let decoded: Vec<GeoLocation> = bincode::deserialize_from(decoder).expect("decode geodata");

        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].name, "Berlin");

        let _ = std::fs::remove_file(path);
    }
}
