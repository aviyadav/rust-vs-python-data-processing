use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufWriter, Write};
use std::path::Path;
use std::time::Instant;

/// Fast xorshift64* PRNG — avoids pulling in a dependency and is much faster
/// than the default CSPRNG for bulk data generation.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn main() -> io::Result<()> {
    let mut args = env::args().skip(1);
    let rows: u64 = args
        .next()
        .expect("usage: generate <rows> [output_path]")
        .parse()
        .expect("rows must be a positive integer");
    let output = args
        .next()
        .unwrap_or_else(|| "data/measurements.txt".to_string());

    // Reuse station names from the existing weather stations file. Resolve
    // the path relative to the crate root so it works regardless of the
    // current working directory.
    let stations_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/weather_stations.csv");
    let stations_file = File::open(&stations_path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("failed to open {}: {e}", stations_path.display()),
        )
    })?;
    let stations: Vec<String> = io::BufReader::new(stations_file)
        .lines()
        .map(|line| {
            let line = line.unwrap();
            line.split(';').next().unwrap_or("").to_string()
        })
        .filter(|s| !s.is_empty())
        .collect();

    assert!(
        !stations.is_empty(),
        "no station names found in {}",
        stations_path.display()
    );

    // Precompute every possible temperature string. We use 4 decimal
    // places (e.g. "35.6897") to match the format of
    // weather_stations.csv. The integer range -900_000..=900_000 maps to
    // -90.0000..=90.0000, mirroring the latitudes/coords in the file.
    // Precomputing avoids formatting a float on every row, which is the
    // main CPU cost at 1B rows.
    let temps: Vec<String> = (-900_000i32..=900_000)
        .map(|t| {
            let sign = if t < 0 { "-" } else { "" };
            let abs = t.unsigned_abs();
            format!("{sign}{}.{:04}", abs / 10_000, abs % 10_000)
        })
        .collect();

    // Create parent directories if needed (e.g. data/ may not exist).
    if let Some(parent) = Path::new(&output).parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    let file = File::create(&output)?;
    let mut out = BufWriter::with_capacity(8 * 1024 * 1024, file);

    // Time only the row-generation loop; setup (station loading,
    // temperature precompute) is excluded.
    let start = Instant::now();

    // Fixed seed for reproducible output.
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let report_every = rows / 10;

    for i in 0..rows {
        let station = &stations[rng.below(stations.len())];
        let temp = &temps[rng.below(temps.len())];
        out.write_all(station.as_bytes())?;
        out.write_all(b";")?;
        out.write_all(temp.as_bytes())?;
        out.write_all(b"\n")?;

        if report_every > 0 && (i + 1) % report_every == 0 {
            eprintln!("progress: {} / {rows} rows", i + 1);
        }
    }

    out.flush()?;
    println!("Time taken: {:.3?}", start.elapsed());
    println!("Wrote {rows} rows to {output}");
    Ok(())
}
