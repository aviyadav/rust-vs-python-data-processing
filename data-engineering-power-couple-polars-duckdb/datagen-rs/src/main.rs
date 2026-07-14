//! Generate million-user-event CSV records using Polars + Rayon.
//!
//! Memory-efficient: generates in batches, stream-writes each batch to CSV
//! without loading the full dataset into memory.
//!
//! Usage:
//!   ROWS=1000000 BATCH_SIZE=50000 OUTPUT=user_events.csv cargo run --release

use anyhow::{Context, Result};
use csv::WriterBuilder;
use polars::prelude::*;
use rand::distributions::{Distribution, Uniform};
use rand::rngs::SmallRng;
use rand::SeedableRng;
use rand_distr::LogNormal;
use rayon::prelude::*;
use std::fs::File;
use std::io::BufWriter;
use std::time::Instant;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------
const COUNTRIES: &[&str] = &["US", "GB", "DE", "FR", "IN", "BR", "JP", "CA", "AU", "SG"];
const DEVICES: &[&str] = &["desktop", "mobile", "tablet"];
const BROWSERS: &[&str] = &["Chrome", "Firefox", "Safari", "Edge"];

const TS_START: i64 = 1_740_787_200; // 2025-03-01T00:00:00Z
const TS_END: i64 = 1_741_046_399; // 2025-03-31T23:59:59Z

/// Default batch size (rows per chunk).
const BATCH_SIZE: usize = 50_000;

// ---------------------------------------------------------------------------
// Batch builder
// ---------------------------------------------------------------------------
fn make_batch(batch_size: usize, seed: u64) -> Result<DataFrame> {
    let mut rng = SmallRng::seed_from_u64(seed);

    // Pre-allocate vectors
    let mut event_ids = Vec::with_capacity(batch_size);
    let mut user_ids = Vec::with_capacity(batch_size);
    let mut countries = Vec::with_capacity(batch_size);
    let mut devices = Vec::with_capacity(batch_size);
    let mut browsers = Vec::with_capacity(batch_size);
    let mut session_times = Vec::with_capacity(batch_size);
    let mut purchase_amounts = Vec::with_capacity(batch_size);
    let mut timestamps = Vec::with_capacity(batch_size);

    let country_dist = Uniform::from(0..COUNTRIES.len());
    let device_dist = Uniform::from(0..DEVICES.len());
    let browser_dist = Uniform::from(0..BROWSERS.len());
    let ts_dist = Uniform::new_inclusive(TS_START, TS_END);
    let purchase_dist = Uniform::new(5.0f64, 500.0);
    let session_dist = LogNormal::new(4.5, 1.0)?;
    let zero_one = Uniform::new(0.0f64, 1.0);

    for _ in 0..batch_size {
        event_ids.push(Uuid::new_v4().to_string()[..12].to_string());
        user_ids.push(Uuid::new_v4().to_string()[..8].to_string());
        countries.push(COUNTRIES[country_dist.sample(&mut rng)].to_string());
        devices.push(DEVICES[device_dist.sample(&mut rng)].to_string());
        browsers.push(BROWSERS[browser_dist.sample(&mut rng)].to_string());

        // session_time: log-normal clipped to [1.0, 3600.0]
        let st: f64 = session_dist.sample(&mut rng);
        let st = (st * 10.0).round() / 10.0;
        let st = st.clamp(1.0, 3600.0);
        session_times.push(st);

        // purchase_amount: 30% chance, uniform(5, 500)
        let amt = if zero_one.sample(&mut rng) < 0.30 {
            let v: f64 = purchase_dist.sample(&mut rng);
            (v * 100.0).round() / 100.0
        } else {
            0.0
        };
        purchase_amounts.push(amt);

        // timestamp
        let ts_unix = ts_dist.sample(&mut rng);
        let ts = chrono::DateTime::from_timestamp(ts_unix, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default();
        timestamps.push(ts);
    }

    let df = df!(
        "event_id" => event_ids,
        "user_id" => user_ids,
        "country" => countries,
        "device" => devices,
        "browser" => browsers,
        "session_time_s" => session_times,
        "purchase_amount" => purchase_amounts,
        "timestamp" => timestamps,
    )?;

    Ok(df)
}

// ---------------------------------------------------------------------------
// Row-based CSV writer (streaming, per-batch)
// ---------------------------------------------------------------------------
fn write_batch_csv(writer: &mut csv::Writer<BufWriter<File>>, batch: &DataFrame) -> Result<usize> {
    let n_rows = batch.height();
    let cols = batch.get_columns();
    let n_cols = cols.len();

    for row_idx in 0..n_rows {
        let mut record = csv::StringRecord::with_capacity(128, n_cols);
        for col in cols {
            let val: String = match col.dtype() {
                DataType::Float64 => {
                    let v = col.f64().unwrap().get(row_idx).unwrap_or(0.0);
                    if v.fract() == 0.0 {
                        format!("{v:.1}")
                    } else {
                        format!("{v:.2}")
                    }
                }
                DataType::String => col.str().unwrap().get(row_idx).unwrap_or("").to_string(),
                _ => String::new(),
            };
            record.push_field(&val);
        }
        writer.write_record(&record).context("writing CSV record")?;
    }
    Ok(n_rows)
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------
fn main() -> Result<()> {
    let total_rows: usize = std::env::var("ROWS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1_000_000);
    let batch_size: usize = std::env::var("BATCH_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(BATCH_SIZE);
    let output = std::env::var("OUTPUT").unwrap_or_else(|_| "user_events.csv".to_string());

    let n_batches = total_rows.div_ceil(batch_size);

    eprintln!(
        "Generating {} rows in {} batches ({} rows/batch) ...",
        total_rows, n_batches, batch_size
    );
    let t0 = Instant::now();

    // ---- Phase 1: parallel batch generation using Rayon ----
    let batches: Vec<DataFrame> = (0..n_batches)
        .into_par_iter()
        .map(|i| {
            let seed = ((i as u64).wrapping_mul(2_654_435_761)) ^ 0xDEAD_BEEF;
            make_batch(batch_size, seed).expect("batch generation failed")
        })
        .collect();

    let t_gen = t0.elapsed();
    eprintln!("  Generation done in {:?} -- writing CSV ...", t_gen);

    // ---- Phase 2: streaming CSV write (single-threaded, sequential) ----
    let file = File::create(&output).with_context(|| format!("creating {output}"))?;
    let mut writer = WriterBuilder::new()
        .has_headers(false)
        .from_writer(BufWriter::with_capacity(4 * 1024 * 1024, file));

    let mut total_rows_written = 0usize;
    for (batch_idx, batch) in batches.iter().enumerate() {
        if batch_idx == 0 {
            // Write header from first batch's column names
            let headers: Vec<&str> = batch
                .get_column_names()
                .iter()
                .map(|s| s.as_str())
                .collect();
            writer
                .write_record(&headers)
                .context("writing CSV header")?;
        }
        total_rows_written += write_batch_csv(&mut writer, batch)?;
    }
    writer.flush().context("flushing CSV writer")?;

    let t_end = t0.elapsed();
    eprintln!("  CSV written in {:?}", t_end - t_gen);
    eprintln!("  Total rows: {}", total_rows_written);
    eprintln!("  Output: {output}");
    eprintln!("  Wall time: {:?}", t_end);

    // Validate: count newlines via buffered read (memory efficient)
    let mut in_file = File::open(&output).context("opening output for validation")?;
    let mut buf = [0u8; 65536];
    let mut newlines = 0usize;
    use std::io::Read;
    loop {
        let n = in_file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        newlines += buf[..n].iter().filter(|&&b| b == b'\n').count();
    }
    let data_lines = newlines.saturating_sub(1); // newline count starts at 1 for header
    assert_eq!(
        data_lines, total_rows,
        "Row count mismatch: got {data_lines}, expected {total_rows}"
    );
    eprintln!("All checks passed.");

    Ok(())
}
