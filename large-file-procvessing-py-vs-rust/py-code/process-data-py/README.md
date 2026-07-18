# process-data-py

Python implementation for large CSV file processing — part of a **Py vs Rust** performance comparison.

Reads a large `transactions.csv` file, aggregates records by `date` and `category` (count + sum of amount), and writes the results to summary CSV files. Two processing strategies are provided:

| Strategy | Description |
|---|---|
| `process_python` | Pure Python using `csv.DictReader` with buffered I/O |
| `process_polars` | [Polars](https://pola.rs/) + [PyArrow](https://arrow.apache.org/docs/python/index.html) — zero-copy Arrow ingestion with native aggregation |

## Requirements

- Python **>= 3.14**
- [uv](https://docs.astral.sh/uv/) (recommended) or pip

## Setup

```bash
# Install dependencies with uv
uv sync

# Or with pip
pip install -e .
```

## Run

```bash
# Run the Polars pipeline (default)
uv run main.py

# To run the pure-Python pipeline instead, edit main.py
# to uncomment process_python() and comment out process_polars()
```

> **Note:** The script expects the input file at `../../data/transactions.csv` relative to the project directory.

## Output

| File | Produced by |
|---|---|
| `summary.csv` | `process_python()` |
| `summary_polars.csv` | `process_polars()` |

## Dependencies

- **polars** >= 1.42.1
- **pyarrow** >= 25.0.0
- **duckdb** >= 1.5.4
