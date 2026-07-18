# gen-data

Generates a large CSV file of fake transaction records with columns:

`date, transaction_id, category, merchant, amount`

## Design

- Records are produced in fixed-size **chunks** (default 10,000 rows).
- Chunks are generated **in parallel** using a [rayon](https://docs.rs/rayon)
  thread pool (one chunk per worker thread per batch) — Rust's equivalent of
  Python's multiprocessing, but using real OS threads with no GIL.
- Each batch of chunks (one per worker thread) is generated, immediately
  written to disk in order, and then dropped before the next batch starts.
  This bounds peak memory usage to roughly `num_workers * chunk_size` rows,
  regardless of how many total records are requested — the full dataset is
  never held in memory at once.

## Usage

```sh
cargo run --release -- [OPTIONS]
```

### Options

| Flag                  | Description                                   | Default                        |
|-----------------------|------------------------------------------------|---------------------------------|
| `-n, --records <N>`   | Total number of records to generate            | `1000000`                       |
| `-c, --chunk-size <N>`| Rows generated per chunk                       | `10000`                         |
| `-o, --output <PATH>` | Output CSV path                                | `<repo>/data/transactions.csv`  |
| `-w, --workers <N>`   | Number of parallel worker threads              | number of CPU cores             |

### Examples

Generate the default 1,000,000 records:

```sh
cargo run --release
```

Generate a smaller test file with a custom chunk size:

```sh
cargo run --release -- -n 25000 -c 5000 -o ../../data/test.csv
```
