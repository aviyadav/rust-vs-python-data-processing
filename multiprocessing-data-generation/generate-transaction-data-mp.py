"""
generate-transaction-data-mp.py
────────────────────────────────
Memory-safe streaming generator for large transaction datasets (tested up to 1 B rows).

Root cause of the OOM crash
────────────────────────────
The old pipeline called pool.map() once for ALL records, then kept every generated
row in RAM simultaneously:

    all chunks → Polars concat → Arrow table → Pandas DataFrame → DuckDB COPY TO CSV
                 ↑ 60-100 GB for 1 B rows before a single byte reaches disk ↑

Fix: one bounded batch at a time
─────────────────────────────────
Only one batch lives in memory at a time:

  1. multiprocessing + NumPy  – N workers each fill a bounded sub-chunk
  2. Polars                   – sub-chunks are concat-ed into one batch DataFrame
  3. PyArrow (zero-copy)      – batch → Arrow table  (del Polars frame immediately)
  4. pa.csv.CSVWriter         – Arrow table is streamed to disk; header written once
  5. del + gc.collect()       – batch memory is released before the next one starts
  ─── repeat for every batch ────────────────────────────────────────────────────────
  Peak RAM ≈ memory for ONE batch, regardless of the total dataset size.

Tunables  (adjust to match your hardware)
──────────────────────────────────────────
  BATCH_SIZE   Rows processed per iteration.
               500_000  ≈  ~100 MB  — safe default for 4 GB free RAM.
               2_000_000 ≈ ~400 MB  — faster throughput if you have ≥ 16 GB free.

  MAX_WORKERS  Process-pool cap.  More workers → faster generation per batch,
               but higher IPC serialisation overhead.  Keep ≤ cpu_count.
"""

import gc
import math
import multiprocessing
import os
import sys
import time
from datetime import datetime, timedelta

import numpy as np
import polars as pl
import pyarrow as pa
import pyarrow.csv as pa_csv

# ── Configuration ──────────────────────────────────────────────────────────────

# NUM_RECORDS = 1_000_000_000
NUM_RECORDS = 5_000_000

# ↓ Tune these two knobs to match your available RAM
BATCH_SIZE = 500_000  # rows held in RAM at once
MAX_WORKERS = min(multiprocessing.cpu_count(), 8)  # worker-process cap

OUTPUT_DIR = "data"
OUTPUT_FILE = os.path.join(OUTPUT_DIR, "transaction-data.csv")

PRODUCTS = [
    "Camera",
    "Charger",
    "Tablet",
    "Printer",
    "Monitor",
    "Laptop",
    "Headphones",
    "Keyboard",
    "Phone",
    "Mouse",
]
STORES = ["Store_A", "Store_B", "Store_C", "Store_D", "Store_E"]
PAYMENT_METHODS = ["Cash", "Credit Card", "Debit Card", "UPI payment", "Apple Pay"]

START_DATE = datetime(2023, 1, 1)
END_DATE = datetime(2024, 12, 31)
DATE_RANGE_DAYS = (END_DATE - START_DATE).days

# ID zero-padding width scales automatically so TXN IDs stay uniform-width
TXN_WIDTH = len(str(NUM_RECORDS))


# ── Worker (runs inside each spawned process) ──────────────────────────────────


def generate_chunk(args: tuple[int, int, int]) -> dict[str, list]:
    """
    Generate one sub-chunk of transaction rows using NumPy for speed.

    Args:
        args: (start_idx, count, seed)
              start_idx – 1-based first transaction number in this sub-chunk
              count     – number of rows to produce
              seed      – unique RNG seed; deterministic & reproducible

    Returns:
        dict column → list, ready to pass to pl.DataFrame()
    """
    start_idx, count, seed = args
    rng = np.random.default_rng(seed)

    # ── IDs ───────────────────────────────────────────────────────────────────
    transaction_ids = [
        f"TXN{i:0{TXN_WIDTH}d}" for i in range(start_idx, start_idx + count)
    ]
    customer_ids = [f"CUST{n:05d}" for n in rng.integers(1, 100_000, count)]

    # ── Categorical fields (vectorised NumPy choice) ──────────────────────────
    products = rng.choice(PRODUCTS, count).tolist()
    stores = rng.choice(STORES, count).tolist()
    payment_methods = rng.choice(PAYMENT_METHODS, count).tolist()

    # ── Numeric fields ────────────────────────────────────────────────────────
    amounts = np.round(rng.uniform(100.00, 2_500.00, count), 2).tolist()
    quantities = rng.integers(1, 11, count).tolist()  # 1–10 inclusive

    # ── Dates ─────────────────────────────────────────────────────────────────
    day_offsets = rng.integers(0, DATE_RANGE_DAYS + 1, count)
    dates = [
        (START_DATE + timedelta(days=int(d))).strftime("%Y-%m-%d") for d in day_offsets
    ]

    return {
        "transaction_id": transaction_ids,
        "customer_id": customer_ids,
        "product": products,
        "amount": amounts,
        "quantity": quantities,
        "store": stores,
        "payment_method": payment_methods,
        "date": dates,
    }


# ── Helpers ────────────────────────────────────────────────────────────────────


def build_sub_tasks(
    batch_start: int,
    batch_size: int,
    num_workers: int,
    batch_num: int,
) -> list[tuple[int, int, int]]:
    """
    Divide one batch into per-worker sub-tasks with unique deterministic seeds.

    Two large primes keep (batch_num, worker_idx) seeds far apart in seed-space
    so no two workers ever share overlapping RNG sequences.
    """
    base = batch_size // num_workers
    extras = batch_size % num_workers
    tasks: list[tuple[int, int, int]] = []
    idx = batch_start
    for i in range(num_workers):
        count = base + (1 if i < extras else 0)
        seed = batch_num * 104_729 + i * 7_919
        tasks.append((idx, count, seed))
        idx += count
    return tasks


def generate_batch_arrow(
    batch_start: int,
    batch_size: int,
    num_workers: int,
    batch_num: int,
    pool: multiprocessing.Pool,
) -> pa.Table:
    """
    Generate one batch and return it as a PyArrow table.

    The Polars DataFrame is deleted immediately after the zero-copy
    .to_arrow() call so it never overlaps in memory with the Arrow table.
    """
    tasks = build_sub_tasks(batch_start, batch_size, num_workers, batch_num)
    chunks = pool.map(generate_chunk, tasks)

    df_polars = pl.concat([pl.DataFrame(c) for c in chunks])
    del chunks  # free raw Python dicts

    arrow_table = df_polars.to_arrow()  # zero-copy Polars → Arrow
    del df_polars  # free Polars frame immediately

    return arrow_table


def fmt_duration(seconds: float) -> str:
    """Format a duration in seconds as  h:mm:ss."""
    h = int(seconds // 3600)
    m = int((seconds % 3600) // 60)
    s = int(seconds % 60)
    return f"{h}:{m:02d}:{s:02d}"


def _log_progress(
    batch_num: int,
    num_batches: int,
    rows_written: int,
    batch_s: float,
    elapsed: float,
) -> None:
    rate = rows_written / elapsed if elapsed else 0
    eta_s = (NUM_RECORDS - rows_written) / rate if rate else 0
    pct = rows_written / NUM_RECORDS * 100
    print(
        f"  Batch {batch_num + 1:>7,}/{num_batches:<7,}"
        f"  {rows_written:>15,} rows  ({pct:5.1f}%)"
        f"  {batch_s:6.2f}s/batch"
        f"  {rate / 1_000:>7.0f}k rows/s"
        f"  ETA {fmt_duration(eta_s)}"
    )


# ── Main ───────────────────────────────────────────────────────────────────────


def main() -> None:
    os.makedirs(OUTPUT_DIR, exist_ok=True)

    num_batches = math.ceil(NUM_RECORDS / BATCH_SIZE)
    num_workers = MAX_WORKERS

    # Rough estimates shown upfront so the user can abort early if disk is tight
    est_ram_mb = BATCH_SIZE * 200 / 1_048_576  # ~200 bytes / row in Python
    est_file_gb = NUM_RECORDS * 85 / 1_073_741_824  # ~85 bytes / row in CSV

    print(f"\n{'═' * 66}")
    print(f"  Records        : {NUM_RECORDS:>20,}")
    print(f"  Batch size     : {BATCH_SIZE:>20,}  ({num_batches:,} batches)")
    print(f"  Workers        : {num_workers:>20}")
    print(f"  Est. RAM/batch : {est_ram_mb:>19.0f} MB")
    print(f"  Est. file size : {est_file_gb:>19.1f} GB")
    print(f"  Output         : {OUTPUT_FILE}")
    print(f"{'═' * 66}\n")

    t_start = time.perf_counter()
    rows_written = 0
    csv_writer: pa_csv.CSVWriter | None = None

    try:
        with multiprocessing.Pool(num_workers) as pool:
            for batch_num in range(num_batches):
                t_batch = time.perf_counter()
                batch_start = batch_num * BATCH_SIZE + 1
                batch_size = min(BATCH_SIZE, NUM_RECORDS - batch_num * BATCH_SIZE)

                # ── 1-3 : generate → Polars concat → zero-copy Arrow ──────────
                arrow_table = generate_batch_arrow(
                    batch_start, batch_size, num_workers, batch_num, pool
                )

                # ── 4 : stream to disk via PyArrow CSV writer ─────────────────
                # Initialise the writer from the first batch's schema so column
                # types (e.g. large_utf8 vs utf8) match the actual data exactly.
                if csv_writer is None:
                    csv_writer = pa_csv.CSVWriter(OUTPUT_FILE, arrow_table.schema)

                csv_writer.write_table(arrow_table)

                # ── 5 : release this batch before generating the next one ──────
                del arrow_table
                gc.collect()

                # ── progress line ─────────────────────────────────────────────
                rows_written += batch_size
                _log_progress(
                    batch_num,
                    num_batches,
                    rows_written,
                    time.perf_counter() - t_batch,
                    time.perf_counter() - t_start,
                )

    except KeyboardInterrupt:
        print(
            "\n  ⚠  Interrupted — the output file is valid up to the last "
            "completed batch."
        )

    except Exception as exc:
        print(f"\n  ✗  Error: {exc}", file=sys.stderr)
        raise

    finally:
        # Always close the writer so the file is not left in a partial state
        if csv_writer is not None:
            csv_writer.close()

        total_s = time.perf_counter() - t_start
        file_bytes = os.path.getsize(OUTPUT_FILE) if os.path.exists(OUTPUT_FILE) else 0
        file_gb = file_bytes / 1_073_741_824
        avg_rate = rows_written / max(total_s, 1e-9) / 1_000

        print(f"\n{'═' * 66}")
        print(f"  Rows written   : {rows_written:>20,}")
        print(f"  File size      : {file_gb:>19.2f} GB")
        print(f"  Total time     : {fmt_duration(total_s):>20}")
        print(f"  Avg rate       : {avg_rate:>16.0f}k rows/s")
        print(f"  Saved to       : {OUTPUT_FILE}")
        print(f"{'═' * 66}\n")


if __name__ == "__main__":
    main()
