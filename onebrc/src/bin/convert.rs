use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::dataframe::DataFrameWriteOptions;
use datafusion::prelude::*;
use tokio::runtime::Runtime;

fn main() {
    let input = "./data/weather_stations.csv";
    let output = "./data/weather_stations.parquet";

    let rt = Runtime::new().unwrap();
    let ctx = SessionContext::new();

    let schema = Schema::new(vec![
        Field::new("station", DataType::Utf8, false),
        Field::new("temperature", DataType::Float32, false),
    ]);

    let opts = CsvReadOptions::new()
        .delimiter(b';')
        .has_header(false)
        .schema(&schema);

    let df = rt.block_on(ctx.read_csv(input, opts)).unwrap();

    rt.block_on(df.write_parquet(output, DataFrameWriteOptions::new(), None))
        .unwrap();

    println!("Wrote {output}");
}
