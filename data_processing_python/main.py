import pandas as pd
from time import time
import pyarrow as pa
import pyarrow.parquet as pq

def main_pandas_only():
    start = time()
    df = pd.read_csv('data_processing_eg/events.csv')
    df = df[df["country"] == "IN"].copy()
    df["amount"] = df["price"].astype(float) * df["qty"].astype(int)
    res = (
        df.groupby(["date", "channel", "category"], as_index=False)
        .agg(rev=("amount", "sum"), orders=("id", "count"))
        .sort_values("rev", ascending=False)
    )
    res.to_parquet('out_in.parquet', compression='zstd', index=False)
    end = time()
    print(f"Processing time: {end - start:.2f} seconds")

def main_pandas_with_pyarrow():
    start = time()
    df = pd.read_csv('data_processing_eg/events.csv')
    df = df[df["country"] == "US"].copy()
    df["amount"] = df["price"].astype(float) * df["qty"].astype(int)
    res = (
        df.groupby(["date", "channel", "category"], as_index=False)
        .agg(rev=("amount", "sum"), orders=("id", "count"))
        .sort_values("rev", ascending=False)
    )
    table = pa.Table.from_pandas(res)
    pq.write_table(table, 'out_us.parquet', compression='ZSTD')
    end = time()
    print(f"Processing time with PyArrow: {end - start:.2f} seconds")

if __name__ == "__main__":
    main_pandas_only()
    main_pandas_with_pyarrow()