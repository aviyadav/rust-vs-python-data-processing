from fastapi import FastAPI
from main import DuckDBClient, revenue_report

app = FastAPI()
db = DuckDBClient()

@app.get("/report/revenue")
async def revenue():
    table = db.arrow("""
        SELECT *
        FROM read_parquet('data/events.parquet')
    """)

    report = revenue_report(table)
    return report.to_dicts()
