import glob
import os
import time
from datetime import datetime
from pathlib import Path

import polars as pl
import psutil


class PipelineMetrics:
    def __init__(self):
        self.start_time = None
        self.end_time = None
        self.peak_memory = 0
        self.process = psutil.Process(os.getpid())

    def start(self):
        self.start_time = time.time()
        self.peak_memory = self.process.memory_info().rss / 1024 / 1024

    def update_memory(self):
        current_memory = self.process.memory_info().rss / 1024 / 1024
        self.peak_memory = max(self.peak_memory, current_memory)

    def end(self):
        self.end_time = time.time()

    def get_duration(self):
        if self.start_time and self.end_time:
            return self.end_time - self.start_time
        return 0

    def get_peak_memory(self):
        return self.peak_memory

    def print_summary(self):
        duration = self.get_duration()
        print(f"\n{'=' * 60}")
        print("Pipeline Execution Summary (Python + Polars)")
        print(f"{'=' * 60}")
        print(f"Duration: {duration:.2f} seconds ({duration / 60:.2f} minutes)")
        print(
            f"Peak Memory: {self.peak_memory:.2f} MB ({self.peak_memory / 1024:.2f} GB)"
        )
        print(f"{'=' * 60}\n")


def load_csv_files(data_dir, metrics):
    """Scan all CSV files lazily and return a concatenated LazyFrame.

    Uses pl.scan_csv() so no data is loaded into memory yet — Polars will
    incorporate these sources into the full query plan built downstream.
    """
    print(f"Loading CSV files from {data_dir}...")
    csv_files = glob.glob(f"{data_dir}/*.csv")
    print(f"Found {len(csv_files)} CSV files")

    if not csv_files:
        raise FileNotFoundError(f"No CSV files found in '{data_dir}'")

    lazy_frames = []
    for i, file in enumerate(csv_files, 1):
        print(f"Scanning file {i}/{len(csv_files)}: {Path(file).name}")
        lf = pl.scan_csv(
            file,
            infer_schema_length=10000,  # sample more rows for accurate type inference
            try_parse_dates=True,  # parse date columns natively, avoiding a later cast
        )
        lazy_frames.append(lf)
        metrics.update_memory()

    print("Building lazy concatenation plan...")
    # Concatenate LazyFrames — no data is materialised here.
    # rechunk=False avoids an eager memory copy; the optimizer handles chunking.
    combined_lf = pl.concat(lazy_frames, rechunk=False)
    metrics.update_memory()

    print("Lazy scan plan ready (data not yet loaded)")
    return combined_lf


def clean_data(lf, metrics):
    """Apply cleaning filters to the LazyFrame — no collect() call here.

    All expressions are appended to the query plan so the optimizer can
    push filters as close to the source as possible.
    """
    print("\nCleaning data (lazy)...")

    # Drop rows where critical columns are null.
    # Using drop_nulls on a subset is equivalent to pandas dropna(subset=[...]).
    lf = lf.drop_nulls(subset=["product_id", "quantity", "price"])

    # Filter out non-positive quantities and prices.
    lf = lf.filter((pl.col("quantity") > 0) & (pl.col("price") > 0))

    # If a "date" column is present and wasn't parsed by try_parse_dates,
    # attempt an explicit cast and drop rows where the cast produced nulls.
    # We can't inspect the schema without collecting, so we handle this
    # defensively by checking column names after a lightweight schema peek.
    schema = lf.collect_schema()
    if "date" in schema.names():
        date_dtype = schema["date"]
        # If Polars didn't already recognise it as a Date/Datetime, cast it.
        if date_dtype not in (pl.Date, pl.Datetime):
            lf = lf.with_columns(pl.col("date").str.to_datetime(strict=False))
        lf = lf.drop_nulls(subset=["date"])

    metrics.update_memory()
    print("Cleaning filters added to query plan")
    return lf


def transform_data(lf, metrics):
    """Derive new columns in a single with_columns() call — no collect() here.

    Batching all derived columns into one call lets the query optimizer
    evaluate them in one pass over the data rather than making multiple passes.
    """
    print("\nTransforming data (lazy)...")

    schema = lf.collect_schema()
    has_date = "date" in schema.names()

    derived_columns = [
        (pl.col("quantity") * pl.col("price")).alias("revenue"),
    ]

    if has_date:
        derived_columns += [
            pl.col("date").dt.year().alias("year"),
            pl.col("date").dt.month().alias("month"),
            pl.col("date").dt.quarter().alias("quarter"),
        ]

    lf = lf.with_columns(derived_columns)

    metrics.update_memory()
    print("Transformation expressions added to query plan")
    return lf


def aggregate_data(lf, metrics):
    """Build the group-by aggregation and sort on the LazyFrame — no collect() here.

    Keeping this lazy lets Polars fuse the aggregation with the preceding
    filter/projection nodes for minimal data movement.
    """
    print("\nAggregating data (lazy)...")

    lf = (
        lf.group_by("product_id")
        .agg(
            [
                pl.sum("quantity").alias("total_quantity"),
                pl.sum("revenue").alias("total_revenue"),
                pl.mean("price").alias("avg_price"),
            ]
        )
        .sort("total_revenue", descending=True)
    )

    metrics.update_memory()
    print("Aggregation plan ready")
    return lf


def save_results(lf, output_path, metrics):
    """Materialise the full lazy pipeline with streaming, then write CSV.

    This is the single collect() call in the entire pipeline.  engine="streaming"
    enables out-of-core execution so datasets larger than available RAM can be
    processed without OOM errors.
    """
    print(f"\nExecuting pipeline and saving results to {output_path}...")
    Path(output_path).parent.mkdir(parents=True, exist_ok=True)

    # --- single materialisation point ---
    # engine="streaming" enables out-of-core execution (datasets larger than RAM).
    # This replaced the deprecated streaming=True kwarg in Polars >= 1.25.
    result = lf.collect(engine="streaming")
    metrics.update_memory()

    print(f"Collected {len(result):,} product rows")

    # Polars native CSV writer is significantly faster than pandas' equivalent.
    result.write_csv(output_path)

    file_size = os.path.getsize(output_path) / 1024 / 1024
    print(f"Results saved ({file_size:.2f} MB)")
    metrics.update_memory()


def run_pipeline(data_dir="data", output_path="results/python_output_polars.csv"):
    metrics = PipelineMetrics()
    metrics.start()
    print(f"\n{'=' * 60}")
    print("Starting Python + Polars Pipeline")
    print(f"Timestamp: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"{'=' * 60}\n")
    try:
        lf = load_csv_files(data_dir, metrics)
        lf = clean_data(lf, metrics)
        lf = transform_data(lf, metrics)
        lf = aggregate_data(lf, metrics)
        save_results(lf, output_path, metrics)
        metrics.end()
        metrics.print_summary()
        return True
    except Exception as e:
        print(f"\n❌ Pipeline failed: {str(e)}")
        metrics.end()
        return False


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Run Python + Polars data pipeline")
    parser.add_argument("--data-dir", default="data")
    parser.add_argument("--output", default="results/python_output_polars.csv")
    args = parser.parse_args()
    success = run_pipeline(args.data_dir, args.output)
    exit(0 if success else 1)
