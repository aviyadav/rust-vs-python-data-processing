//! generate-transaction-data-mp  (Rust port)
//! ─────────────────────────────────────────
//! Memory-safe streaming generator for large transaction datasets.
//! Faithfully mirrors the Python pipeline:
//!
//!   1. Rayon work-stealing pool  – N threads each fill a bounded sub-chunk
//!   2. Streaming CSV write       – one batch at a time goes to disk
//!   3. Automatic memory reclaim  – chunk Vecs are dropped before next batch
//!
//! Peak RAM ≈ memory for ONE batch, regardless of total dataset size.
//!
//! Tunables (top of file)
//! ──────────────────────
//!   NUM_RECORDS   Total rows to produce            (default 1_000_000_000)
//!   BATCH_SIZE    Rows processed per iteration     (default 500_000 ≈ 50 MB)
//!   MAX_WORKERS   Thread-pool cap                  (default capped at 8)

use std::fs;
use std::io::{BufWriter, Write};
use std::time::Instant;

use rayon::prelude::*;
use rand::prelude::*;
use rand::rngs::SmallRng;

// ── Configuration ──────────────────────────────────────────────────────────────

const NUM_RECORDS: u64 = 1_000_000_000;

/// Rows held in RAM at once.  Each row ≈ 90 bytes, so 500 k rows ≈ 45 MB.
const BATCH_SIZE: u64 = 500_000;

/// Hard cap on the worker-thread count (mirrors Python's min(cpu_count, 8)).
const MAX_WORKERS: usize = 8;

const OUTPUT_DIR: &str = "data";
const OUTPUT_FILE: &str = "data/transaction-data.csv";

// ── Domain constants ───────────────────────────────────────────────────────────

const PRODUCTS: &[&str] = &[
    "Camera", "Charger", "Tablet", "Printer", "Monitor",
    "Laptop", "Headphones", "Keyboard", "Phone", "Mouse",
];
const STORES: &[&str] = &["Store_A", "Store_B", "Store_C", "Store_D", "Store_E"];
const PAYMENT_METHODS: &[&str] = &[
    "Cash", "Credit Card", "Debit Card", "UPI payment", "Apple Pay",
];

/// Inclusive date range: 2023-01-01 .. 2024-12-31  → 730 possible offsets (0..=730).
const DATE_RANGE_DAYS: u32 = 730;

// ── Date lookup table ──────────────────────────────────────────────────────────

/// Pre-compute all 731 date strings as fixed 10-byte arrays ("YYYY-MM-DD").
/// Avoids any per-row date arithmetic; O(1) lookup at generation time.
fn build_date_table() -> Vec<[u8; 10]> {
    let mut table = Vec::with_capacity(DATE_RANGE_DAYS as usize + 1);

    let days_in_month = |yr: u32, mo: u32| -> u32 {
        match mo {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                // Gregorian leap-year rule
                if yr % 4 == 0 && (yr % 100 != 0 || yr % 400 == 0) {
                    29
                } else {
                    28
                }
            }
            _ => unreachable!(),
        }
    };

    let (mut year, mut month, mut day) = (2023u32, 1u32, 1u32);
    for _ in 0..=DATE_RANGE_DAYS as usize {
        let mut buf = [0u8; 10];
        buf[0] = b'0' + (year / 1000) as u8;
        buf[1] = b'0' + (year / 100 % 10) as u8;
        buf[2] = b'0' + (year / 10 % 10) as u8;
        buf[3] = b'0' + (year % 10) as u8;
        buf[4] = b'-';
        buf[5] = b'0' + (month / 10) as u8;
        buf[6] = b'0' + (month % 10) as u8;
        buf[7] = b'-';
        buf[8] = b'0' + (day / 10) as u8;
        buf[9] = b'0' + (day % 10) as u8;
        table.push(buf);

        // Advance to next calendar day
        day += 1;
        if day > days_in_month(year, month) {
            day = 1;
            month += 1;
            if month > 12 {
                month = 1;
                year += 1;
            }
        }
    }
    table
}

// ── Chunk generation (runs inside each Rayon worker thread) ────────────────────

/// Generate one sub-chunk of transaction rows into a raw CSV byte buffer.
///
/// # Arguments
/// * `start_idx`  – 1-based first transaction number in this sub-chunk
/// * `count`      – number of rows to produce
/// * `seed`       – unique RNG seed; deterministic & reproducible
/// * `txn_width`  – zero-padding width for `TXN` IDs
/// * `date_table` – shared pre-computed date strings
///
/// # Returns
/// `Vec<u8>` containing newline-separated CSV rows (no header).
fn generate_chunk(
    start_idx: u64,
    count: u64,
    seed: u64,
    txn_width: usize,
    date_table: &[[u8; 10]],
) -> Vec<u8> {
    let mut rng = SmallRng::seed_from_u64(seed);

    // Reserve ~90 bytes per row to avoid mid-loop reallocations
    let mut buf: Vec<u8> = Vec::with_capacity(count as usize * 95);

    for txn_num in start_idx..(start_idx + count) {
        let cust_num: u32 = rng.gen_range(1..=100_000);
        let product = PRODUCTS[rng.gen_range(0..PRODUCTS.len())];
        // Round to 2 decimal places the same way Python's np.round does
        let amount: f64 =
            (rng.gen_range(100.0_f64..=2_500.0_f64) * 100.0).round() / 100.0;
        let quantity: u8 = rng.gen_range(1..=10);
        let store = STORES[rng.gen_range(0..STORES.len())];
        let payment = PAYMENT_METHODS[rng.gen_range(0..PAYMENT_METHODS.len())];
        let day_offset: u32 = rng.gen_range(0..=DATE_RANGE_DAYS);

        // {0:0>1$} → print arg 0 (txn_num) zero-padded right-aligned to width arg 1 (txn_width)
        write!(buf, "TXN{:0>1$},", txn_num, txn_width).unwrap();
        write!(
            buf,
            "CUST{:05},{},{:.2},{},{},{},",
            cust_num, product, amount, quantity, store, payment
        )
        .unwrap();
        buf.extend_from_slice(&date_table[day_offset as usize]);
        buf.push(b'\n');
    }

    buf
}

// ── Batch coordination ─────────────────────────────────────────────────────────

struct SubTask {
    start_idx: u64,
    count: u64,
    seed: u64,
}

/// Divide one batch into per-worker sub-tasks with unique deterministic seeds.
///
/// Two large primes keep (batch_num, worker_idx) seeds far apart in seed-space
/// so no two workers ever share overlapping RNG sequences (mirrors Python logic).
fn build_sub_tasks(
    batch_start: u64,
    batch_size: u64,
    num_workers: usize,
    batch_num: u64,
) -> Vec<SubTask> {
    let base = batch_size / num_workers as u64;
    let extras = (batch_size % num_workers as u64) as usize;
    let mut tasks = Vec::with_capacity(num_workers);
    let mut idx = batch_start;

    for i in 0..num_workers {
        let count = base + u64::from(i < extras);
        let seed = batch_num * 104_729 + i as u64 * 7_919;
        tasks.push(SubTask { start_idx: idx, count, seed });
        idx += count;
    }
    tasks
}

// ── Formatting helpers ─────────────────────────────────────────────────────────

fn fmt_duration(secs: f64) -> String {
    let h = (secs / 3_600.0) as u64;
    let m = ((secs % 3_600.0) / 60.0) as u64;
    let s = (secs % 60.0) as u64;
    format!("{}:{:02}:{:02}", h, m, s)
}

/// Format a u64 with comma thousand separators (e.g. 1_000_000 → "1,000,000").
fn fmt_num(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

// ── Entry point ────────────────────────────────────────────────────────────────

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── Worker-thread count ────────────────────────────────────────────────────
    let num_workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(MAX_WORKERS);

    // Set Rayon's global pool size once, before any parallel work begins
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_workers)
        .build_global()?;

    // ── Derived configuration ──────────────────────────────────────────────────
    let num_batches = (NUM_RECORDS + BATCH_SIZE - 1) / BATCH_SIZE;
    let txn_width = NUM_RECORDS.to_string().len(); // 10 for 1 B records

    let est_ram_mb = BATCH_SIZE as f64 * 95.0 / 1_048_576.0;
    let est_file_gb = NUM_RECORDS as f64 * 85.0 / 1_073_741_824.0;

    // ── Startup banner ─────────────────────────────────────────────────────────
    let bar = "═".repeat(66);
    println!("\n{bar}");
    println!("  Records        : {:>20}", fmt_num(NUM_RECORDS));
    println!(
        "  Batch size     : {:>20}  ({} batches)",
        fmt_num(BATCH_SIZE),
        fmt_num(num_batches)
    );
    println!("  Workers        : {:>20}", num_workers);
    println!("  Est. RAM/batch : {:>18.0} MB", est_ram_mb);
    println!("  Est. file size : {:>18.1} GB", est_file_gb);
    println!("  Output         : {OUTPUT_FILE}");
    println!("{bar}\n");

    // ── One-time setup ─────────────────────────────────────────────────────────
    // Build date lookup table once; share read-only via & across threads.
    let date_table = build_date_table();

    fs::create_dir_all(OUTPUT_DIR)?;
    let file = fs::File::create(OUTPUT_FILE)?;
    // 8 MB write buffer keeps syscall count low regardless of batch size
    let mut writer = BufWriter::with_capacity(8 * 1024 * 1024, file);

    // CSV header
    writeln!(
        writer,
        "transaction_id,customer_id,product,amount,quantity,store,payment_method,date"
    )?;

    // ── Main batch loop ────────────────────────────────────────────────────────
    let t_start = Instant::now();
    let mut rows_written: u64 = 0;

    for batch_num in 0..num_batches {
        let t_batch = Instant::now();
        let batch_start = batch_num * BATCH_SIZE + 1;
        let batch_size = BATCH_SIZE.min(NUM_RECORDS - batch_num * BATCH_SIZE);

        // ── Step 1: parallel generation ────────────────────────────────────────
        // Each Rayon thread generates its sub-chunk into an independent Vec<u8>.
        // `date_table` is &-borrowed (Sync), so sharing across threads is safe.
        let tasks = build_sub_tasks(batch_start, batch_size, num_workers, batch_num);
        let chunks: Vec<Vec<u8>> = tasks
            .par_iter()
            .map(|t| generate_chunk(t.start_idx, t.count, t.seed, txn_width, &date_table))
            .collect();

        // ── Step 2: stream to disk (sequential, ordered) ───────────────────────
        for chunk in &chunks {
            writer.write_all(chunk)?;
        }

        // ── Step 3: release this batch before the next one starts ──────────────
        drop(chunks);

        // ── Progress line ──────────────────────────────────────────────────────
        rows_written += batch_size;
        let elapsed = t_start.elapsed().as_secs_f64();
        let batch_s = t_batch.elapsed().as_secs_f64();
        let rate = rows_written as f64 / elapsed.max(1e-9);
        let eta_s = (NUM_RECORDS - rows_written) as f64 / rate.max(1.0);
        let pct = rows_written as f64 / NUM_RECORDS as f64 * 100.0;

        println!(
            "  Batch {:>7}/{:<7}  {:>15} rows  ({:5.1}%)  {:6.2}s/batch  {:>7.0}k rows/s  ETA {}",
            fmt_num(batch_num + 1),
            fmt_num(num_batches),
            fmt_num(rows_written),
            pct,
            batch_s,
            rate / 1_000.0,
            fmt_duration(eta_s),
        );
    }

    writer.flush()?;

    // ── Final summary ──────────────────────────────────────────────────────────
    let total_s = t_start.elapsed().as_secs_f64();
    let file_bytes = fs::metadata(OUTPUT_FILE)?.len();
    let file_gb = file_bytes as f64 / 1_073_741_824.0;
    let avg_rate = rows_written as f64 / total_s.max(1e-9) / 1_000.0;

    println!("\n{bar}");
    println!("  Rows written   : {:>20}", fmt_num(rows_written));
    println!("  File size      : {:>18.2} GB", file_gb);
    println!("  Total time     : {:>20}", fmt_duration(total_s));
    println!("  Avg rate       : {:>16.0}k rows/s", avg_rate);
    println!("  Saved to       : {OUTPUT_FILE}");

    Ok(())
}
