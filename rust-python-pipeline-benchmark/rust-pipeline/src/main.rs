use anyhow::{Context, Result};
use chrono::Local;
use duckdb::Connection;
use std::path::Path;
use std::time::Instant;
use sysinfo::System;

struct PipelineMetrics {
    start_time: Instant,
    peak_memory_mb: f64,
    system: System,
}

impl PipelineMetrics {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
            peak_memory_mb: 0.0,
            system: System::new_all(),
        }
    }

    fn update_memory(&mut self) {
        self.system.refresh_all();
        if let Some(process) = self.system.process(sysinfo::get_current_pid().unwrap()) {
            let memory_mb = process.memory() as f64 / 1024.0 / 1024.0;
            if memory_mb > self.peak_memory_mb {
                self.peak_memory_mb = memory_mb;
            }
        }
    }

    fn print_summary(&self) {
        let duration = self.start_time.elapsed();
        let duration_secs = duration.as_secs_f64();

        println!("\n{}", "=".repeat(60));
        println!("Pipeline Execution Summary (Rust + DuckDB)");
        println!("{}", "=".repeat(60));
        println!(
            "Duration: {:.2} seconds ({:.2} minutes)",
            duration_secs,
            duration_secs / 60.0
        );
        println!(
            "Peak Memory: {:.2} MB ({:.2} GB)",
            self.peak_memory_mb,
            self.peak_memory_mb / 1024.0
        );
        println!("{}", "=".repeat(60));
        println!();
    }
}

fn run_pipeline(data_dir: &str, output_path: &str) -> Result<()> {
    let mut metrics = PipelineMetrics::new();

    println!("\n{}", "=".repeat(60));
    println!("Starting Rust + DuckDB Pipeline");
    println!("Timestamp: {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
    println!("{}", "=".repeat(60));
    println!();

    // Connect to DuckDB (in-memory)
    println!("Initializing DuckDB...");
    let conn = Connection::open_in_memory().context("Failed to create DuckDB connection")?;

    metrics.update_memory();

    // Step 1: Load CSV files
    let abs_data_dir = std::fs::canonicalize(data_dir)
        .with_context(|| format!("Data directory not found: '{}'", data_dir))?;
    let abs_data_dir = abs_data_dir.to_string_lossy();
    println!("\nLoading CSV files from {}...", abs_data_dir);
    let csv_pattern = format!("{}/*.csv", abs_data_dir);

    conn.execute(
        &format!(
            "CREATE VIEW raw_data AS SELECT * FROM read_csv_auto('{}', ignore_errors=true)",
            csv_pattern
        ),
        [],
    )
    .context("Failed to load CSV files")?;

    metrics.update_memory();

    // Count total rows
    let row_count: i64 = conn.query_row("SELECT COUNT(*) FROM raw_data", [], |row| row.get(0))?;
    println!("Total rows loaded: {}", row_count);

    // Step 2: Clean data
    println!("\nCleaning data...");
    conn.execute(
        "CREATE VIEW cleaned_data AS
         SELECT * FROM raw_data
         WHERE product_id IS NOT NULL
           AND quantity > 0
           AND price > 0
           AND TRY_CAST(date AS DATE) IS NOT NULL",
        [],
    )
    .context("Failed to clean data")?;

    let cleaned_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM cleaned_data", [], |row| row.get(0))?;

    let removed = row_count - cleaned_count;
    let removed_pct = (removed as f64 / row_count as f64) * 100.0;
    println!("Removed {} invalid rows ({:.2}%)", removed, removed_pct);
    println!("Remaining rows: {}", cleaned_count);

    metrics.update_memory();

    // Step 3: Transform data
    println!("\nTransforming data...");
    conn.execute(
        "CREATE VIEW transformed_data AS
         SELECT
             *,
             quantity * price AS revenue,
             EXTRACT(YEAR FROM CAST(date AS DATE)) AS year,
             EXTRACT(MONTH FROM CAST(date AS DATE)) AS month,
             EXTRACT(QUARTER FROM CAST(date AS DATE)) AS quarter
         FROM cleaned_data",
        [],
    )
    .context("Failed to transform data")?;

    println!("Transformations complete");
    metrics.update_memory();

    // Step 4: Aggregate data
    println!("\nAggregating data...");
    conn.execute(
        "CREATE VIEW aggregated_data AS
         SELECT
             product_id,
             SUM(quantity) AS total_quantity,
             SUM(revenue) AS total_revenue,
             AVG(price) AS avg_price
         FROM transformed_data
         GROUP BY product_id
         ORDER BY total_revenue DESC",
        [],
    )
    .context("Failed to aggregate data")?;

    let agg_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM aggregated_data", [], |row| row.get(0))?;
    println!("Aggregated to {} products", agg_count);

    metrics.update_memory();

    // Step 5: Save results
    println!("\nSaving results to {}...", output_path);

    // Create output directory if it doesn't exist
    if let Some(parent) = Path::new(output_path).parent() {
        std::fs::create_dir_all(parent).context("Failed to create output directory")?;
    }

    conn.execute(
        &format!(
            "COPY aggregated_data TO '{}' (HEADER, DELIMITER ',')",
            output_path
        ),
        [],
    )
    .context("Failed to save results")?;

    let file_size = std::fs::metadata(output_path)?.len() as f64 / 1024.0 / 1024.0;
    println!("Results saved ({:.2} MB)", file_size);

    metrics.update_memory();
    metrics.print_summary();

    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let data_dir = args.get(1).map(|s| s.as_str()).unwrap_or("data");

    let output_path = args
        .get(2)
        .map(|s| s.as_str())
        .unwrap_or("results/rust_output.csv");

    match run_pipeline(data_dir, output_path) {
        Ok(_) => {
            println!("✅ Pipeline completed successfully");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("❌ Pipeline failed: {}", e);
            std::process::exit(1);
        }
    }
}
