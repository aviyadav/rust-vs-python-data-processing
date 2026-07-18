import csv
import time
from collections import defaultdict

import polars as pl
import pyarrow.csv as pa_csv


def process_python():
    start = time.perf_counter()
    aggregates = defaultdict(lambda: {"count": 0, "total": 0.0})

    with open("../../data/transactions.csv", "r", buffering=8192*1024) as f:
        reader = csv.DictReader(f)
        for row in reader:
            key = f"{row['date']}_{row['category']}"
            aggregates[key]["count"] += 1
            aggregates[key]["total"] += float(row["amount"])

    # Write output
    with open("summary.csv", "w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(["date", "category", "count", "total"])
        for key, vals in aggregates.items():
            date, category = key.split("_")
            writer.writerow([date, category, vals["count"], vals["total"]])

    print(f"Python: {time.perf_counter() - start:.4f}s")



def process_polars():
    start = time.perf_counter()

    # pyarrow parses the CSV into an Arrow table; polars runs the
    # aggregation on the zero-copy Arrow data.
    table = pa_csv.read_csv("../../data/transactions.csv")
    df = pl.from_arrow(table)

    summary = (
        df.group_by(["date", "category"])
        .agg(
            pl.len().alias("count"),
            pl.col("amount").sum().alias("total"),
        )
        .sort(["date", "category"])
    )

    summary.write_csv("summary_polars.csv")

    print(f"Polars: {time.perf_counter() - start:.4f}s")


if __name__ == "__main__":
    process_python()
    process_polars()
