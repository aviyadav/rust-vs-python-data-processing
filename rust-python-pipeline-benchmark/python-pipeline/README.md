# Python Data Pipeline Benchmark

Three equivalent data pipelines that perform the same ETL workload — load, clean, transform, aggregate, and save CSV data — each implemented with a different DataFrame library. The goal is to provide a direct apples-to-apples performance comparison between **Pandas**, **Polars**, and **Apache DataFusion**.

## Pipelines

| File | Library | Execution model | Output |
|---|---|---|---|
| `pipeline-pandas.py` | pandas 3.x | Eager, in-memory | `results/python_output_pandas.csv` |
| `pipeline-polars.py` | Polars 1.x | Lazy, streaming | `results/python_output_polars.csv` |
| `pipeline-datafusion.py` | DataFusion 53.x | SQL / Arrow, parallel | `results/python_output_datafusion.csv` |

## Requirements

- Python >= 3.14
- [uv](https://github.com/astral-sh/uv) (recommended) or `pip`

Dependencies are declared in `pyproject.toml`:

```python-pipeline/pyproject.toml#L6-L13
dependencies = [
    "datafusion>=53.0.0",
    "pandas>=3.0.3",
    "polars>=1.40.1",
    "psutil>=7.2.2",
    "pyarrow>=24.0.0",
]
```

Install with:

```python-pipeline/README.md#L1-1
uv sync
```

## Input Data

Each pipeline reads all `*.csv` files from a directory (default: `data/`). Files must share a common schema with at least these columns:

| Column | Type | Description |
|---|---|---|
| `product_id` | string | Unique product identifier |
| `quantity` | integer | Units sold (must be > 0) |
| `price` | float | Unit price (must be > 0) |
| `date` | date string | Transaction date |

Additional columns are ignored.

## Pipeline Stages

All three pipelines execute the same five logical stages:

1. **Load** — Discover and ingest all CSV files from the input directory.
2. **Clean** — Drop rows where `product_id`, `quantity`, `price`, or `date` is null; discard rows with non-positive `quantity` or `price`.
3. **Transform** — Derive `revenue = quantity × price` and extract `year`, `month`, `quarter` from `date`.
4. **Aggregate** — Group by `product_id`, compute `total_quantity`, `total_revenue`, and `avg_price`; sort by `total_revenue` descending.
5. **Save** — Write the aggregated result to a CSV file and print execution time and peak memory.

## Usage

```python-pipeline/README.md#L1-1
# Pandas
uv run python pipeline-pandas.py --data-dir data --output results/python_output_pandas.csv

# Polars
uv run python pipeline-polars.py --data-dir data --output results/python_output_polars.csv

# DataFusion
uv run python pipeline-datafusion.py --data-dir data --output results/python_output_datafusion.csv
```

Both flags are optional; the defaults shown above are used when omitted.

## Output Schema

Each pipeline writes a single CSV with one row per product:

| Column | Description |
|---|---|
| `product_id` | Product identifier |
| `total_quantity` | Sum of all units sold |
| `total_revenue` | Sum of `quantity × price` across all transactions |
| `avg_price` | Mean unit price across all transactions |

Rows are sorted by `total_revenue` descending.

## Library-Specific Optimizations

### Pandas (`pipeline-pandas.py`)

Pandas serves as the **baseline**. Its eager, row-oriented execution model is straightforward but loads all data into memory before any filtering takes place.

- Files are read one at a time with `pd.read_csv()` and concatenated into a single `DataFrame`.
- Cleaning and transformation mutate the `DataFrame` in place across sequential steps.
- `df.query("quantity > 0 and price > 0")` is used for row filtering; `typing.cast` annotates the return type correctly against pandas 3.x stubs.
- Results are written with `DataFrame.to_csv()`.

### Polars (`pipeline-polars.py`)

Polars uses a **lazy evaluation** model: every stage appends expressions to a query plan rather than executing immediately. The query optimizer sees the full plan before touching disk, enabling predicate push-down, column pruning, and operation fusion.

| Optimization | Detail |
|---|---|
| `pl.scan_csv()` | Registers each file as a lazy source — no I/O at scan time |
| `infer_schema_length=10000` | Samples more rows for accurate type inference on sparse columns |
| `try_parse_dates=True` | Parses date columns natively during the scan pass, eliminating a separate cast |
| `pl.concat(rechunk=False)` | Skips an eager memory reorganisation; the streaming engine handles chunking at execution time |
| `collect_schema()` | Inspects the plan schema without reading any data, used for conditional date handling |
| Single `with_columns([...])` | Derives `revenue`, `year`, `month`, and `quarter` in one expression, computed in a single data pass |
| `collect(engine="streaming")` | The sole materialisation point; streaming engine enables out-of-core execution for datasets larger than RAM |
| `result.write_csv()` | Polars' Rust-native CSV writer, faster than pandas' Python-level equivalent |

### DataFusion (`pipeline-datafusion.py`)

DataFusion exposes a **SQL-over-Arrow** interface backed by a Volcano-style query optimizer and a multi-threaded Tokio runtime. All operations are expressed as SQL so the optimizer can compile them into a single fused physical plan.

| Optimization | Detail |
|---|---|
| `with_target_partitions(cpu_count)` | Distributes file scans, hash aggregations, and sorts across all CPU cores |
| `with_batch_size(65536)` | 64 K-row Arrow RecordBatches hit SIMD-friendly sizes for vectorised execution |
| `with_repartition_file_scans(True)` | Splits large individual files across partitions instead of reading them serially |
| `with_repartition_joins/sorts(True)` | Enables partition-level parallelism for join and sort operators |
| Glob `register_csv` | Registers all `*.csv` files as one logical table in a single call via DataFusion's `ListingTable` provider |
| `_VALID_ROWS_FILTER` constant | Shared `WHERE` clause reused by COUNT queries, the VIEW, and aggregation — predicate push-down applies everywhere |
| `CREATE OR REPLACE VIEW cleaned` | Zero-cost logical VIEW encoding the filter and `quantity * price` projection; DataFusion inlines it before optimization |
| Single compound SQL aggregation | `GROUP BY / ORDER BY` against `cleaned` compiles to one fused plan: predicate push-down → parallel scan → vectorised hash-agg → sort |
| `pa_csv.write_csv()` | Results stay in Arrow columnar buffers from execution through to disk — no pandas roundtrip |

## Metrics

All three pipelines print an execution summary on completion:

```python-pipeline/README.md#L1-1
============================================================
Pipeline Execution Summary (Python + Polars)
============================================================
Duration: 12.34 seconds (0.21 minutes)
Peak Memory: 512.00 MB (0.50 GB)
============================================================
```

Peak memory is sampled via `psutil` at each pipeline stage boundary. For Polars and DataFusion, peak memory will be significantly lower than Pandas because neither library materialises intermediate full-dataset copies.
