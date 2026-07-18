# process-data-rs

A Rust application for large-scale CSV transaction processing and aggregation. This project benchmarks Rust's performance for data-intensive workloads as part of a Python vs Rust comparison.

## Overview

The program reads transaction records from a CSV file, aggregates them by **date** and **category** (count and total amount), and writes the results to `summary_rust.csv`.

## Input Format

Expects a CSV file at `../../data/transactions.csv` (relative to the crate root) with the following columns:

| Column          | Description                |
|-----------------|----------------------------|
| `date`          | Transaction date           |
| `transaction_id`| Unique transaction ID     |
| `category`      | Transaction category       |
| `merchant`      | Merchant name              |
| `amount`        | Transaction amount (f64)   |

## Output

Writes `summary_rust.csv` to the current working directory with columns:

| Column    | Description                        |
|-----------|------------------------------------|
| `date`    | Transaction date                   |
| `category`| Transaction category               |
| `count`   | Number of transactions             |
| `total`   | Sum of amounts (2 decimal places)  |

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (edition 2024)
- Place `transactions.csv` at the path expected by the program (see Input Format above)

## Build & Run

```bash
# Build the project
cargo build --release

# Run (from any working directory)
cargo run --release
```

The program prints timing information to stderr and stdout.

## Dependencies

| Crate   | Version | Purpose                     |
|---------|---------|-----------------------------|
| `csv`   | 1.4.0   | CSV parsing and writing      |
| `serde` | 1.0     | Deserialization of records   |
