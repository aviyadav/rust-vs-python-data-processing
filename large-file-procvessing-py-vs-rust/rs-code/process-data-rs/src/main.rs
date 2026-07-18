use csv::{Reader, Writer};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::time::Instant;

#[derive(Debug, Deserialize)]
struct Record {
    date: String,
    #[serde(rename = "transaction_id")]
    _id: String,
    category: String,
    #[serde(rename = "merchant")]
    _merchant: String,
    amount: f64,
}
#[derive(Debug, Default)]
struct Agg {
    count: u64,
    total: f64,
}
fn main() {
    let start = Instant::now();
    eprintln!("start");

    // Anchor the path to this crate's manifest directory (process-data-rs),
    // so the relative path to <repo>/data/transactions.csv is resolved
    // correctly no matter what the current working directory is when run.
    let input_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/transactions.csv");
    eprintln!("opening {:?}", input_path);
    let file = File::open(&input_path).unwrap();
    eprintln!("opened");
    let mut reader = Reader::from_reader(file);

    let mut aggregates: HashMap<String, Agg> = HashMap::new();

    for result in reader.deserialize() {
        let record: Record = result.unwrap();
        let key = format!("{}_{}", record.date, record.category);

        let agg = aggregates.entry(key).or_default();
        agg.count += 1;
        agg.total += record.amount;
    }

    eprintln!("parsed {} keys", aggregates.len());
    let mut writer = Writer::from_path("summary_rust.csv").unwrap();
    writer
        .write_record(&["date", "category", "count", "total"])
        .unwrap();

    for (key, agg) in aggregates {
        let parts: Vec<&str> = key.split('_').collect();
        writer
            .write_record(&[
                parts[0],
                parts[1],
                &agg.count.to_string(),
                &format!("{:.2}", agg.total),
            ])
            .unwrap();
    }
    writer.flush().unwrap();

    println!("Rust: {:?}", start.elapsed());
}
