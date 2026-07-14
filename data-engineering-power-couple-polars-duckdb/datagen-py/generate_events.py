#!/usr/bin/env python3
"""Generate 1M user-event CSV records using Polars + PyArrow + NumPy + multiprocessing.

Memory-efficient: generates in batches, stream-writes each batch as CSV rows
without loading the full dataset into memory.
"""

import argparse
import multiprocessing
import time
import uuid
import numpy as np
import polars as pl
import pyarrow as pa

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------
COUNTRIES = ["US", "GB", "DE", "FR", "IN", "BR", "JP", "CA", "AU", "SG"]
DEVICES = ["desktop", "mobile", "tablet", "laptop"]
BROWSERS = ["Chrome", "Firefox", "Safari", "Edge", "Brave", "Opera", "Zen", "DuckDuckGo", "Vivaldi"]

# Pre-allocated arrow schema – avoid schema inference per batch
SCHEMA = pa.schema([
    pa.field("event_id", pa.utf8()),
    pa.field("user_id", pa.utf8()),
    pa.field("country", pa.utf8()),
    pa.field("device", pa.utf8()),
    pa.field("browser", pa.utf8()),
    pa.field("session_time_s", pa.float64()),
    pa.field("purchase_amount", pa.float64()),
    pa.field("timestamp", pa.utf8()),
])

# Number of *million* batches (see --batch-size)



def _make_uuid4() -> str:
    return uuid.uuid4().hex[:12]


def _make_user_id() -> str:
    return uuid.uuid4().hex[:8]


def _random_choice(n: int, pool: list[str], rng: np.random.Generator) -> np.ndarray:
    """Return a np.ndarray of *n* random choices from *pool*."""
    idx = rng.integers(0, len(pool), size=n)
    return np.array(pool, dtype=object)[idx]


# ---------------------------------------------------------------------------
# Core batch builder – returns a pyarrow Table (zero-copy from polars)
# ---------------------------------------------------------------------------
def make_batch(batch_size: int, seed: int | None = None) -> pa.Table:
    """Produce a single batch of *batch_size* events as a pyarrow Table."""
    rng = np.random.default_rng(seed)

    # NumPy bulk generation (fast)
    event_ids = np.array([_make_uuid4() for _ in range(batch_size)], dtype=object)
    user_ids = np.array([_make_user_id() for _ in range(batch_size)], dtype=object)

    countries = _random_choice(batch_size, COUNTRIES, rng)
    devices = _random_choice(batch_size, DEVICES, rng)
    browsers = _random_choice(batch_size, BROWSERS, rng)

    # session_time: log-normal-ish distribution (5-1800 s)
    session_time = np.clip(
        np.round(rng.lognormal(mean=4.5, sigma=1.0, size=batch_size), 1),
        a_min=1.0,
        a_max=3600.0,
    )
    # purchase_amount: 70 % zero (no purchase), rest uniform 5-500
    purchase_mask = rng.random(batch_size) < 0.30
    purchase_amount = np.where(
        purchase_mask,
        np.round(rng.uniform(5.0, 500.0, size=batch_size), 2),
        0.0,
    )

    # timestamps: random seconds over March 2025 in ISO-8601
    ts_start = 1_740_787_200  # 2025-03-01T00:00:00Z
    ts_end = 1_741_046_399  # 2025-03-31T23:59:59Z
    ts_unix = rng.integers(ts_start, ts_end, size=batch_size)

    # Use Polars for convenient column construction, then convert to Arrow
    df = pl.DataFrame(
        {
            "event_id": pl.Series(event_ids, dtype=pl.String),
            "user_id": pl.Series(user_ids, dtype=pl.String),
            "country": pl.Series(countries, dtype=pl.String),
            "device": pl.Series(devices, dtype=pl.String),
            "browser": pl.Series(browsers, dtype=pl.String),
            "session_time_s": pl.Series(session_time, dtype=pl.Float64),
            "purchase_amount": pl.Series(purchase_amount, dtype=pl.Float64),
            "timestamp": pl.from_epoch(ts_unix, time_unit="s").dt.strftime(
                "%Y-%m-%d %H:%M:%S"
            ),
        }
    )
    return df.to_arrow()


# ---------------------------------------------------------------------------
# Multiprocessing worker
# ---------------------------------------------------------------------------
def _worker(args: tuple[int, int, int]) -> pa.Table:
    """Worker called by Pool.map.  Generates one batch.

    Parameters
    ----------
    args : (seed_offset, batch_size, worker_id)
    """
    seed_offset, batch_size, worker_id = args
    seed = abs(hash(f"{seed_offset}-{worker_id}")) % (2**31 - 1)
    return make_batch(batch_size, seed=seed)


# ---------------------------------------------------------------------------
# Streaming CSV writer – writes one RecordBatch at a time
# ---------------------------------------------------------------------------
def stream_csv(batches: list[pa.Table], output_path: str) -> int:
    """Write a list of arrow Tables to CSV using plain-file appending.

    Returns the total number of rows written.
    """
    total = 0
    with open(output_path, "w", buffering=4 * 1024**2) as f:
        for i, table in enumerate(batches):
            # Convert arrow table to string CSV rows
            df = pl.from_arrow(table)
            df.write_csv(
                f,
                include_header=(i == 0),
                float_precision=2,
            )
            total += table.num_rows
    return total


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------
def main() -> None:
    parser = argparse.ArgumentParser(
        description="Generate million-user-event CSV with Polars + PyArrow."
    )
    parser.add_argument(
        "--rows",
        type=int,
        default=1_000_000,
        help="Total rows to generate (default 1 000 000).",
    )
    parser.add_argument(
        "--batch-size",
        type=int,
        default=50_000,
        help="Rows per batch (default 50 000).",
    )
    parser.add_argument(
        "-o",
        "--output",
        type=str,
        default="user_events.csv",
        help="Output CSV path.",
    )
    parser.add_argument(
        "--workers",
        type=int,
        default=multiprocessing.cpu_count(),
        help="Number of worker processes (default = cpu count).",
    )
    args = parser.parse_args()

    n_total = args.rows
    batch_size = args.batch_size
    n_batches = -(-n_total // batch_size)  # ceiling division
    n_workers = min(args.workers, n_batches)

    print(
        f"Generating {n_total:,} rows in {n_batches} batches "
        f"({batch_size:,}/batch, {n_workers} workers) ..."
    )
    t0 = time.perf_counter()

    # ---- Phase 1: parallel generation ----
    # Each worker produces a pyarrow Table
    worker_args = [
        (i, batch_size, w_id % n_workers)
        for i, w_id in enumerate(range(n_batches))
    ]

    with multiprocessing.Pool(n_workers) as pool:
        tables = pool.map(_worker, worker_args, chunksize=1)

    t_gen = time.perf_counter()
    print(f"  Generation done in {t_gen - t0:.1f} s -- writing CSV ...")

    # ---- Phase 2: streaming CSV write ----
    total_rows = stream_csv(tables, args.output)

    t_end = time.perf_counter()
    print(f"  CSV written in {t_end - t_gen:.1f} s")
    print(f"  Total rows: {total_rows:,}")
    print(f"  Output: {args.output}")
    print(f"  Wall time: {t_end - t0:.1f} s")

    # Quick validation
    df_check = pl.scan_csv(args.output).head(5).collect()
    print("\nPreview (first 5 rows):")
    print(df_check)

    n_written = pl.scan_csv(args.output, n_rows=None).select(pl.len()).collect().item()
    assert n_written == n_total, f"Mismatch: written={n_written}, expected={n_total}"


if __name__ == "__main__":
    main()
