# Rust + DuckDB Data Pipeline

A high-performance CSV data processing pipeline built with Rust and DuckDB. It reads raw CSV sales data, cleans and transforms it in-memory using DuckDB's SQL engine, aggregates the results by product, and writes the final output to a CSV file — while tracking execution time and peak memory usage throughout.

---

## Table of Contents

- [Overview](#overview)
- [Pipeline Steps](#pipeline-steps)
- [Project Structure](#project-structure)
- [Dependencies](#dependencies)
- [Prerequisites](#prerequisites)
- [Dependency Fixes Required Before Building](#dependency-fixes-required-before-building)
- [Build Instructions](#build-instructions)
- [Running the Pipeline](#running-the-pipeline)
- [Input Format](#input-format)
- [Output Format](#output-format)
- [Release Profile](#release-profile)

---

## Overview

The pipeline is designed to benchmark Rust + DuckDB against equivalent Python-based data pipelines. It uses DuckDB's in-memory SQL engine to efficiently process large CSV datasets with minimal memory overhead, and `sysinfo` to report peak RSS memory usage at each stage.

---

## Pipeline Steps

The pipeline executes five sequential steps:

| Step | Name        | Description                                                                                     |
|------|-------------|-------------------------------------------------------------------------------------------------|
| 1    | **Load**    | Reads all `*.csv` files from the input directory into a DuckDB view using `read_csv_auto`       |
| 2    | **Clean**   | Filters out rows where `product_id` is NULL, `quantity <= 0`, `price <= 0`, or `date` is invalid |
| 3    | **Transform** | Computes `revenue = quantity * price` and extracts `year`, `month`, and `quarter` from `date`  |
| 4    | **Aggregate** | Groups by `product_id`, computing `total_quantity`, `total_revenue`, and `avg_price`           |
| 5    | **Save**    | Writes the aggregated results to a CSV file; creates the output directory if it does not exist  |

At the end of the run a summary is printed showing total wall-clock duration and peak memory in MB and GB.

---

## Project Structure

```
rust-pipeline/
├── src/
│   └── main.rs        # Full pipeline implementation
├── Cargo.toml         # Package manifest and dependencies
├── Cargo.lock         # Pinned dependency versions (committed)
└── .gitignore         # Excludes /target from version control
```

---

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| [`duckdb`](https://crates.io/crates/duckdb) | `1.10502.0` | In-memory SQL engine for loading, transforming, and aggregating CSV data |
| [`anyhow`](https://crates.io/crates/anyhow) | `1.0` | Ergonomic error handling with context propagation |
| [`chrono`](https://crates.io/crates/chrono) | `0.4` | Timestamp formatting for the pipeline start message |
| [`sysinfo`](https://crates.io/crates/sysinfo) | `0.30` | Cross-platform process memory tracking (peak RSS) |

The `duckdb` crate is built with the **`bundled`** feature, which statically compiles DuckDB from C++ source during `cargo build`. No system-level DuckDB installation is required.

---

## Prerequisites

### 1. Install Rust

Rust is not installed by the system package manager. Install it using the official `rustup` installer:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

When prompted, press **1** to accept the default installation. This installs `rustup`, `rustc`, and `cargo`.

### 2. Load Rust into the current shell session

```bash
source "$HOME/.cargo/env"
```

> This is only needed once per terminal session. Future sessions load it automatically via `~/.bashrc` or `~/.profile`.

### 3. Verify the installation

```bash
rustc --version
cargo --version
```

Both commands should print a version string such as `rustc 1.x.x` and `cargo 1.x.x`.

### 4. C++ build toolchain (for bundled DuckDB)

Because `duckdb` is compiled from C++ source, a working C/C++ compiler must be present:

```bash
# Debian / Ubuntu
sudo apt install build-essential

# Fedora / RHEL
sudo dnf install gcc gcc-c++ make
```

---

## Dependency Fixes Required Before Building

The original `Cargo.toml` shipped with three bugs that prevent a successful build. All three have already been applied in the current `Cargo.toml`, but they are documented here for reference.

### Fix 1 — `sysinfo`: remove deleted trait imports

**File:** `src/main.rs`

`sysinfo` **0.29** removed the `ProcessExt` and `SystemExt` traits. Their methods are now available directly on `System` and `Process` without any trait import.

```diff
- use sysinfo::{ProcessExt, System, SystemExt};
+ use sysinfo::System;
```

### Fix 2 — `duckdb`: upgrade from `0.10` to `1.10502.0`

**File:** `Cargo.toml`

Two separate problems were both fixed by upgrading `duckdb`:

**a) `chrono` vs `arrow-arith 51` method conflict (`E0034`)**

`duckdb 0.10` depended on `arrow-arith 51.0.0`, which defined its own `ChronoDateExt::quarter()` method. `chrono 0.4.38+` added `Datelike::quarter()`. Having both traits in scope caused an irresolvable ambiguity error in `arrow-arith`'s own source:

```
error[E0034]: multiple applicable items in scope
  --> arrow-arith-51.0.0/src/temporal.rs:90:36
   |
   |  DatePart::Quarter => |d| d.quarter() as i32,
   |                               ^^^^^^^ multiple `quarter` found
```

`duckdb 1.10502.0` pulls in `arrow-arith 58`, where this conflict was resolved upstream.

**b) Bundled C++ build failure on GCC 14+ (`uint8_t` undeclared)**

`duckdb 0.10.2`'s bundled C++ source (`libfsst.hpp`) was missing `#include <cstdint>`. GCC 14+ no longer provides `uint8_t`, `uint16_t`, etc. as implicit includes, so the build failed with hundreds of errors like:

```
error: 'uint8_t' does not name a type
note: 'uint8_t' is defined in header '<cstdint>'; this is probably fixable by adding '#include <cstdint>'
```

`duckdb 1.10502.0`'s bundled source is fully GCC 14/15 compatible.

```diff
- duckdb = "0.10"
+ duckdb = { version = "1.10502.0", features = ["bundled"] }
```

### Fix 3 — `duckdb`: enable the `bundled` feature

**File:** `Cargo.toml`

Without `features = ["bundled"]`, the `duckdb` crate attempts to dynamically link against a system-installed `libduckdb.so`. If that library is not present, the linker fails at the very end of compilation:

```
error: linking with `cc` failed
  = note: rust-lld: error: unable to find library -lduckdb
```

Enabling `bundled` compiles DuckDB from source and links it statically — no system library is needed.

```diff
- duckdb = "0.10"
+ duckdb = { version = "1.10502.0", features = ["bundled"] }
```

---

## Build Instructions

### Release build (recommended)

```bash
cargo build --release
```

> The first build is slow (~5–10 minutes) because DuckDB's C++ source (~6 MB) is compiled from scratch. Subsequent builds are fast since the compiled artifact is cached in `target/`.

### Limit parallel compile jobs (lower CPU/RAM usage)

```bash
cargo build --release -j 2
```

### Debug build (faster compile, slower binary)

```bash
cargo build
```

---

## Running the Pipeline

The compiled binary accepts two optional positional arguments:

```bash
./target/release/rust-pipeline [DATA_DIR] [OUTPUT_PATH]
```

| Argument | Default | Description |
|---|---|---|
| `DATA_DIR` | `data` | Directory containing input `*.csv` files |
| `OUTPUT_PATH` | `results/rust_output.csv` | Path for the aggregated output CSV |

> **Important:** Always run the binary from the `rust-pipeline/` directory (the project root where `Cargo.toml` lives). Both arguments accept relative or absolute paths.

### Recommended — run the pre-built binary directly

Build once, then invoke the binary directly for every subsequent run. This avoids the overhead of cargo checking for recompilation.

```bash
# Step 1 — build (only needed once, or after code changes)
cargo build --release -j 2

# Step 2 — run
./target/release/rust-pipeline ../data ../results/rust-pipeline-output.csv
```

### Alternative — build and run in one step with `cargo run`

When using `cargo run`, the `--` separator is **required** to distinguish cargo's own flags from the arguments passed to the binary.

```bash
cargo run --release -j 2 -- ../data ../results/rust-pipeline-output.csv
```

### Using absolute paths

Absolute paths always work regardless of which directory you run from:

```bash
./target/release/rust-pipeline /home/avinash/codebase/rust-corner/rust-python-pipeline-benchmark/data /home/avinash/codebase/rust-corner/rust-python-pipeline-benchmark/results/rust-pipeline-output.csv
```

### Use defaults (no arguments)

If no arguments are given the binary reads from `./data` and writes to `./results/rust_output.csv` relative to the current working directory:

```bash
./target/release/rust-pipeline
```

### Path resolution notes

- Relative paths such as `../data` are resolved by the program to their absolute form before being passed to DuckDB. DuckDB cannot expand glob patterns like `*.csv` in paths that contain `..`, so this conversion is done automatically.
- If the output directory does not exist it is created automatically.
- If the data directory does not exist the program exits immediately with a clear error: `Data directory not found: '<path>'`.

### Example output

```
============================================================
Starting Rust + DuckDB Pipeline
Timestamp: 2026-05-17 22:18:23
============================================================

Initializing DuckDB...

Loading CSV files from /home/avinash/codebase/rust-corner/rust-python-pipeline-benchmark/data...
Total rows loaded: 536870800

Cleaning data...
Removed 26843400 invalid rows (5.00%)
Remaining rows: 510027400

Transforming data...
Transformations complete

Aggregating data...
Aggregated to 1000 products

Saving results to ../results/rust-pipeline-output.csv...
Results saved (0.05 MB)

============================================================
Pipeline Execution Summary (Rust + DuckDB)
============================================================
Duration: 45.79 seconds (0.76 minutes)
Peak Memory: 2566.59 MB (2.51 GB)
============================================================

✅ Pipeline completed successfully
```

---

## Input Format

The pipeline expects CSV files with at least the following columns (additional columns are allowed and ignored):

| Column | Type | Validation |
|---|---|---|
| `product_id` | any | Must not be NULL |
| `quantity` | numeric | Must be `> 0` |
| `price` | numeric | Must be `> 0` |
| `date` | date string | Must be castable to `DATE` |

Multiple CSV files in the input directory are automatically combined by DuckDB's `read_csv_auto` glob reader.

---

## Output Format

The output is a CSV file with one row per `product_id`, sorted by `total_revenue` descending:

| Column | Description |
|---|---|
| `product_id` | Product identifier |
| `total_quantity` | Sum of all valid quantities |
| `total_revenue` | Sum of `quantity * price` across all rows |
| `avg_price` | Average unit price across all rows |

---

## Release Profile

The `Cargo.toml` release profile is tuned for maximum runtime performance at the cost of longer compile times:

| Setting | Value | Effect |
|---|---|---|
| `opt-level` | `3` | Full optimisation (default for release, stated explicitly) |
| `lto` | `true` | Link-time optimisation across all crates |
| `codegen-units` | `1` | Single codegen unit — enables maximum inlining across the whole binary |
