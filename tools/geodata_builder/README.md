# GeoData Builder

Builds `src/geodata.bin.gz` from a GeoNames TSV dump.

PhotoMap expects the embedded file to contain:

```text
gzip(bincode(Vec<GeoLocation>))
```

where `GeoLocation` matches `src/geocoding.rs`.

## Usage

Download `cities5000.zip` from GeoNames, extract `cities5000.txt`, then run:

```bash
cargo run --release -- cities5000.txt ../../src/geodata.bin.gz
```

The tool reads GeoNames columns:

- `name`
- `latitude`
- `longitude`
- `country code`
- `population`

Malformed rows, rows with non-finite coordinates, and rows below 5,000 population are skipped.
