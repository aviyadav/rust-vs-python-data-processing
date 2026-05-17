"""
Sample Data Generator for Pipeline Benchmarks

Generates realistic CSV files with sales data for testing both pipelines.
Uses Polars for fast, vectorized generation and multi-threaded CSV writing.
Files are generated in parallel via multiprocessing; each file is written
in batches to keep memory usage bounded.
"""

import multiprocessing as mp
import polars as pl
import numpy as np
from pathlib import Path
import argparse
from datetime import date


# Module-level lookup arrays — built once, reused across all files
_EPOCH = date(1970, 1, 1)
_PRODUCT_LOOKUP = np.array([f"PROD_{i:05d}" for i in range(1, 1001)])
_CUSTOMER_LOOKUP = np.array([f"CUST_{i:06d}" for i in range(1, 10001)])
_REGIONS = np.array(['North', 'South', 'East', 'West', 'Central'])
_CATEGORIES = np.array(['Electronics', 'Clothing', 'Food', 'Books', 'Home'])


def parse_size(size_str: str) -> int:
    """Convert size string (e.g., '1GB', '500MB') to bytes"""
    
    size_str = size_str.upper().strip()

    multipliers = {
        'KB': 1024,
        'MB': 1024 ** 2,
        'GB': 1024 ** 3,
    }

    for suffix, multiplier in multipliers.items():
        if size_str.endswith(suffix):
            return int(float(size_str[:-len(suffix)]) * multiplier)
        

    raise ValueError(f"Invalid size format: {size_str}. Use format like '1GB', '500MB', '100KB'")


def generate_sample_data(num_rows: int, start_date: date) -> pl.DataFrame:
    """Generate a sample dataset with realistic sales data"""
    rng = np.random.default_rng()

    # All random generation is fully vectorized — no Python loops
    day_offsets = rng.integers(0, 366, num_rows, dtype=np.int32)
    product_indices = rng.integers(0, 1000, num_rows, dtype=np.int32)
    quantities = rng.integers(1, 100, num_rows, dtype=np.int32)
    prices = np.round(rng.uniform(10.0, 1000.0, num_rows), 2)
    customer_indices = rng.integers(0, 10000, num_rows, dtype=np.int32)
    region_indices = rng.integers(0, 5, num_rows, dtype=np.int32)
    category_indices = rng.integers(0, 5, num_rows, dtype=np.int32)

    # Dates stored as days-since-epoch (Int32), cast to pl.Date
    start_epoch_days = np.int32((start_date - _EPOCH).days)
    date_values = start_epoch_days + day_offsets

    df = pl.DataFrame({
        'date': pl.Series(date_values, dtype=pl.Int32).cast(pl.Date),
        'product_id': pl.Series(_PRODUCT_LOOKUP[product_indices]),
        'quantity': pl.Series(quantities),
        'price': pl.Series(prices),
        'customer_id': pl.Series(_CUSTOMER_LOOKUP[customer_indices]),
        'region': pl.Series(_REGIONS[region_indices]),
        'category': pl.Series(_CATEGORIES[category_indices]),
    })

    # Inject data quality issues into 5% of rows
    num_bad_rows = int(num_rows * 0.05)
    bad_indices = rng.choice(num_rows, num_bad_rows, replace=False)
    third = num_bad_rows // 3

    null_mask = np.zeros(num_rows, dtype=bool)
    null_mask[bad_indices[:third]] = True
    neg_mask = np.zeros(num_rows, dtype=bool)
    neg_mask[bad_indices[third:2*third]] = True
    zero_mask = np.zeros(num_rows, dtype=bool)
    zero_mask[bad_indices[2*third:]] = True

    return (
        df
        .with_columns([
            pl.Series('__null_mask', null_mask),
            pl.Series('__neg_mask', neg_mask),
            pl.Series('__zero_mask', zero_mask),
        ])
        .with_columns([
            pl.when(pl.col('__null_mask')).then(None).otherwise(pl.col('product_id')).alias('product_id'),
            pl.when(pl.col('__neg_mask')).then(pl.lit(-1)).otherwise(pl.col('quantity')).alias('quantity'),
            pl.when(pl.col('__zero_mask')).then(pl.lit(0.0)).otherwise(pl.col('price')).alias('price'),
        ])
        .drop(['__null_mask', '__neg_mask', '__zero_mask'])
    )


def estimate_rows_for_size(target_bytes: int) -> int:
    """Estimate number of rows needed to reach target file size"""
    # Average row size is approximately 100 bytes in CSV format
    avg_row_size = 100
    return int(target_bytes / avg_row_size)


# ---------------------------------------------------------------------------
# Multiprocessing worker — must be a top-level function so it can be pickled.
# ---------------------------------------------------------------------------

def _generate_file_worker(args: tuple) -> tuple[int, int]:
    """
    Generate and write a single CSV file in row batches.

    Streams data in chunks of `batch_size` rows so that only one batch
    lives in memory at a time, regardless of the total file size.

    Returns (file_index, bytes_written).
    """
    file_index, rows_per_file, output_dir, batch_size = args

    start_date = date(2023, 1, 1)
    file_path = Path(output_dir) / f"sales_data_{file_index + 1:04d}.csv"

    remaining = rows_per_file
    first_batch = True

    with open(file_path, "wb") as fh:
        while remaining > 0:
            current_batch = min(batch_size, remaining)
            df = generate_sample_data(current_batch, start_date)
            # write_csv uses Polars' multi-threaded Rust writer;
            # skip the header on every batch after the first.
            df.write_csv(fh, include_header=first_batch)
            first_batch = False
            remaining -= current_batch
            del df  # release batch memory immediately

    return file_index, file_path.stat().st_size


def generate_dataset(
    target_size: str,
    output_dir: str,
    num_files: int = None,
    num_workers: int = None,
    batch_size: int = 100_000,
):
    """Generate dataset split across multiple CSV files using multiprocessing."""

    print(f"\n{'='*60}")
    print(f"Generating Dataset: {target_size}")
    print(f"{'='*60}\n")

    # Parse target size
    target_bytes = parse_size(target_size)
    target_mb = target_bytes / (1024 ** 2)

    # Determine number of files based on size
    if num_files is None:
        if target_mb < 100:
            num_files = 5
        elif target_mb < 1000:
            num_files = 20
        elif target_mb < 10000:
            num_files = 50
        else:
            num_files = 200

    # Default workers: one per CPU, capped at num_files
    if num_workers is None:
        num_workers = min(mp.cpu_count(), num_files)

    print(f"Target size:      {target_mb:.2f} MB")
    print(f"Number of files:  {num_files}")
    print(f"Workers:          {num_workers}")
    print(f"Batch size:       {batch_size:,} rows")

    # Calculate rows per file
    total_rows = estimate_rows_for_size(target_bytes)
    rows_per_file = total_rows // num_files

    print(f"Estimated rows:   {total_rows:,}")
    print(f"Rows per file:    {rows_per_file:,}\n")

    # Create output directory
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)

    # Build argument list for worker processes
    worker_args = [
        (i, rows_per_file, str(output_path), batch_size)
        for i in range(num_files)
    ]

    total_size = 0
    completed = 0

    # imap_unordered starts workers eagerly and yields results as they finish,
    # so we print progress without blocking the pool.
    with mp.Pool(processes=num_workers) as pool:
        for file_index, file_size in pool.imap_unordered(
            _generate_file_worker, worker_args
        ):
            completed += 1
            total_size += file_size
            print(
                f"  ✓ File {file_index + 1:04d} done "
                f"({file_size / (1024 ** 2):.2f} MB) "
                f"[{completed}/{num_files}]"
            )

    print(f"\n{'='*60}")
    print(f"Dataset Generation Complete")
    print(f"{'='*60}")
    print(f"Total files: {num_files}")
    print(f"Total size:  {total_size / (1024**2):.2f} MB ({total_size / (1024**3):.2f} GB)")
    print(f"Output dir:  {output_path.absolute()}")
    print(f"{'='*60}\n")


def main():
    parser = argparse.ArgumentParser(
        description="Generate sample CSV data for pipeline benchmarks",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Generate 1GB of data
  python generate_data.py --size 1GB --output data/

  # Generate 100MB of data with 10 files
  python generate_data.py --size 100MB --output data/ --files 10

  # Generate 50GB of data (production scale)
  python generate_data.py --size 50GB --output data/

  # Use 8 parallel workers, 200k-row batches
  python generate_data.py --size 10GB --output data/ --workers 8 --batch-size 200000
        """
    )

    parser.add_argument(
        '--size',
        required=True,
        help='Target dataset size (e.g., 100MB, 1GB, 50GB)'
    )

    parser.add_argument(
        '--output',
        default='data',
        help='Output directory for CSV files (default: data)'
    )

    parser.add_argument(
        '--files',
        type=int,
        help='Number of CSV files to generate (auto-calculated if not specified)'
    )

    parser.add_argument(
        '--workers',
        type=int,
        default=None,
        help='Number of parallel worker processes (default: CPU count)'
    )

    parser.add_argument(
        '--batch-size',
        type=int,
        default=100_000,
        dest='batch_size',
        help='Rows generated per batch per file (default: 100000)'
    )

    args = parser.parse_args()

    try:
        generate_dataset(args.size, args.output, args.files, args.workers, args.batch_size)
    except Exception as e:
        print(f"\n❌ Error: {str(e)}")
        exit(1)


if __name__ == "__main__":
    main()