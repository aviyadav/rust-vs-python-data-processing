# Multi-Threaded File Processor

A high-throughput Rust CLI tool that recursively scans a directory tree and processes every file in parallel using a configurable pool of OS threads. It reports the **size** and a **checksum** for each file, along with aggregate timing statistics.

## Architecture

```
  walker thread                 worker pool                main thread
  ─────────────                ────────────               ────────────
  walkdir::WalkDir ──┐        ┌─ worker 0 ──┐
                     │        │              │
                     ├─ job_tx ──── job_rx ──┼─ worker 1 ──┐
                     │ (bounded)  (Arc<Mutex>)│             │
                     │        └─ worker N ──┘              │
                     │              │ result_tx ─── result_rx
                     │              │ error_tx ───── error_rx
                     ▼                                      ▼
              job_tx dropped                        Vec<FileResult>
              → workers exit                        + Summary counters
```

- **Jobs, not borrows** — the walker wraps each discovered path in an owned `FileJob { path: PathBuf }`, so ownership moves through the channel and workers never borrow data tied to another thread's stack frame.
- **Bounded job queue** — jobs travel on a `sync_channel` with a capacity of 1024. If workers fall behind, the walker blocks, applying backpressure so memory stays flat even on huge directory trees.
- **Worker pool** — `N` OS threads share the receiver behind an `Arc<Mutex<>>`. Each worker pops one job at a time, reads the file, and computes a checksum.
- **Separate result and error channels** — successes go to `result_rx`, failures to `error_rx`. One unreadable file is counted and reported instead of aborting the run.
- **Main thread** — drains both channels until every worker hangs up, prints the per-file listing and an end-of-run summary, and joins all threads.

## Checksum

The tool uses `std::hash::DefaultHasher` (SipHash-1-3), formatted as a 16-character hex string. This hash is seeded per-process, so the same file on the same machine may produce a different checksum across invocations. For a deterministic checksum, swap in a fixed-seed hasher or a dedicated crate such as `sha2` or `xxhash`.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) **1.60+** (edition 2021)

## Build

```sh
cargo build --release
```

The optimized binary lands at `target/release/multi-threaded-file-processor` (or `.exe` on Windows).

## Usage

```sh
multi-threaded-file-processor <directory> [num-workers]
```

| Argument        | Required | Description                                                                 |
|-----------------|----------|-----------------------------------------------------------------------------|
| `<directory>`   | yes      | Root directory to scan recursively.                                         |
| `[num-workers]` | no       | Number of worker threads. Defaults to the number of logical CPUs available. |

### Examples

```sh
# Process all files in ./data with the default thread count
multi-threaded-file-processor ./data

# Use exactly 8 worker threads
multi-threaded-file-processor ./large-archive 8

# Scan a project tree with a single thread (useful for deterministic ordering)
multi-threaded-file-processor ./src 1
```

### Output

Each file is printed on a single line with **size** (bytes, right-aligned), **checksum**, and **path**:

```
        4096  3f8a1b2c9d0e4a7b  ./docs/readme.txt
       12345  a1b2c3d4e5f67890  ./images/photo.png
         512  0123456789abcdef  ./config.json

files scanned:              3
files processed:            3
files skipped:              0
files failed:               0
bytes read:           17.5 KiB
elapsed:             142.5ms
```

The summary reports **scanned** (files found by the walker), **processed** (successfully hashed), **skipped** (directories, symlinks, and walk errors), **failed** (files that could not be read), total **bytes read** in binary units (KiB/MiB/GiB), and wall-clock **elapsed** time.

Progress and errors are written to **stderr**; file results are written to **stdout**, making it easy to redirect:

```sh
multi-threaded-file-processor ./data > file-inventory.txt
```

## Project structure

```
.
├── Cargo.toml          # Crate manifest (depends on walkdir)
├── Cargo.lock
├── README.md
└── src/
    └── main.rs         # Entire application
```

## License

This project is provided as-is for educational and utility purposes.
