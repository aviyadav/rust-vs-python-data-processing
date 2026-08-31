use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::functions_aggregate::expr_fn::{avg, max, min};
use datafusion::prelude::*;
use std::time::Instant;
use tokio::runtime::Runtime;

fn main() {
    let start = Instant::now();
    // put in the path
    let path = "./data/measurements.parquet";

    let rt = Runtime::new().unwrap();
    let ctx = SessionContext::new();

    let station_field = Field::new("station", DataType::Utf8, false);
    let temp_field = Field::new("temperature", DataType::Float32, false);

    let schema = Schema::new(vec![station_field, temp_field]);

    let opts = ParquetReadOptions::new().schema(&schema);

    let df = rt.block_on(ctx.read_parquet(path, opts)).unwrap();

    let results_fut = df
        .aggregate(
            vec![col("station")],
            vec![
                min(col("temperature")).alias("min_temp"),
                avg(col("temperature")).alias("mean_temp"),
                max(col("temperature")).alias("max_temp"),
            ],
        )
        .unwrap()
        .sort(vec![col("station").sort(true, false)])
        .unwrap()
        .collect();

    let results = rt.block_on(results_fut);

    let pretty = datafusion::arrow::util::pretty::pretty_format_batches(&results.unwrap()).unwrap();

    println!("Time taken: {:.3?}", start.elapsed());
    println!("{pretty}");
}
