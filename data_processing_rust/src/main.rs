use polars::lazy::dsl::{col, lit};
use polars::prelude::{
    CsvReadOptions, ParquetCompression, ParquetWriter, SortMultipleOptions, *,
};
use std::time::Instant;

fn main() -> PolarsResult<()> {
    let start = Instant::now();
    
    let df = CsvReadOptions::default()
        .with_has_header(true)
        .try_into_reader_with_file_path(Some("events.csv".into()))?
        .finish()?;
    let mut result = df
        .lazy()
        .filter(col("country").eq(lit("IN")))
        .with_column((col("price") * col("qty")).alias("amount"))
        .group_by([col("date"), col("channel"), col("category")])
        .agg([
            col("amount").sum().alias("rev"),
            col("id").count().alias("orders"),
        ])
        .sort(["rev"], SortMultipleOptions::default())
        .collect()?; // executes the optimized plan
    ParquetWriter::new(std::fs::File::create("out_in.parquet")?)
        .with_compression(ParquetCompression::Zstd(None))
        .finish(&mut result)?;
    
    let duration = start.elapsed();
    println!("Execution time: {:?}", duration);
    
    Ok(())
}
