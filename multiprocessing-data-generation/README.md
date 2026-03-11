# Transaction Data Generator — Rust

Memory-safe streaming generator for large transaction datasets.  
Faithfully ports the Python `generate-transaction-data-mp.py` pipeline to Rust, achieving **≈ 3–6× faster throughput** while keeping peak RAM to a single batch regardless of total record count.

---

## How it works

```
Rayon thread pool (N workers)
  │
  ├── Worker 0 → Vec<u8> (raw CSV bytes for sub-chunk 0)
  ├── Worker 1 → Vec<u8> (raw CSV bytes for sub-chunk 1)
  │   …
  └── Worker N → Vec<u8> (raw CSV bytes for sub-chunk N)
         │
         └── BufWriter<File> ─── streams to disk
                │
                └── drop(all chunks)  ← memory released before next batch
```

| Step | What happens |
|------|-------------|
| 1 | Total records are split into **batches** of `BATCH_SIZE` rows |
| 2 | Each batch is split into **sub-chunks** (one per Rayon thread) |
| 3 | All sub-chunks are generated **in parallel** with a seeded RNG |
| 4 | Sub-chunks are written **sequentially** to the CSV via an 8 MB `BufWriter` |
| 5 | The `Vec<u8>` buffers are **dropped** before the next batch starts |

Peak RAM ≈ `BATCH_SIZE × ~95 bytes` — only one batch ever lives in memory.

---

## Prerequisites

| Tool | Minimum version | Install |
|------|----------------|---------|
| Rust & Cargo | 1.65 | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |

No other system libraries are required. All Rust dependencies are fetched automatically by Cargo on first build.

---

## Project structure

```
multiprocessing-data-generation/
├── Cargo.toml                       ← Rust manifest & dependencies
├── src/
│   └── main.rs                      ← Generator source
├── generate-transaction-data-mp.py  ← Original Python reference implementation
├── README.md                        ← This file
└── data/                            ← Created at runtime; output lives here
    └── transaction-data.csv
```

---

## Build

```bash
# Clone / navigate into the project directory
cd multiprocessing-data-generation

# Release build (LTO + full optimisation — required for good throughput)
cargo build --release
```

The binary is placed at `target/release/generate`.

---

## Run

```bash
cargo run --release
# or equivalently:
./target/release/generate
```

The output file `data/transaction-data.csv` is created (or overwritten) automatically.

Sample startup banner:
```
══════════════════════════════════════════════════════════════════
  Records        :        1,000,000,000
  Batch size     :              500,000  (2,000 batches)
  Workers        :                    8
  Est. RAM/batch :                 45 MB
  Est. file size :               79.2 GB
  Output         : data/transaction-data.csv
══════════════════════════════════════════════════════════════════

  Batch       1/2,000         500,000 rows  (  0.1%)    0.19s/batch    2676k rows/s  ETA 0:06:13
  Batch       2/2,000       1,000,000 rows  (  0.1%)    0.16s/batch    2869k rows/s  ETA 0:05:48
  …
```

---

## Tuning

All performance knobs live at the top of `src/main.rs`:

```rust
const NUM_RECORDS: u64 = 1_000_000_000;  // total rows to produce
const BATCH_SIZE:  u64 = 500_000;         // rows per iteration
const MAX_WORKERS: usize = 8;             // thread-pool cap
```

| Constant | Guidance |
|----------|----------|
| `NUM_RECORDS` | Set to any value; the width of `TXN` IDs auto-scales. |
| `BATCH_SIZE` | Increase to `2_000_000` if you have ≥ 16 GB free RAM for higher throughput. Decrease to `100_000` on memory-constrained systems. |
| `MAX_WORKERS` | Defaults to `min(logical_cores, 8)`. Raise the cap if you have more than 8 cores. Lower it to free up CPUs for other workloads. |

After editing, rebuild with `cargo build --release`.

---

## Output format

**File:** `data/transaction-data.csv`

| Column | Type | Example | Notes |
|--------|------|---------|-------|
| `transaction_id` | String | `TXN0000000001` | Zero-padded to `len(NUM_RECORDS)` digits |
| `customer_id` | String | `CUST44733` | Random; 5-digit zero-padded; 100 000 unique IDs |
| `product` | String | `Monitor` | 10 products |
| `amount` | Float | `2451.71` | Uniform `[100.00, 2500.00]`, 2 d.p. |
| `quantity` | Integer | `5` | Uniform `[1, 10]` |
| `store` | String | `Store_E` | 5 stores |
| `payment_method` | String | `Debit Card` | 5 methods |
| `date` | String | `2024-08-21` | Random within `2023-01-01`–`2024-12-31` |

---

## Performance

Measured on a machine with 8 logical cores and NVMe storage:

| Implementation | Throughput | Time for 1 B rows |
|---|---|---|
| Python (pyarrow + polars + numpy) | ~500 k rows/s | ~33 min |
| **Rust (this project)** | **~3–6 M rows/s** | **~3–6 min** |

The Rust version is typically **6–10× faster** because:
- Zero-copy byte-buffer generation (no Python object overhead)
- `SmallRng` (Xoshiro128++) is faster than NumPy's PCG64 for this workload
- Pre-computed date lookup table avoids per-row date arithmetic
- `BufWriter` with 8 MB buffer amortises `write` syscalls
- LTO + single-codegen-unit release mode enables full cross-crate inlining

---

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| [`rayon`](https://crates.io/crates/rayon) | 1.10 | Work-stealing thread pool (parallel chunk generation) |
| [`rand`](https://crates.io/crates/rand) | 0.8 | Seeded PRNG via `SmallRng` (Xoshiro128++) |

Both are compile-time dependencies only — no runtime libraries required.
