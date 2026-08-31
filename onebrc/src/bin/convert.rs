use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::dataframe::DataFrameWriteOptions;
use datafusion::prelude::*;
use std::time::Instant;
use tokio::runtime::Runtime;

fn main() {
    let start = Instant::now();
    let input = "./data/measurements.txt";
    let output = "./data/measurements.parquet";

    let rt = Runtime::new().unwrap();
    let ctx = SessionContext::new();

    let schema = Schema::new(vec![
        Field::new("station", DataType::Utf8, false),
        Field::new("temperature", DataType::Float32, false),
    ]);

    let opts = CsvReadOptions::new()
        .delimiter(b';')
        .has_header(false)
        .file_extension("txt")
        .schema(&schema);

    let df = rt.block_on(ctx.read_csv(input, opts)).unwrap();

    rt.block_on(df.write_parquet(output, DataFrameWriteOptions::new(), None))
        .unwrap();

    println!("Time taken: {:.3?}", start.elapsed());
    println!("Wrote {output}");
}
