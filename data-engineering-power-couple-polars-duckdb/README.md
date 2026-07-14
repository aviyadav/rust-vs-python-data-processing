# Data Engineering Power Couple: Polars + DuckDB

A hands-on playground comparing **Pandas**, **DuckDB**, and **Polars** for querying and
transforming a synthetic user-events dataset, plus a small **FastAPI** service that
exposes a revenue report as JSON. It also includes standalone data generators (Python
and Rust) and a Bun/TypeScript benchmark comparing them.

## Project Structure

```
.
├── main.py              # Pandas / DuckDB / Polars examples + revenue_report()
├── app.py                # FastAPI app exposing the revenue report as JSON
├── data/
│   ├── events.csv         # Raw generated event data
│   ├── events.parquet     # Parquet version of events.csv
│   └── summary.parquet    # Output of using_pandas()
├── analytics.db           # DuckDB database file used by DuckDBClient
├── datagen-py/            # Python (Polars + NumPy + multiprocessing) event generator
├── datagen-rs/             # Rust (Polars + Rayon) event generator
└── bun/                    # Bun/TypeScript benchmark: Python vs Rust generator
```

## Requirements

- Python >= 3.13
- [uv](https://docs.astral.sh/uv/) (recommended) or `pip`

## Setup

```bash
uv sync
# or
pip install -e .
```

### Dependencies

- [`duckdb`](https://pypi.org/project/duckdb/) — embedded OLAP SQL engine
- [`polars`](https://pypi.org/project/polars/) — fast DataFrame library
- [`pandas`](https://pypi.org/project/pandas/) — for the baseline comparison
- [`pyarrow`](https://pypi.org/project/pyarrow/) — zero-copy interchange between DuckDB and Polars
- [`fastapi`](https://pypi.org/project/fastapi/) + [`uvicorn`](https://pypi.org/project/uvicorn/) — JSON API layer

## Data

`data/events.parquet` contains synthetic user-event records with the following schema:

| Column            | Type  | Description                                  |
| ----------------- | ----- | --------------------------------------------- |
| `event_id`        | str   | Unique event identifier                        |
| `user_id`         | str   | Unique user identifier                         |
| `country`         | str   | ISO-ish country code (US, GB, DE, FR, ...)     |
| `device`          | str   | desktop / mobile / tablet / laptop             |
| `browser`         | str   | Chrome, Firefox, Safari, Edge, ...             |
| `session_time_s`  | f64   | Session duration in seconds                    |
| `purchase_amount` | f64   | 0.0 for non-purchases, otherwise a dollar amount |
| `timestamp`       | str   | ISO-8601 `YYYY-MM-DD HH:MM:SS`                 |

Regenerate it from `data/events.csv` at any time with:

```bash
python -c "from main import csv_to_parquet; csv_to_parquet()"
```

To generate a fresh, larger CSV dataset, see the [`datagen-py`](datagen-py/README.md) or
[`datagen-rs`](datagen-rs/README.md) generators.

## Usage

### Run the examples in `main.py`

Uncomment the function you want to run in the `if __name__ == "__main__":` block, then:

```bash
python main.py
```

Available examples:

- `using_pandas()` — filter/aggregate with Pandas, round-trip through Parquet
- `csv_to_parquet()` — convert `events.csv` to `events.parquet` via Polars
- `using_duckdb()` — aggregate directly in SQL with DuckDB
- `using_duckdb_class()` — query via the reusable `DuckDBClient` wrapper, load into Polars
- `using_polars()` — query with DuckDB, hand the Arrow result to Polars for transformation
- `revenue_report(table)` — group a PyArrow `orders` table by `country`/`device` and compute
  `revenue`, `avg_order`, and `orders` counts, sorted by revenue descending

### Run the API

```bash
uvicorn app:app --reload
```

Then request the revenue report:

```bash
curl http://127.0.0.1:8000/report/revenue
```

This reads `data/events.parquet` via DuckDB, builds the report with `revenue_report`,
and returns it as a JSON array of records, e.g.:

```json
[
  { "country": "GB", "device": "mobile", "revenue": 1961667.29, "avg_order": 77.31, "orders": 25375 }
]
```

## Related Sub-projects

- [`datagen-py/`](datagen-py/README.md) — Polars/NumPy/multiprocessing CSV generator (~1M rows/0.6s)
- [`datagen-rs/`](datagen-rs/README.md) — Rust/Rayon CSV generator (~1M rows/0.4s)
- [`bun/`](bun/README.md) — Bun/TypeScript benchmark comparing the two generators' speed and memory use
