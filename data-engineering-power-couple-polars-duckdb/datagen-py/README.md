# Python User-Event CSV Generator

Generates million-row user-event CSV files using **Polars**, **PyArrow**, **NumPy**, and **multiprocessing**.

Memory-efficient: batches are generated in parallel, then streamed sequentially to CSV — never loading the full dataset into memory.

## Dependencies

- Python >= 3.13
- [polars](https://pypi.org/project/polars/)
- [pyarrow](https://pypi.org/project/pyarrow/)
- [numpy](https://pypi.org/project/numpy/)

Install with pip:

```bash
pip install -e .
# or
pip install polars pyarrow numpy
```

## Usage

```bash
python generate_events.py [--rows ROWS] [--batch-size BATCH] [-o OUTPUT] [--workers N]
```

### CLI Arguments

| Argument       | Default    | Description                                 |
| -------------- | ---------- | ------------------------------------------- |
| `--rows`       | `1000000`  | Total number of rows to generate            |
| `--batch-size` | `50000`    | Rows per batch (controls peak memory)       |
| `-o, --output` | `user_events.csv` | Output CSV file path                 |
| `--workers`    | CPU count  | Number of multiprocessing worker processes  |

### Example

```bash
# Generate 1 million rows, 50k per batch, using all CPU cores
python generate_events.py --rows 1000000 --batch-size 50000 --workers 8 -o events.csv

# Quick test with 100k rows
python generate_events.py --rows 100000 --batch-size 25000 --workers 4
```

## Output Schema

| Column           | Type     | Description                                |
| ---------------- | -------- | ------------------------------------------ |
| `event_id`       | `str`    | UUID hex (first 12 chars)                  |
| `user_id`        | `str`    | UUID hex (first 8 chars)                   |
| `country`        | `str`    | One of 10 countries (US, GB, DE, FR, ...)  |
| `device`         | `str`    | desktop / mobile / tablet / laptop         |
| `browser`        | `str`    | Chrome, Firefox, Safari, Edge, ... (9)      |
| `session_time_s` | `f64`    | Log-normal session duration, clipped [1, 3600] |
| `purchase_amount`| `f64`    | 0.0 (70 %), uniform [5.00, 500.00] (30 %)  |
| `timestamp`      | `str`    | ISO-8601 `YYYY-MM-DD HH:MM:SS` (March 2025)|

## Performance

~1M rows in **0.6 seconds** (8-core, 50k batch). 68 MB CSV output. Zero nulls, verified.
