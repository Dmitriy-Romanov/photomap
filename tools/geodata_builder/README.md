# GeoData Builder

Builds `src/geodata.bin.gz` from a GeoNames TSV dump.

PhotoMap expects the embedded file to contain:

```text
gzip(bincode(Vec<GeoLocation>))
```

where `GeoLocation` matches `src/geocoding.rs`.

## Usage

Download `cities1000.zip` from GeoNames, extract `cities1000.txt`, then run:

```bash
cargo run --release -- cities1000.txt ../../src/geodata.bin.gz
```

The tool reads GeoNames columns:

- `name`
- `latitude`
- `longitude`
- `country code`
- `admin1 code`

Malformed rows and rows with non-finite coordinates are skipped.
