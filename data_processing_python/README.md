# Data Generator

A Python project for generating and processing e-commerce event data. This project includes utilities for creating synthetic event datasets and performing aggregated analytics on event data.

## Features

- **Event Data Generation**: Generate synthetic e-commerce event data with realistic attributes (countries, channels, categories, prices, etc.)
- **High-Performance Generation**: Optimized for generating billions of rows using vectorized NumPy operations and multiprocessing
- **Data Processing**: Filter, transform, and aggregate event data by geographic region, sales channel, and product category
- **Parquet Output**: Store processed results in optimized Parquet format with compression
- **Performance Tracking**: Built-in timing metrics to monitor processing performance
- **Multi-processing Support**: Efficient parallel processing for large datasets
- **Memory-Efficient Streaming**: Chunked processing with temporary Parquet files to handle datasets larger than RAM

## Project Structure

```
.
├── main.py                   # Main data processing script
├── generate_events.py        # Synthetic event data generator
├── pyproject.toml           # Python project configuration
├── README.md                # This file
└── data_processing_eg/      # Example data processing (Rust implementation)
    ├── Cargo.toml
    ├── events.csv
    └── src/
        └── main.rs
```

## Requirements

- Python >= 3.14
- Dependencies (see `pyproject.toml`):
  - pandas >= 2.3.3
  - polars >= 1.36.1
  - pyarrow >= 22.0.0
  - numpy >= 2.4.0
  - duckdb >= 1.4.3

## Installation

Using `uv` (recommended):
```bash
uv sync
```

Or using pip:
```bash
pip install -r requirements.txt
```

## Usage

### Generate Event Data

Generate synthetic event data:
```bash
uv run ./generate_events.py
```

This creates a CSV file (`events.csv`) with synthetic e-commerce events.

#### Generated Data Schema

| Column | Type | Description |
|--------|------|-------------|
| `id` | Int64 | Unique transaction ID |
| `country` | String | Country code (US, CA, GB, DE, FR, IT, ES, JP, AU, IN, BR, MX, CN, RU, KR) |
| `price` | Float32 | Product price (10.00 - 1000.00) |
| `qty` | Int16 | Quantity purchased (1 - 100) |
| `date` | Date | Transaction date (2025-01-01 to 2025-12-31) |
| `channel` | String | Sales channel (online, retail, mobile, social, email, direct, referral, affiliate) |
| `category` | String | Product category (electronics, clothing, food, books, sports, home, beauty, toys, automotive, health) |

#### Performance Optimizations

The generator uses several techniques for high-speed data generation:

- **Vectorized NumPy operations**: All random data generated using NumPy's vectorized functions
- **Modern RNG**: Uses `np.random.Generator` with PCG64 backend (faster than legacy `np.random`)
- **NumPy datetime64**: Native datetime operations instead of Python datetime objects
- **Integer indexing**: Generates integer indices then maps to strings (faster than `np.random.choice`)
- **Optimized dtypes**: Uses `float32`, `int16`, `int8` where possible to reduce memory
- **Streaming architecture**: Writes chunks to temporary LZ4-compressed Parquet files
- **Parallel processing**: Utilizes all CPU cores via multiprocessing

#### Configuration

Edit `generate_events.py` to adjust:
```python
NUM_ROWS = 5_000_000      # Total rows to generate (supports billions)
CHUNK_SIZE = 500_000      # Rows per chunk (larger = less overhead)
OUTPUT_FILE = "events.csv" # Output filename
```

#### Expected Performance

| Dataset Size | Approximate Time |
|-------------|------------------|
| 1 million | ~2-5 seconds |
| 100 million | ~30-60 seconds |
| 1 billion | ~2-5 minutes |

*Performance depends on CPU cores and disk speed.*

### Process Event Data

Run the main data processing pipeline:
```bash
uv run main.py
```

The script runs two processing pipelines:

#### Pipeline 1: Pandas-only Processing
- Reads the events CSV file
- Filters for transactions from India (IN)
- Calculates total revenue per transaction (price × quantity)
- Groups by date, channel, and category
- Computes aggregated revenue and order counts
- Outputs results to `out_in.parquet` with zstd compression

#### Pipeline 2: Pandas + PyArrow Processing
- Reads the events CSV file
- Filters for transactions from United States (US)
- Calculates total revenue per transaction (price × quantity)
- Groups by date, channel, and category
- Computes aggregated revenue and order counts
- Converts to PyArrow Table for optimized writing
- Outputs results to `out_us.parquet` with ZSTD compression

### Processing Output

The output files contain:
- **date**: Transaction date
- **channel**: Sales channel
- **category**: Product category
- **rev**: Total revenue
- **orders**: Number of orders

**Output files:**
- `out_in.parquet` - India (IN) transaction aggregates
- `out_us.parquet` - US transaction aggregates

## Performance

The script includes timing information to track processing performance for both pipelines:
```
Processing time: X.XX seconds
Processing time with PyArrow: X.XX seconds
```

This allows you to compare the performance characteristics of pandas-only vs. pandas+PyArrow approaches.

## Data Processing Example (Rust)

The `data_processing_eg/` directory contains an alternative Rust implementation for high-performance data processing.

To build and run:
```bash
cd data_processing_eg
cargo build --release
cargo run --release
```
