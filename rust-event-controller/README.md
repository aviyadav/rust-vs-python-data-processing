# rust-event-controller

A Rust application that tails log files from a watched folder, enriches each line with a UTC timestamp, aggregates them into a staging file, and periodically flushes them in batches to an output folder. Read progress is persisted to disk so the process can safely resume from exactly where it left off after a restart or crash.

---

## Table of Contents

- [Overview](#overview)
- [How It Works](#how-it-works)
- [Project Structure](#project-structure)
- [Configuration](#configuration)
- [Output File Naming](#output-file-naming)
- [Prerequisites](#prerequisites)
- [Compiling](#compiling)
- [Running](#running)
- [First Run vs. Subsequent Runs](#first-run-vs-subsequent-runs)
- [Dependencies](#dependencies)

---

## Overview

`rust-event-controller` is designed to act as a lightweight log-forwarding agent. It watches a local folder for log files, reads them line by line (picking up from the last known position), enriches every line with the current UTC timestamp, and writes the enriched output into rotated batch files. The batch files are named with Kubernetes-style cluster/namespace/pod/container metadata and a unique UUID, making them suitable for ingestion by a downstream log pipeline.

---

## How It Works

```
log_files/          (source log files)
     │
     │  read line-by-line, skipping already-processed lines
     ▼
data/log_aggregated_file.txt   (temporary staging file)
     │
     │  every N lines (rotation_threshold), flush staging → output
     ▼
flush_folder/       (enriched, rotated output batches)
```

1. **Config loading** — `config/config.json` is read at startup to obtain all paths and tuning parameters.
2. **Counter recovery** — `data/counter.txt` is read to build a map of `{ file_path → last_line_processed }`. On the very first run, if the file does not exist, all counters default to `0`.
3. **Tailing** — Every file inside `tail_folder` is processed sequentially. Lines already accounted for by the counter are skipped; only new lines are processed.
4. **Enrichment** — Each new line is prepended with the current UTC timestamp and appended to the aggregate staging file.
5. **Counter update** — After each line is written, the running line number is appended to `counter.txt` so progress is never lost.
6. **Rotation / Flush** — When the number of lines processed reaches a multiple of `rotation_threshold`, the staging file is moved to `flush_folder` as a new batch file and then deleted. Any remaining lines at end-of-file are also flushed.
7. **Audit trail** — Every flush event is recorded in `data/flush_counter.txt` as `source_file - output_file`.

### Enriched line format

```
2026-04-24 21:26:01.753813367 UTC - <original log line content>
```

---

## Project Structure

```
rust-event-controller/
├── config/
│   └── config.json          # Application configuration
├── data/
│   ├── counter.txt           # Per-file line counters (resume state)
│   └── flush_counter.txt     # Audit log of all flush operations
├── flush_folder/             # Output destination for enriched log batches
├── log_files/                # Source log files to be tailed
├── src/
│   ├── main.rs               # Entry point – orchestrates tailing loop
│   ├── config.rs             # Config struct + JSON deserialisation
│   ├── counter.rs            # Counter file read/write helpers
│   ├── flush.rs              # Batch flush logic + output file creation
│   └── util.rs               # Line enrichment + counter append helpers
├── Cargo.toml
└── Cargo.lock
```

---

## Configuration

All configuration lives in `config/config.json`:

```json
{
    "counter_file":    "./data/counter.txt",
    "aggregate_file":  "./data/log_aggregated_file.txt",
    "tail_folder":     "./log_files/",
    "audit_file":      "./data/flush_counter.txt",
    "flush": {
        "mode":                "folder",
        "location":            "./flush_folder/",
        "rotation_threshold":  5
    }
}
```

| Field | Type | Description |
|---|---|---|
| `counter_file` | string | Path to the file that persists per-file line counters. Created automatically on first flush. |
| `aggregate_file` | string | Path to the temporary staging file where enriched lines accumulate between flushes. |
| `tail_folder` | string | Directory containing the source log files to tail. All files in this folder are processed. |
| `audit_file` | string | Path to the audit log that records every source→output flush mapping. |
| `flush.mode` | string | Flush mode. Currently `"folder"` (writes batches to `flush.location`). |
| `flush.location` | string | Directory where enriched batch files are written. |
| `flush.rotation_threshold` | integer | Number of lines after which a flush is triggered. A final flush is always performed at end-of-file for any remaining lines. |

> **Tip:** All paths are relative to the working directory from which the binary is executed. Run the binary from the project root to match the defaults above.

---

## Output File Naming

Batch files written to `flush_folder` follow a URL-encoded naming convention that embeds Kubernetes metadata:

```
{location}{cluster}%2F{namespace}%2F{pod}%2F{container}-{filename}-{uuid}
```

| Segment | Meaning |
|---|---|
| `cluster` | Cluster name (currently hardcoded: `cluster0`) |
| `namespace` | Kubernetes namespace (currently hardcoded: `namespace0`) |
| `pod` | Pod name (currently hardcoded: `pod0`) |
| `container` | Container name (currently hardcoded: `container0`) |
| `filename` | Original source filename with `.` URL-encoded as `%2E` |
| `uuid` | A randomly generated UUIDv4 ensuring uniqueness per batch |
| `%2F` | URL-encoded `/` separator between metadata segments |

**Example:**
```
./flush_folder/cluster0%2Fnamespace0%2Fpod0%2Fcontainer0-event_1%2Etxt-650116db-dafa-4559-bcaf-ed3c810cffc5
```

> The cluster, namespace, pod, and container values are defined as constants in `src/flush.rs` and should be updated (or replaced with environment variable reads) to reflect the actual deployment environment.

---

## Prerequisites

- **Rust toolchain** — Install via [rustup](https://rustup.rs/):
  ```
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
  Minimum supported edition: **Rust 2024** (Rust 1.85+).

- **C linker**
  - **Linux / macOS** — `gcc` or `clang` (usually pre-installed or via `build-essential` / Xcode CLT).
  - **Windows** — [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (MSVC toolchain) or use the `x86_64-pc-windows-gnu` target with MinGW.

- **Required directories** — The following directories must exist before the first run (they are not created automatically):
  ```
  mkdir -p data flush_folder log_files
  ```

- **Source log files** — Place any plain-text log files you want to process inside `log_files/`. Each line in a file is treated as one log event.

---

## Compiling

### Debug build
```
cargo build
```
The binary is written to `target/debug/rust-event-controller`.

### Release build (optimised)
```
cargo build --release
```
The binary is written to `target/release/rust-event-controller`.

---

## Running

Always run the binary from the **project root directory** so that the relative paths in `config/config.json` resolve correctly.

### Using Cargo (debug)
```
cargo run
```

### Using Cargo (release)
```
cargo run --release
```

### Running the compiled binary directly

**Linux / macOS:**
```
./target/release/rust-event-controller
```

**Windows:**
```
.\target\release\rust-event-controller.exe
```

### Expected console output
```
COUNTER LOCATION FOR FILE - ./log_files/event_1.txt is 0
COUNTER LOCATION FOR FILE - ./log_files/event_2.txt is 0
Log File Flushed - ./flush_folder/cluster0%2Fnamespace0%2Fpod0%2Fcontainer0-event_1%2Etxt-<uuid>
Log File Flushed - ./flush_folder/cluster0%2Fnamespace0%2Fpod0%2Fcontainer0-event_2%2Etxt-<uuid>
```

---

## First Run vs. Subsequent Runs

| Scenario | Behaviour |
|---|---|
| **First run** – `data/counter.txt` does not exist | All files in `log_files/` are processed from line 1. A message is printed noting that a fresh start is being made. |
| **Subsequent run** – `data/counter.txt` exists | Each file is resumed from its last recorded line number. Already-processed lines are skipped. |
| **New file added** to `log_files/` | Processed from line 1 (no entry in counter map yet). |
| **Source file unchanged** since last run | All lines are skipped; no output is produced. |

---

## Dependencies

| Crate | Version | Purpose |
|---|---|---|
| [`serde`](https://crates.io/crates/serde) | 1.0 | Serialisation framework (with `derive` feature for `#[derive(Deserialize)]`) |
| [`serde_json`](https://crates.io/crates/serde_json) | 1.0 | JSON deserialisation of `config.json` |
| [`chrono`](https://crates.io/crates/chrono) | 0.4 | UTC timestamp generation for log line enrichment |
| [`uuid`](https://crates.io/crates/uuid) | 1.8 | UUIDv4 generation for unique output batch filenames |
| [`rand`](https://crates.io/crates/rand) | 0.10 | Fast random number generation (used by `uuid`) |
| [`regex`](https://crates.io/crates/regex) | 1.12 | Regular expression support (available for future use) |
| [`tokio`](https://crates.io/crates/tokio) | 1.52 | Async runtime (available for future async/concurrent tailing) |