use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A unit of work: an owned path handed from the walker to a worker.
///
/// Ownership of the path moves through the channel, so workers never
/// borrow data tied to the walker's stack frame.
#[derive(Debug)]
struct FileJob {
    path: PathBuf,
}

/// Result of processing a single file.
#[derive(Debug)]
struct FileResult {
    path: String,
    size: u64,
    checksum: String,
}

// ---------------------------------------------------------------------------
// Checksum
// ---------------------------------------------------------------------------

/// Compute a hex checksum for a byte slice.
///
/// Uses `DefaultHasher` (SipHash-1-3).  The hash is **not** guaranteed to
/// be stable across process invocations because the hasher is randomly
/// seeded.  For a deterministic checksum, swap in a fixed-seed hasher or
/// a dedicated crate like `sha2` / `xxhash`.
fn checksum(data: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

// ---------------------------------------------------------------------------
// Single-file processing
// ---------------------------------------------------------------------------

/// Read a file from disk and return its size and checksum.
fn process_job(job: FileJob) -> io::Result<FileResult> {
    let bytes = fs::read(&job.path)?;

    Ok(FileResult {
        path: job.path.to_string_lossy().to_string(),
        size: bytes.len() as u64,
        checksum: checksum(&bytes),
    })
}

// ---------------------------------------------------------------------------
// Multi-threaded directory processing
// ---------------------------------------------------------------------------

/// Counters describing a whole run.
#[derive(Debug, Default)]
struct Summary {
    scanned: u64,
    processed: u64,
    skipped: u64,
    failed: u64,
    bytes_read: u64,
}

/// Format a byte count using binary units (e.g. `96.4 GiB`).
fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// Format a duration compactly (e.g. `7m 42s`, `3.2s`).
fn human_duration(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    if secs >= 60 {
        format!("{}m {:02}s", secs / 60, secs % 60)
    } else {
        format!("{d:.1?}")
    }
}

/// Recursively process every file under `root` using a pool of `num_workers`
/// OS threads connected by two MPSC channels.
///
/// # Architecture
///
/// ```text
///   walker thread                 worker pool              main thread
///   ─────────────                ────────────             ────────────
///   walkdir::WalkDir ──┐        ┌─ worker 0 ──┐
///                      │        │              │
///                      ├─ path_tx ─── path_rx ─┼─ worker 1 ──┐
///                      │        │    (Arc<Mutex>)            │
///                      │        └─ worker N ──┘              │
///                      │              │                       │
///                      │           result_tx ──── result_rx ──┤
///                      │                                      │
///                      ▼                                      ▼
///               path_tx dropped                         Vec<FileResult>
///               → workers exit
/// ```
fn process_directory(root: &Path, num_workers: usize) -> io::Result<(Vec<FileResult>, Summary)> {
    // ---- channels ----------------------------------------------------------

    // Paths to process: walker → workers.  The queue is *bounded* so the
    // walker cannot enqueue paths faster than workers drain them — this
    // applies backpressure and keeps memory flat on huge directory trees.
    const QUEUE_CAPACITY: usize = 1024;
    let (job_tx, job_rx) = mpsc::sync_channel::<FileJob>(QUEUE_CAPACITY);

    // Results and errors travel on separate channels: one corrupt or
    // unreadable file must not kill the whole run.
    let (result_tx, result_rx) = mpsc::channel::<FileResult>();
    let (error_tx, error_rx) = mpsc::channel::<io::Error>();

    // `Receiver` is not `Clone`, so we wrap it in `Arc<Mutex<>>` so every
    // worker can safely pull work items from the shared queue.
    let job_rx = Arc::new(Mutex::new(job_rx));

    // ---- spawn worker threads ----------------------------------------------

    let mut workers = Vec::with_capacity(num_workers);

    for id in 0..num_workers {
        let job_rx = Arc::clone(&job_rx);
        let result_tx = result_tx.clone();
        let error_tx = error_tx.clone();

        workers.push(
            thread::Builder::new()
                .name(format!("worker-{id}"))
                .spawn(move || {
                    loop {
                        // Lock the shared receiver just long enough to pop one
                        // job, then release the lock so other workers can
                        // proceed.
                        let job = {
                            let rx = job_rx.lock().unwrap();
                            match rx.recv() {
                                Ok(j) => j,
                                Err(_) => break, // channel closed — no more work
                            }
                        };

                        // Process the file and send the outcome back.  If the
                        // collector has hung up there is nothing left to do.
                        let sent = match process_job(job) {
                            Ok(result) => result_tx.send(result).is_ok(),
                            Err(e) => error_tx.send(e).is_ok(),
                        };
                        if !sent {
                            break;
                        }
                    }
                })?,
        );
    }

    // Drop our *own* clones of the senders so that the receivers will close
    // once every worker has dropped theirs.
    drop(result_tx);
    drop(error_tx);

    // ---- spawn walker thread -----------------------------------------------

    let root = root.to_path_buf();

    let walker = thread::Builder::new()
        .name("walker".into())
        .spawn(move || {
            let mut scanned = 0u64;
            let mut skipped = 0u64;
            for entry in walkdir::WalkDir::new(&root) {
                match entry {
                    Ok(entry) if entry.file_type().is_file() => {
                        scanned += 1;
                        // If the workers have all hung up, stop walking.
                        let job = FileJob {
                            path: entry.path().to_path_buf(),
                        };
                        if job_tx.send(job).is_err() {
                            break;
                        }
                    }
                    Ok(_) => skipped += 1, // directories, symlinks, etc.
                    Err(e) => {
                        skipped += 1;
                        eprintln!("walkdir error: {e}");
                    }
                }
            }
            (scanned, skipped)
            // `job_tx` is dropped here → channel closes → workers break.
        })?;

    // ---- drain results -----------------------------------------------------

    let mut results = Vec::new();
    let mut failed = 0u64;

    // Drain both channels until every worker has hung up.  Once both
    // receivers report `Err`, all senders are gone and the run is over.
    let mut results_open = true;
    let mut errors_open = true;
    while results_open || errors_open {
        if results_open {
            match result_rx.recv() {
                Ok(r) => {
                    results.push(r);
                    continue;
                }
                Err(_) => results_open = false,
            }
        }
        if errors_open {
            match error_rx.recv() {
                Ok(e) => {
                    failed += 1;
                    eprintln!("processing error: {e}");
                }
                Err(_) => errors_open = false,
            }
        }
    }

    // Wait for the walker to exhaust the directory tree.
    let (scanned, skipped) = walker.join().unwrap();

    // ---- join workers (belt-and-suspenders) --------------------------------

    for worker in workers {
        worker.join().unwrap();
    }

    let summary = Summary {
        scanned,
        processed: results.len() as u64,
        skipped,
        failed,
        bytes_read: results.iter().map(|r| r.size).sum(),
    };

    Ok((results, summary))
}

// ---------------------------------------------------------------------------
// CLI entry point
// ---------------------------------------------------------------------------

fn main() {
    let args: Vec<String> = env::args().collect();

    // Default worker count = logical CPUs on the machine.
    let default_workers = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    let (root, num_workers) = match args.len() {
        2 => (PathBuf::from(&args[1]), default_workers),
        3 => (
            PathBuf::from(&args[1]),
            args[2].parse::<usize>().unwrap_or(default_workers),
        ),
        _ => {
            eprintln!("Usage: {} <directory> [num-workers]", args[0]);
            std::process::exit(1);
        }
    };

    if !root.is_dir() {
        eprintln!("Error: '{}' is not a directory", root.display());
        std::process::exit(1);
    }

    eprintln!(
        "Scanning '{}' with {num_workers} worker{}…",
        root.display(),
        if num_workers == 1 { "" } else { "s" },
    );

    let started = Instant::now();

    match process_directory(&root, num_workers) {
        Ok((results, summary)) => {
            let elapsed = started.elapsed();

            for r in &results {
                println!("{:>12}  {}  {}", r.size, r.checksum, r.path);
            }

            println!();
            println!("files scanned:   {:>12}", summary.scanned);
            println!("files processed: {:>12}", summary.processed);
            println!("files skipped:   {:>12}", summary.skipped);
            println!("files failed:    {:>12}", summary.failed);
            println!("bytes read:      {:>12}", human_bytes(summary.bytes_read));
            println!("elapsed:         {:>12}", human_duration(elapsed));
        }
        Err(e) => {
            eprintln!("Fatal error: {e}");
            std::process::exit(1);
        }
    }
}
