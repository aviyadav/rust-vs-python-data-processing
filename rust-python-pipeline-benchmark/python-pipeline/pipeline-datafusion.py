"""
Python + DataFusion data pipeline.

Mirrors the structure and logic of pipeline-pandas.py but replaces every
pandas/glob/concat operation with DataFusion equivalents optimised for
throughput on large CSV datasets:

  - Parallel CSV scan via a glob-registered logical table
  - SQL-first: all cleaning, transformation, and aggregation are expressed as
    SQL so DataFusion's Volcano optimizer can push predicates, prune columns,
    and schedule work across all CPU cores automatically
  - Arrow-native output: results stay in columnar Arrow memory and are written
    with pyarrow.csv.write_csv, avoiding the per-row Python overhead of
    pandas.DataFrame.to_csv

Note on async
-------------
DataFusion's Python bindings (>= 40.0) embed their own Tokio async runtime
and expose a fully *synchronous* interface.  ctx.sql() and df.collect() block
until the query completes while releasing the GIL internally so they do not
starve other threads.  There is therefore no need to call asyncio.run(); doing
so would add overhead without any benefit.  The pipeline is structured as plain
synchronous Python functions.
"""

import glob
import os
import time
from datetime import datetime
from pathlib import Path

import psutil
import pyarrow as pa
import pyarrow.csv as pa_csv
from datafusion import CsvReadOptions, SessionConfig, SessionContext

# ---------------------------------------------------------------------------
# Reusable filter shared by COUNT queries, the cleaned VIEW, and the main
# aggregation query.  Keeping it in one place ensures all three operations
# agree on what constitutes a "valid" row.
# ---------------------------------------------------------------------------
_VALID_ROWS_FILTER = """
        product_id IS NOT NULL
    AND quantity   IS NOT NULL
    AND price      IS NOT NULL
    AND quantity   > 0
    AND price      > 0
    AND date       IS NOT NULL
""".strip()


# ---------------------------------------------------------------------------
# Metrics
# ---------------------------------------------------------------------------


class PipelineMetrics:
    """Track pipeline performance metrics (identical to the pandas version)."""

    def __init__(self):
        self.start_time = None
        self.end_time = None
        self.peak_memory = 0
        self.process = psutil.Process(os.getpid())

    def start(self):
        self.start_time = time.time()
        self.peak_memory = self.process.memory_info().rss / 1024 / 1024  # MB

    def update_memory(self):
        current_memory = self.process.memory_info().rss / 1024 / 1024  # MB
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
        print("Pipeline Execution Summary (Python + DataFusion)")
        print(f"{'=' * 60}")
        print(f"Duration: {duration:.2f} seconds ({duration / 60:.2f} minutes)")
        print(
            f"Peak Memory: {self.peak_memory:.2f} MB ({self.peak_memory / 1024:.2f} GB)"
        )
        print(f"{'=' * 60}\n")


# ---------------------------------------------------------------------------
# Session context factory
# ---------------------------------------------------------------------------


def _build_session_context() -> SessionContext:
    """
    Build a :class:`SessionContext` tuned for bulk CSV ingestion and
    aggregation.

    Optimisations applied
    ---------------------
    target_partitions
        Match the number of logical CPU cores so that file scans,
        aggregations, and sorts can all run in parallel.
    batch_size
        64 K rows per Arrow RecordBatch gives SIMD-friendly chunk sizes while
        keeping per-batch overhead low.
    repartition_file_scans
        Allow the planner to spread a single large file across multiple
        partitions instead of reading it serially.
    repartition_joins / sorts
        Enable partition-level parallelism for hash joins and sort operators
        (relevant when the pipeline is extended with joins later).
    collect_statistics
        Let the optimizer use column-level statistics (min/max/null counts)
        from Arrow's metadata to produce better physical plans.
    parquet.pushdown_filters
        Push row-group-level predicates into the Parquet reader so entire row
        groups are skipped without decoding.  Included here for consistency
        because the same context is often reused for mixed workloads.
    """
    cpu_count = os.cpu_count() or 1
    config = (
        SessionConfig()
        .with_target_partitions(cpu_count)
        .with_batch_size(65536)
        .with_repartition_file_scans(True)
        .with_repartition_joins(True)
        .with_repartition_sorts(True)
        .set("datafusion.execution.collect_statistics", "true")
        .set("datafusion.execution.parquet.pushdown_filters", "true")
    )
    return SessionContext(config=config)


# ---------------------------------------------------------------------------
# Pipeline steps
# ---------------------------------------------------------------------------


def load_csv_files(
    data_dir: str, metrics: PipelineMetrics
) -> tuple[SessionContext, int]:
    """
    Register all CSV files in *data_dir* as a single logical table and return
    the total row count.

    Unlike the pandas version, no data is loaded into Python memory here.
    DataFusion's CSV reader discovers every file matching the glob, infers a
    unified schema from the first ``schema_infer_max_records`` rows of each
    file, and defers the actual scan until a query is executed.  At query time
    the physical planner distributes file chunks across
    ``target_partitions`` worker threads.

    Args:
        data_dir: Directory that contains the ``*.csv`` source files.
        metrics:  Shared :class:`PipelineMetrics` instance.

    Returns:
        A tuple of *(ctx, total_rows)* where *ctx* is the configured
        :class:`SessionContext` and *total_rows* is the unfiltered row count.
    """
    print(f"Loading CSV files from {data_dir}...")

    csv_files = glob.glob(f"{data_dir}/*.csv")
    if not csv_files:
        raise ValueError(f"No CSV files found in {data_dir}")

    print(f"Found {len(csv_files)} CSV files")

    ctx = _build_session_context()

    # Register all matching files as one logical table.  The glob path is
    # resolved by DataFusion's ListingTable provider — no Python-level loop
    # needed.
    glob_path = str(Path(data_dir) / "*.csv")
    ctx.register_csv(
        "sales_data",
        glob_path,
        options=CsvReadOptions(has_header=True),
    )

    print("CSV table registered — counting total rows...")
    total_rows: int = (
        ctx.sql("SELECT COUNT(*) AS cnt FROM sales_data").collect()[0]["cnt"][0].as_py()
    )
    metrics.update_memory()

    print(f"Total rows loaded: {total_rows:,}")
    return ctx, total_rows


def clean_data(
    ctx: SessionContext, metrics: PipelineMetrics
) -> tuple[SessionContext, int, int]:
    """
    Report data-quality statistics without materialising any rows.

    Two lightweight COUNT queries determine how many rows pass the validity
    filter so the pipeline can log cleaning statistics that are directly
    comparable to the pandas version.  DataFusion's predicate push-down
    ensures the scan only decodes the columns referenced in the WHERE clause.

    Args:
        ctx:     Active :class:`SessionContext` with ``sales_data`` registered.
        metrics: Shared :class:`PipelineMetrics` instance.

    Returns:
        A tuple of *(ctx, valid_rows, removed_rows)*.
    """
    print("\nCleaning data...")

    total_rows: int = (
        ctx.sql("SELECT COUNT(*) AS cnt FROM sales_data").collect()[0]["cnt"][0].as_py()
    )

    valid_rows: int = (
        ctx.sql(f"SELECT COUNT(*) AS cnt FROM sales_data WHERE {_VALID_ROWS_FILTER}")
        .collect()[0]["cnt"][0]
        .as_py()
    )

    metrics.update_memory()

    removed_rows = total_rows - valid_rows
    print(
        f"Removed {removed_rows:,} invalid rows"
        f" ({removed_rows / total_rows * 100:.2f}%)"
    )
    print(f"Remaining rows: {valid_rows:,}")

    return ctx, valid_rows, removed_rows


def transform_data(ctx: SessionContext, metrics: PipelineMetrics) -> SessionContext:
    """
    Register a logical VIEW that encodes cleaning predicates and the derived
    ``revenue`` column.

    No data is scanned or materialised.  DataFusion stores the SQL expression
    tree and inlines the VIEW definition into every downstream query before
    optimisation.  The Volcano optimizer can then fuse the filter, the
    column projection (``quantity * price``), and the GROUP BY into a single
    physical plan with no intermediate materialisation step.

    Args:
        ctx:     Active :class:`SessionContext`.
        metrics: Shared :class:`PipelineMetrics` instance.

    Returns:
        The same *ctx* with the ``cleaned`` VIEW registered.
    """
    print("\nTransforming data...")

    ctx.sql(f"""
        CREATE OR REPLACE VIEW cleaned AS
        SELECT
            product_id,
            CAST(quantity AS BIGINT) AS quantity,
            price,
            date,
            quantity * price          AS revenue
        FROM sales_data
        WHERE {_VALID_ROWS_FILTER}
    """).collect()  # .collect() executes the DDL; returns an empty list

    metrics.update_memory()
    print("Transformations complete (cleaned VIEW registered)")
    return ctx


def aggregate_data(
    ctx: SessionContext, metrics: PipelineMetrics
) -> list[pa.RecordBatch]:
    """
    Run the full clean → transform → aggregate pipeline as a single compound
    SQL query against the ``cleaned`` VIEW.

    Because DataFusion inlines the VIEW definition before optimisation, the
    physical plan for this query covers:

    * Predicate push-down into the CSV scan (skip invalid rows early)
    * Parallel file scan across all CPU cores
    * Vectorised hash aggregation
    * Top-level sort

    All in a single pass with no intermediate in-memory DataFrames.

    Args:
        ctx:     Active :class:`SessionContext` with the ``cleaned`` VIEW.
        metrics: Shared :class:`PipelineMetrics` instance.

    Returns:
        A list of :class:`pyarrow.RecordBatch` objects ready for writing.
    """
    print("\nAggregating data...")

    aggregation_sql = """
        SELECT
            product_id,
            SUM(quantity) AS total_quantity,
            SUM(revenue)  AS total_revenue,
            AVG(price)    AS avg_price
        FROM cleaned
        GROUP BY product_id
        ORDER BY total_revenue DESC
    """

    batches: list[pa.RecordBatch] = ctx.sql(aggregation_sql).collect()
    metrics.update_memory()

    total_products = sum(b.num_rows for b in batches)
    print(f"Aggregated to {total_products:,} products")
    return batches


def save_results(
    batches: list[pa.RecordBatch],
    output_path: str,
    metrics: PipelineMetrics,
) -> None:
    """
    Write Arrow RecordBatches to a CSV file using pyarrow's native writer.

    ``pyarrow.csv.write_csv`` is significantly faster than
    ``pandas.DataFrame.to_csv`` because it operates directly on columnar Arrow
    buffers without per-row Python overhead or an intermediate object-array
    conversion step.

    Args:
        batches:     Result of :func:`aggregate_data`.
        output_path: Destination CSV file path (parent directory is created
                     automatically).
        metrics:     Shared :class:`PipelineMetrics` instance.
    """
    print(f"\nSaving results to {output_path}...")

    Path(output_path).parent.mkdir(parents=True, exist_ok=True)

    table = pa.Table.from_batches(batches)
    pa_csv.write_csv(table, output_path)  # type: ignore[attr-defined]

    file_size = os.path.getsize(output_path) / 1024 / 1024  # MB
    print(f"Results saved ({file_size:.2f} MB)")
    metrics.update_memory()


# ---------------------------------------------------------------------------
# Orchestrator
# ---------------------------------------------------------------------------


def run_pipeline(
    data_dir: str = "data",
    output_path: str = "results/python_output_datafusion.csv",
) -> bool:
    """
    Orchestrate the end-to-end DataFusion pipeline.

    Steps
    -----
    1. Register CSV files as a single logical table (no data in memory yet).
    2. Run COUNT queries to report cleaning statistics.
    3. Register a ``cleaned`` VIEW (still no data in memory).
    4. Execute the compound aggregation query — this is where DataFusion does
       all the real work in parallel.
    5. Write the Arrow result to CSV via pyarrow's native writer.

    Args:
        data_dir:    Directory containing ``*.csv`` source files.
        output_path: Destination for the aggregated output CSV.

    Returns:
        ``True`` on success, ``False`` on failure.
    """
    metrics = PipelineMetrics()
    metrics.start()

    print(f"\n{'=' * 60}")
    print("Starting Python + DataFusion Pipeline")
    print(f"Timestamp: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"{'=' * 60}\n")

    try:
        # Step 1: Register CSV table, get total row count
        ctx, _total_rows = load_csv_files(data_dir, metrics)

        # Step 2: Report cleaning statistics (two COUNT queries)
        ctx, _valid_rows, _removed_rows = clean_data(ctx, metrics)

        # Step 3: Register cleaned VIEW (lazy — no data scanned)
        ctx = transform_data(ctx, metrics)

        # Step 4: Execute the compound aggregation query
        batches = aggregate_data(ctx, metrics)

        # Step 5: Write Arrow result to CSV
        save_results(batches, output_path, metrics)

        metrics.end()
        metrics.print_summary()
        return True

    except Exception as e:
        print(f"\n❌ Pipeline failed: {str(e)}")
        metrics.end()
        return False


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(
        description="Run Python + DataFusion data pipeline"
    )
    parser.add_argument(
        "--data-dir",
        default="data",
        help="Directory containing CSV files (default: data)",
    )
    parser.add_argument(
        "--output",
        default="results/python_output_datafusion.csv",
        help="Output file path (default: results/python_output_datafusion.csv)",
    )

    args = parser.parse_args()
    success = run_pipeline(args.data_dir, args.output)
    exit(0 if success else 1)
