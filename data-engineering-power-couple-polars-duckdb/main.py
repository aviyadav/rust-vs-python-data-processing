import pandas as pd
import duckdb
import polars as pl
import pyarrow.parquet as pq

class DuckDBClient:
    def __init__(self, database="analytics.db"):
        self.connnection = duckdb.connect(database)

    def execute(self, query: str, params: dict = None):
        return self.connnection.execute(
            query,
            params or {}
        )

    def arrow(self, query: str, params: dict = None):
        return (
            self.execute(query, params)
            .arrow()
        )


def using_pandas():
    df = pd.read_csv("data/events.csv")

    df = df[df["country"] == "IN"]

    df["purchase_amount"] *= 1.18

    result = (
        df.groupby("device")
        .agg({"purchase_amount": "sum"})
    )

    result.to_parquet("data/summary.parquet")


    pdf = pd.read_parquet("data/summary.parquet")
    print(pdf.head())


def csv_to_parquet():
    df = pl.read_csv("data/events.csv")
    df.write_parquet("data/events.parquet")


def using_duckdb():
    con = duckdb.connect()
    result = con.execute("""
        SELECT
            device,
            SUM(purchase_amount * 1.18) AS revenue
        FROM 'data/events.parquet'
        WHERE country = 'US'
        GROUP BY device
        ORDER BY revenue DESC
    """).fetch_df()
    print(result)

def using_duckdb_class():
    client = DuckDBClient()
    orders = client.arrow("""
    SELECT *
    FROM read_parquet('data/events.parquet')
    WHERE CAST(timestamp AS TIMESTAMP) >= DATE '2025-03-15' - INTERVAL 30 DAY
    """)
    df = pl.from_arrow(orders)
    print(df)

def using_polars():
    con = duckdb.connect()

    arrow_table = con.execute("""
    SELECT *
    FROM 'data/events.parquet'
    WHERE CAST(timestamp AS TIMESTAMP) >= DATE '2025-03-15' - INTERVAL 30 DAY
    """).arrow()

    df = pl.from_arrow(arrow_table)

    result = (
        df
        .with_columns(
            (
                pl.col("purchase_amount") * 1.18
            ).alias("taxed_amount")
        )
        .group_by("country")
        .agg(
            pl.sum("taxed_amount").alias("revenue"),
            pl.len().alias("orders")
            )
        .sort("revenue", descending=True)
    )

    print(result)

def revenue_report(table):

    df = pl.from_arrow(table)
    return (
        df
        .with_columns([
            pl.col("purchase_amount").alias("total")
        ])
        .group_by([
            "country",
            "device"
        ])
        .agg([
            pl.sum("total").alias("revenue"),
            pl.mean("total").alias("avg_order"),
            pl.len().alias("orders")
        ])
        .sort(
            "revenue",
            descending=True
        )
    )


if __name__ == "__main__":
    # csv_to_parquet()
    # using_pandas()
    # using_duckdb()
    # using_duckdb_class()
    # using_polars()
    orders = pq.read_table("data/events.parquet")
    print(revenue_report(orders))
