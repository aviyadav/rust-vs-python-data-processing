#![allow(dead_code)]

use rand::Rng;
use std::fs;
use std::fs::File as StdFile;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Instant;

const LOG_LEVELS: &[&str] = &["INFO", "WARN", "ERROR", "DEBUG", "TRACE"];
const MESSAGES: &[&str] = &[
    "request processed successfully",
    "connection established",
    "timeout reached, retrying",
    "cache miss for key",
    "database query executed",
    "authentication passed",
    "file uploaded",
    "rate limit hit",
    "memory usage high",
    "service health check OK",
    "failed to parse payload",
    "retry attempt exhausted",
    "user session expired",
    "task queue drained",
    "config reloaded",
];
const MODULES: &[&str] = &["api", "db", "auth", "cache", "worker", "gateway", "storage"];

#[tokio::main]
async fn main() {
    let start = Instant::now();
    generate_log_files(10);
    let elapsed = start.elapsed();
    println!("Time taken: {:.2?}", elapsed);

    let logs_dir = Path::new("logs");
    if logs_dir.exists() && logs_dir.is_dir() {
        for entry in fs::read_dir(logs_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                // Process logs as streams asynchronously
                process_logs_streams(path.to_str().unwrap()).await;

                // Process logs normally (synchronously)
                // process_logs(path.to_str().unwrap());

            }
        }
    } else {
        println!("Logs directory does not exist. Please generate log files first.");
    }
}

fn generate_log_files(total_files: u32) {
    let logs_dir = Path::new("logs");
    fs::create_dir_all(logs_dir).unwrap();

    let start_hour = chrono_like_offset();
    let batch_size: u32 = 100;
    let batch_count = (total_files + batch_size - 1) / batch_size;

    let mut handles = Vec::with_capacity(batch_count as usize);

    for batch in 0..batch_count {
        let logs_dir: PathBuf = logs_dir.to_path_buf();
        let begin = batch * batch_size;
        let end = total_files.min(begin + batch_size);

        handles.push(thread::spawn(move || {
            let mut rng = rand::thread_rng();
            for i in begin..end {
                let hour_offset = start_hour + i as i64;
                let dt = hour_offset_to_datetime(hour_offset);
                let filename = format!("{}_{:02}.log", dt.date(), dt.hour);
                let path = logs_dir.join(&filename);

                let line_count: usize = rng.gen_range(10..=1000);
                let mut file = StdFile::create(&path).unwrap();

                let base_ts = hour_offset * 3600;

                for _ in 0..line_count {
                    let second: u32 = rng.gen_range(0..3600);
                    let ts = base_ts + second as i64;
                    let dt_line = seconds_to_datetime(ts);
                    let level = LOG_LEVELS[rng.gen_range(0..LOG_LEVELS.len())];
                    let module = MODULES[rng.gen_range(0..MODULES.len())];
                    let msg = MESSAGES[rng.gen_range(0..MESSAGES.len())];
                    let req_id: u64 = rng.gen_range(10000..99999);
                    let latency: u32 = rng.gen_range(1..500);

                    writeln!(
                        file,
                        "{} [{}] [{}] [req-{}] {} (latency: {}ms)",
                        dt_line, level, module, req_id, msg, latency
                    )
                    .unwrap();
                }
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!(
        "Generated {} log files in '{}'",
        total_files,
        logs_dir.display()
    );
}

fn process_logs(path: &str) {
    let file = StdFile::open(path).unwrap();
    let reader = BufReader::new(file);

    for line in reader.lines() {
        println!("Log: {}", line.unwrap());
    }
}

async fn process_logs_streams(path: &str) {
    use tokio::io::AsyncBufReadExt;
    let file = tokio::fs::File::open(path).await.unwrap();
    let mut reader = tokio::io::BufReader::new(file);
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).await.unwrap();
        if n == 0 {
            break;
        }
        print!("{}", line);
    }
}

// ── Minimal date-time helpers (no chrono dependency) ──

struct DateTime {
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    min: u32,
    sec: u32,
}

impl DateTime {
    fn date(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl std::fmt::Display for DateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.min, self.sec
        )
    }
}

/// Returns a "start hour" offset — an epoch-hour counter with some reasonable
/// base so that our 1000 hours start near 2024-01-01 00:00:00 UTC.
fn chrono_like_offset() -> i64 {
    // 2024-01-01 00:00:00 UTC is 1704067200 unix seconds → /3600 = 473352 hours
    473_352
}

fn hour_offset_to_datetime(offset_hours: i64) -> DateTime {
    seconds_to_datetime(offset_hours * 3600)
}

fn seconds_to_datetime(total_secs: i64) -> DateTime {
    let (year, month, day, hour, min, sec) = unix_to_ymdhms(total_secs);
    DateTime {
        year,
        month,
        day,
        hour,
        min,
        sec,
    }
}

/// Convert seconds since 1970-01-01 UTC to (year, month, day, hour, min, sec).
fn unix_to_ymdhms(mut ts: i64) -> (i32, u32, u32, u32, u32, u32) {
    let sec = (ts % 60) as u32;
    ts /= 60;
    let min = (ts % 60) as u32;
    ts /= 60;
    let hour = (ts % 24) as u32;
    ts /= 24; // days since epoch

    // Convert days since 1970-01-01 to year/month/day using a standard
    // algorithm (civil_from_days).
    let (year, month, day) = civil_from_days(ts as i32 + 719_468); // 719468 = days from 0000-03-01 to 1970-01-01
    (year, month, day, hour, min, sec)
}

/// Gregorian calendar: days since 0000-03-01 to (year, month, day).
/// Adapted from Howard Hinnant's date algorithms.
fn civil_from_days(z: i32) -> (i32, u32, u32) {
    let z = z - 60; // adjust from 0000-03-01 epoch to civil
    let era = if z >= 0 { z as u32 } else { (z as i64 + 146_097 * 400) as u32 } / 146_097;
    let doe = if z >= 0 {
        z as u32 - era * 146_097
    } else {
        (z as u32).wrapping_sub(era * 146_097)
    };
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d as u32)
}
