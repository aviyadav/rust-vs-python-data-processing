//! Generates a large CSV file of fake transaction records.
//!
//! Columns: date, transaction_id, category, merchant, amount
//!
//! Records are produced in fixed-size chunks (default 10,000 rows). Chunks
//! within a batch are generated in parallel across worker threads (one batch
//! = one chunk per thread), then written to disk in order and dropped before
//! the next batch starts. This keeps peak memory bounded to roughly
//! `num_workers * chunk_size` rows, no matter how many total records are
//! requested.

use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::Instant;

use chrono::Duration;
use rand::seq::SliceRandom;
use rand::Rng;
use rayon::prelude::*;
use uuid::Uuid;

const CATEGORIES: &[&str] = &[
    "Groceries",
    "Electronics",
    "Entertainment",
    "Utilities",
    "Travel",
    "Dining",
    "Healthcare",
    "Clothing",
    "Education",
    "Other",
];

const MERCHANTS: &[&str] = &[
    "Walmart",
    "Amazon",
    "Target",
    "Best Buy",
    "Costco",
    "Starbucks",
    "McDonald's",
    "Shell",
    "Chevron",
    "Home Depot",
    "Apple Store",
    "Netflix",
    "Spotify",
    "Uber",
    "Lyft",
    "Delta Airlines",
    "Marriott",
    "AT&T",
    "Verizon",
    "CVS Pharmacy",
    "Walgreens",
    "Whole Foods",
    "Trader Joe's",
    "IKEA",
    "Nike",
    "Adidas",
    "PlayStation Store",
    "Steam",
    "Zara",
    "H&M",
];

struct Config {
    total_records: usize,
    chunk_size: usize,
    output_path: PathBuf,
    num_workers: usize,
}

fn print_usage() {
    println!(
        "Usage: gen-data [OPTIONS]\n\n\
         Options:\n\
         \x20 -n, --records <N>      Total number of records to generate (default: 1000000)\n\
         \x20 -c, --chunk-size <N>   Rows generated per chunk (default: 10000)\n\
         \x20 -o, --output <PATH>    Output CSV path (default: ../../data/transactions.csv)\n\
         \x20 -w, --workers <N>      Number of parallel worker threads (default: number of CPUs)\n\
         \x20 -h, --help             Print this help message"
    );
}

fn parse_args() -> Config {
    let mut total_records: usize = 1_000_000;
    let mut chunk_size: usize = 10_000;
    // Anchored to this crate's location (rs-code/gen-data) so the default
    // always resolves to <repo>/data/transactions.csv, regardless of the
    // current working directory the binary happens to be run from.
    let mut output_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/transactions.csv");
    let mut num_workers = num_cpus::get();

    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-n" | "--records" => {
                if let Some(v) = args.get(i + 1) {
                    total_records = v.parse().unwrap_or_else(|_| {
                        eprintln!("Invalid value for --records: {v}");
                        std::process::exit(1);
                    });
                    i += 1;
                }
            }
            "-c" | "--chunk-size" => {
                if let Some(v) = args.get(i + 1) {
                    chunk_size = v.parse().unwrap_or_else(|_| {
                        eprintln!("Invalid value for --chunk-size: {v}");
                        std::process::exit(1);
                    });
                    i += 1;
                }
            }
            "-o" | "--output" => {
                if let Some(v) = args.get(i + 1) {
                    output_path = PathBuf::from(v);
                    i += 1;
                }
            }
            "-w" | "--workers" => {
                if let Some(v) = args.get(i + 1) {
                    num_workers = v.parse().unwrap_or_else(|_| {
                        eprintln!("Invalid value for --workers: {v}");
                        std::process::exit(1);
                    });
                    i += 1;
                }
            }
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            other => {
                eprintln!("Unknown argument: {other}");
                print_usage();
                std::process::exit(1);
            }
        }
        i += 1;
    }

    if chunk_size == 0 {
        eprintln!("--chunk-size must be greater than 0");
        std::process::exit(1);
    }
    if num_workers == 0 {
        num_workers = 1;
    }

    Config {
        total_records,
        chunk_size,
        output_path,
        num_workers,
    }
}

/// Generates `row_count` fake transaction rows as already-encoded CSV bytes
/// (no header). Each call uses its own thread-local RNG, so this is safe to
/// invoke concurrently from multiple threads.
fn generate_chunk(row_count: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false)
        .terminator(csv::Terminator::Any(b'\n'))
        .from_writer(Vec::with_capacity(row_count * 64));

    let today = chrono::Local::now().date_naive();
    let start_date = today - Duration::days(730); // two years of history
    let date_range_days: i64 = 730;

    for _ in 0..row_count {
        let days_offset = rng.gen_range(0..=date_range_days);
        let date = start_date + Duration::days(days_offset);
        let date_str = date.format("%Y-%m-%d").to_string();

        let transaction_id = Uuid::new_v4().to_string();
        let category = CATEGORIES.choose(&mut rng).unwrap();
        let merchant = MERCHANTS.choose(&mut rng).unwrap();
        let amount = rng.gen_range(1.00f64..5000.00f64);
        let amount_str = format!("{amount:.2}");

        wtr.write_record([
            date_str.as_str(),
            transaction_id.as_str(),
            category,
            merchant,
            amount_str.as_str(),
        ])
        .expect("failed to write CSV record");
    }

    wtr.flush().expect("failed to flush chunk writer");
    wtr.into_inner().expect("failed to extract chunk buffer")
}

fn main() {
    let config = parse_args();
    let start_time = Instant::now();

    if let Some(parent) = config.output_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).expect("failed to create output directory");
        }
    }

    let file = File::create(&config.output_path).expect("failed to create output file");
    let mut writer = BufWriter::with_capacity(8 * 1024 * 1024, file);

    writeln!(writer, "date,transaction_id,category,merchant,amount")
        .expect("failed to write CSV header");

    let total_chunks = (config.total_records + config.chunk_size - 1) / config.chunk_size;

    println!(
        "Generating {} records into {:?}",
        config.total_records, config.output_path
    );
    println!(
        "Chunk size: {} rows | Total chunks: {} | Worker threads: {}",
        config.chunk_size, total_chunks, config.num_workers
    );

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(config.num_workers)
        .build()
        .expect("failed to build worker thread pool");

    let mut records_written = 0usize;
    let mut chunk_idx = 0usize;

    // Process chunks in batches of `num_workers`. Only one batch's worth of
    // chunks (num_workers * chunk_size rows) is ever held in memory at once;
    // each batch is written to disk and dropped before the next is generated.
    while chunk_idx < total_chunks {
        let batch_end = (chunk_idx + config.num_workers).min(total_chunks);

        let chunk_row_counts: Vec<usize> = (chunk_idx..batch_end)
            .map(|idx| {
                if idx == total_chunks - 1 {
                    config.total_records - idx * config.chunk_size
                } else {
                    config.chunk_size
                }
            })
            .collect();

        let batch_bytes: Vec<Vec<u8>> = pool.install(|| {
            chunk_row_counts
                .par_iter()
                .map(|&row_count| generate_chunk(row_count))
                .collect()
        });

        for (rows, bytes) in chunk_row_counts.iter().zip(batch_bytes.into_iter()) {
            writer
                .write_all(&bytes)
                .expect("failed to write chunk to output file");
            records_written += rows;
        }

        chunk_idx = batch_end;

        print!(
            "\rProgress: {}/{} records ({:.1}%)",
            records_written,
            config.total_records,
            (records_written as f64 / config.total_records as f64) * 100.0
        );
        std::io::stdout().flush().ok();
    }

    writer.flush().expect("failed to flush output file");
    println!(
        "\nDone. Wrote {} records to {:?} in {:.2?}",
        records_written,
        config.output_path,
        start_time.elapsed()
    );
}
