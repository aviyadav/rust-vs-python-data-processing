import glob
import os
import time
from datetime import datetime
from pathlib import Path

import pandas as pd
import psutil


class PipelineMetrics:
    """Track pipeline performance metrics"""

    def __init__(self):
        self.start_time = None
        self.end_time = None
        self.peak_memory = 0
        self.process = psutil.Process(os.getpid())

    def start(self):
        self.start_time = time.time()
        self.peak_memory = self.process.memory_info().rss / 1024 / 1024  # in MB

    def update_memory(self):
        current_memory = self.process.memory_info().rss / 1024 / 1024  # in MB
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
        print(f"Pipeline Execution Summary (Python + Pandas)")
        print(f"{'=' * 60}")
        print(f"Duration: {duration:.2f} seconds ({duration / 60:.2f} minutes)")
        print(
            f"Peak Memory: {self.peak_memory:.2f} MB ({self.peak_memory / 1024:.2f} GB)"
        )
        print(f"{'=' * 60}\n")


def load_csv_files(data_dir: str, metrics: PipelineMetrics):
    """
    Load all CSV files from directory into a single DataFrame.
    This is the memory-intensive operation in the Python version.
    """
    print(f"Loading CSV files from {data_dir}...")
    csv_files = glob.glob(f"{data_dir}/*.csv")
    print(f"Found {len(csv_files)} CSV files")
    df = pd.concat([pd.read_csv(file) for file in csv_files], ignore_index=True)

    if not csv_files:
        raise ValueError(f"No CSV files found in {data_dir}")

    dataframes = []
    for i, file in enumerate(csv_files, 1):
        print(f"Loading file {i}/{len(csv_files)}: {Path(file).name}")
        df = pd.read_csv(file)
        dataframes.append(df)
        metrics.update_memory()

    print("Concatenating dataframes...")
    combined_df = pd.concat(dataframes, ignore_index=True)
    metrics.update_memory()

    print(f"Total rows loaded: {len(combined_df):,}")
    return combined_df


def clean_data(df: pd.DataFrame, metrics: PipelineMetrics) -> pd.DataFrame:
    """
    Clean and validate data.
    """
    print("\nCleaning data...")

    initial_rows = len(df)

    # Remove rows with missing critical fields
    df = df.dropna(subset=["product_id", "quantity", "price"])

    # Remove invalid quantities and prices
    df = df[df["quantity"] > 0]
    df = df[df["price"] > 0]

    # Convert date column if exists
    if "date" in df.columns:
        df["date"] = pd.to_datetime(df["date"], errors="coerce")
        df = df.dropna(subset=["date"])

    metrics.update_memory()

    removed_rows = initial_rows - len(df)
    print(
        f"Removed {removed_rows:,} invalid rows ({removed_rows / initial_rows * 100:.2f}%)"
    )
    print(f"Remaining rows: {len(df):,}")

    return df


def transform_data(df: pd.DataFrame, metrics: PipelineMetrics) -> pd.DataFrame:
    """
    Apply business logic transformations.
    """
    print("\nTransforming data...")

    # Calculate revenue
    df["revenue"] = df["quantity"] * df["price"]

    # Add derived columns
    if "date" in df.columns:
        df["year"] = df["date"].dt.year
        df["month"] = df["date"].dt.month
        df["quarter"] = df["date"].dt.quarter

    metrics.update_memory()

    print("Transformations complete")
    return df


def aggregate_data(df: pd.DataFrame, metrics: PipelineMetrics) -> pd.DataFrame:
    """
    Perform aggregations for reporting.
    """
    print("\nAggregating data...")

    # Product-level aggregation
    product_summary = (
        df.groupby("product_id")
        .agg({"quantity": "sum", "revenue": "sum", "price": "mean"})
        .reset_index()
    )

    product_summary.columns = [
        "product_id",
        "total_quantity",
        "total_revenue",
        "avg_price",
    ]

    # Sort by revenue
    product_summary = product_summary.sort_values("total_revenue", ascending=False)

    metrics.update_memory()

    print(f"Aggregated to {len(product_summary):,} products")
    return product_summary


def save_results(df: pd.DataFrame, output_path: str, metrics: PipelineMetrics):
    """
    Save results to output file.
    """
    print(f"\nSaving results to {output_path}...")

    # Create output directory if it doesn't exist
    Path(output_path).parent.mkdir(parents=True, exist_ok=True)

    df.to_csv(output_path, index=False)

    file_size = os.path.getsize(output_path) / 1024 / 1024  # MB
    print(f"Results saved ({file_size:.2f} MB)")

    metrics.update_memory()


def run_pipeline(
    data_dir: str = "data", output_path: str = "results/python_output.csv"
):
    """
    Main pipeline execution.
    """
    metrics = PipelineMetrics()
    metrics.start()

    print(f"\n{'=' * 60}")
    print(f"Starting Python + Pandas Pipeline")
    print(f"Timestamp: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"{'=' * 60}\n")

    try:
        # Step 1: Load data
        df = load_csv_files(data_dir, metrics)

        # Step 2: Clean data
        df = clean_data(df, metrics)

        # Step 3: Transform data
        df = transform_data(df, metrics)

        # Step 4: Aggregate data
        result = aggregate_data(df, metrics)

        # Step 5: Save results
        save_results(result, output_path, metrics)

        metrics.end()
        metrics.print_summary()

        return True

    except Exception as e:
        print(f"\n❌ Pipeline failed: {str(e)}")
        metrics.end()
        return False


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Run Python + Pandas data pipeline")
    parser.add_argument(
        "--data-dir",
        default="data",
        help="Directory containing CSV files (default: data)",
    )
    parser.add_argument(
        "--output",
        default="results/python_output_pandas.csv",
        help="Output file path (default: results/python_output_pandas.csv)",
    )

    args = parser.parse_args()

    success = run_pipeline(args.data_dir, args.output)
    exit(0 if success else 1)
