# Large File Processing: Python vs Rust

A benchmarking project that compares CSV data processing performance between Python and Rust implementations. Both versions read a large CSV of financial transactions, aggregate them by date and category (count and total), and write a summary CSV.

## Dataset

`data/transactions.csv` — 1 million rows (~71 MB)

| Column        | Type   | Description                      |
|---------------|--------|----------------------------------|
| `date`        | string | Transaction date (`YYYY-MM-DD`) |
| `transaction_id` | string | UUID                       |
| `category`    | string | Transaction category             |
| `merchant`    | string | Merchant name                    |
| `amount`      | float  | Transaction amount               |

## Implementations

### Rust (`rs-code/process-data-rs/`)

Standard-library-focused approach using the `csv` crate with `serde` deserialization and `HashMap` aggregation.

**Dependencies:** `csv`, `serde`

```sh
cd rs-code/process-data-rs
cargo run --release
```

Output: `summary_rust.csv` (written to repo root)

### Python (`py-code/process-data-py/`)

Two approaches:

| Function          | Approach                                                  |
|-------------------|-----------------------------------------------------------|
| `process_python()`  | stdlib `csv.DictReader` + `defaultdict` aggregation     |
| `process_polars()`  | `pyarrow` reads the CSV into Arrow; `polars` aggregates |

**Dependencies:** `polars`, `pyarrow`, `duckdb`

```sh
cd py-code/process-data-py
uv run main.py
```

Output: `summary.csv` (stdlib) / `summary_polars.csv` (polars)

## Project Structure

```
.
├── data/
│   └── transactions.csv
├── py-code/
│   └── process-data-py/
│       ├── main.py
│       ├── pyproject.toml
│       └── .venv/
├── rs-code/
│   └── process-data-rs/
│       ├── Cargo.toml
│       └── src/
│           └── main.rs
└── README.md
```

## Running Benchmarks

Each implementation prints its wall-clock time on completion:

```
Python:  0.XXXXs
Polars:  0.XXXXs
Rust:    X.XXXXs
```

All timings use high-resolution timers (`time.perf_counter()` / `std::time::Instant`) and exclude OS file cache warm-up — run each binary twice for the most accurate comparison.
