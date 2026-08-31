# DataFusion 1BRC (1 Billion Row Challenge) in Rust

This repository contains a Rust implementation for processing weather station temperature measurements using Apache Arrow [DataFusion](https://datafusion.apache.org/). It serves as an exploration of querying large datasets (up to 1 billion rows) both from local disk formats (CSV & Parquet) and hosted remote servers.

## Project Structure

```text
onebrc/
├── Cargo.toml            # Project dependencies (DataFusion, object_store, tokio)
├── data/
│   ├── weather_stations.csv        # Source CSV (station names + values)
│   ├── weather_stations.parquet    # Generated local Parquet file
│   └── measurements.txt            # Generated dataset (up to 1B rows)
└── src/
    ├── main.rs           # Processes the CSV dataset
    └── bin/
        ├── convert.rs    # Converts CSV to Parquet
        ├── generate.rs    # Generates synthetic 1BRC-style data (up to 1B rows)
        └── pq1brc.rs     # Processes the Parquet dataset (faster, compressed)
```

---

## Getting Started

Make sure you have Rust and Cargo installed. Clone the repository and navigate to the project directory:

```bash
cd onebrc/
```

### 1. Generate Test Data

The `generate` binary produces synthetic data in the same format as `weather_stations.csv` — `station;temperature` rows with a name taken from the source CSV and a random value with four decimal places in the range `-90.0000` to `90.0000`. It is designed to scale to one billion rows.

```bash
cargo run --release --bin generate -- <rows> [output_path]
```

- `<rows>` — required, number of rows to generate.
- `output_path` — optional, defaults to `data/measurements.txt`. Parent directories are created automatically.

#### Examples by load size

| Rows        | Command                                                              | Approx. output size |
| ----------- | -------------------------------------------------------------------- | -------------------- |
| 1,000       | `cargo run --release --bin generate -- 1000`                         | ~15 KB               |
| 1,000,000   | `cargo run --release --bin generate -- 1000000`                      | ~15 MB               |
| 10,000,000  | `cargo run --release --bin generate -- 10000000`                     | ~150 MB              |
| 100,000,000 | `cargo run --release --bin generate -- 100000000`                    | ~1.4 GB              |
| 1,000,000,000 | `cargo run --release --bin generate -- 1000000000`                | ~13–15 GB            |

To write to a custom path (e.g. for benchmarking different sizes side by side):

```bash
cargo run --release --bin generate -- 1000000 ./data/measurements_1m.txt
```

#### How it stays fast at 1B rows

- A custom xorshift64* PRNG avoids the overhead of a crypto RNG and adds no dependencies.
- All 1,800,001 possible temperature strings (`-90.0000` to `90.0000`) are precomputed once, so per-row work becomes a small lookup and a few `write_all` calls instead of float formatting.
- An 8 MB `BufWriter` keeps syscall overhead low.
- A fixed RNG seed makes output reproducible across runs.
- Progress is reported to stderr every 10%.

After generating, you can point `main.rs` (CSV) or `pq1brc.rs` (Parquet) at the new file by editing the `let path = ...` line in each binary.

---

### 2. Run the CSV Query (`src/main.rs`, binary `onebrc`)

Reads a `station;temperature` CSV file and prints min/mean/max temperature per station, sorted by station name.

```bash
# default binary (default-run = "onebrc" in Cargo.toml)
cargo run --release

# explicit form
cargo run --release --bin onebrc

# build only, then run the binary directly
cargo build --release --bin onebrc
target/release/onebrc
```

**Input**: edit the `let path = ...` line in `src/main.rs` (currently `./data/measurements.txt`). The file's extension must match the `file_extension` option (currently `txt`).

### 3. Convert CSV to Parquet (`src/bin/convert.rs`, binary `convert`)

Parquet is columnar, compressed, and much faster to scan. Reads `./data/weather_stations.csv` and outputs `./data/weather_stations.parquet`.

```bash
cargo run --release --bin convert

# build only, then run the binary directly
cargo build --release --bin convert
target/release/convert
```

**Input/output**: edit the `input`/`output` variables in `src/bin/convert.rs`.

### 4. Run the Parquet Query (`src/bin/pq1brc.rs`, binary `pq1brc`)

Same query as the CSV binary, but reading a Parquet file — faster and compressed.

```bash
cargo run --release --bin pq1brc

# build only, then run the binary directly
cargo build --release --bin pq1brc
target/release/pq1brc
```

**Input**: edit the `let path = ...` line in `src/bin/pq1brc.rs` (currently `./data/weather_stations.parquet`).

### 5. Generate Test Data (`src/bin/generate.rs`, binary `generate`)

See the [Generate Test Data](#1-generate-test-data) section above for full details and load-size examples.

```bash
cargo run --release --bin generate -- <rows> [output_path]
```

---

## All Run Options — Quick Reference

| Binary    | Source file             | Command                                   | Purpose |
| --------- | ----------------------- | ----------------------------------------- | ------- |
| `onebrc`  | `src/main.rs`           | `cargo run --release`                      | Query CSV, print min/mean/max per station |
| `convert` | `src/bin/convert.rs`    | `cargo run --release --bin convert`        | Convert CSV to Parquet |
| `pq1brc`  | `src/bin/pq1brc.rs`     | `cargo run --release --bin pq1brc`         | Query Parquet, print min/mean/max per station |
| `generate`| `src/bin/generate.rs`   | `cargo run --release --bin generate -- <rows> [output_path]` | Generate synthetic data (up to 1B rows) |

Notes:

- `cargo run --release` with no `--bin` runs `onebrc` (set via `default-run = "onebrc"` in `Cargo.toml`).
- Always use `--release`; debug builds are far too slow for large datasets.
- All binaries resolve input paths relative to the directory you run from, so run from the crate root (`onebrc/`) or use absolute paths.

---

## Timing

Every program measures its processing time with `std::time::Instant` and prints `Time taken: ...` at the end of execution, just before printing the result.

| Program | What is timed | Sample time (same dataset) |
| ------- | ------------- | --------------------------- |
| `generate` | Row-generation loop only — station loading and temperature precomputation are excluded | 100K rows: ~10 ms |
| `onebrc` | Full CSV pipeline: scan + aggregate + sort + collect (result formatting excluded) | ~11.2 s |
| `convert` | CSV read + Parquet write | ~37.2 s |
| `pq1brc` | Full Parquet pipeline: scan + aggregate + sort + collect (result formatting excluded) | ~7.3 s |

Sample times were measured on the same generated dataset; your numbers will vary with hardware and dataset size. As the table shows, querying Parquet is roughly 35% faster than querying the equivalent CSV with DataFusion.

---

---

## Querying Hosted/Remote Files (1 Billion Rows)

If your dataset is very large (like the full 1-billion-row `measurements.txt` from the 1BRC challenge) and is hosted on a remote HTTP server, S3, or GCS, you can configure DataFusion to stream it directly rather than downloading it manually.

### Required Dependencies
Ensure your `Cargo.toml` includes the `object_store` crate with the `http` feature enabled:

```toml
[dependencies]
datafusion = "54.1.0"
object_store = { version = "0.13", features = ["http"] }
tokio = { version = "1.53.1", features = ["rt-multi-thread"] }
url = "2"
```

### Registering an HTTP Object Store in Rust
To query a hosted CSV/Parquet file directly from DataFusion, register the HTTP store scheme with your `SessionContext`:

```rust
use std::sync::Arc;
use url::Url;
use object_store::http::HttpBuilder;
use datafusion::prelude::*;
use datafusion::execution::options::ReadOptions;

// 1. Define remote file path
let path = "https://your-host.example.com/measurements.txt";

// 2. Parse URL and build an HTTP Object Store
let url = Url::parse(path).unwrap();
let base = Url::parse(&format!("{}://{}/", url.scheme(), url.host_str().unwrap())).unwrap();
let store = HttpBuilder::new().with_url(base.clone()).build().unwrap();

// 3. Register the store with your SessionContext
ctx.runtime_env().register_object_store(&base, Arc::new(store));

// 4. Register the file as a listing table
let opts = CsvReadOptions::new()
    .delimiter(b';')
    .has_header(false)
    .file_extension("txt")
    .schema(&schema);

let listing_opts = opts.to_listing_options(
    &ctx.copied_config(),
    ctx.copied_table_options()
);

ctx.register_listing_table("measurements", path, listing_opts, None, None).await.unwrap();

// 5. Query the listing table
let df = ctx.table("measurements").await.unwrap();
```

*Note: For a 1-billion-row CSV file over HTTP, the entire ~13 GB will be streamed. Using remote **Parquet** files instead allows DataFusion to use range requests to download only the metadata and required columns/row groups, dramatically reducing network consumption.*
