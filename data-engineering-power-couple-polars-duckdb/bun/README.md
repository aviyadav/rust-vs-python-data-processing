# Benchmark: Python vs Rust CSV Generator

This Bun-based benchmark compares the **performance** (wall-clock time per phase) and **peak memory** (RSS from `/usr/bin/time -v`) of the Python and Rust user-event CSV generators.

## Prerequisites

- [Bun](https://bun.sh) >= 1.3
- Python 3 with dependencies installed (`pip install -r ../python/requirements.txt`)
- Rust release binary built (`cargo build --release` in `../rust`)
- `/usr/bin/time` (GNU time, for memory measurement)

## Setup

```bash
cd bun
bun install
```

## Usage

```bash
bun run benchmark.ts
```

The benchmark runs all combinations of:
- **Rows:** 100,000 | 500,000 | 1,000,000
- **Languages:** Python, Rust
- **Runs:** 3 iterations each (results are averaged)

Results are printed as a table showing generation time, write time, total wall time, peak RSS, and output CSV size.

## Output

Example output:

```
───────────────────────────────────────────────────────────────────────────────
│ Lang    │ Rows     │ Gen (s)  │ Write (s)│ Wall (s) │ Peak RSS  │ CSV Size │
───────────────────────────────────────────────────────────────────────────────
│ python  │  100,000 │    0.200 │    0.000 │    0.300 │  145.9 MB │    6.8 MB │
│ rust    │  100,000 │    0.061 │    0.040 │    0.101 │   42.1 MB │    6.7 MB │
│ python  │  500,000 │    0.567 │    0.100 │    0.667 │  225.9 MB │   33.9 MB │
│ rust    │  500,000 │    0.120 │    0.144 │    0.264 │  155.6 MB │   33.5 MB │
│ python  │1,000,000 │    0.700 │    0.100 │    0.800 │  318.6 MB │   67.8 MB │
│ rust    │1,000,000 │    0.274 │    0.353 │    0.627 │  230.3 MB │   67.0 MB │
───────────────────────────────────────────────────────────────────────────────
```

## Notes

- Rust's row-by-row CSV writing is slower than Python's batch `DataFrame.write_csv` for the write phase, but generation is faster.
- Python's batch-level CSV writing via Polars is highly optimized for the write phase.
- Peak RSS includes all multiprocessing workers for Python + the initial Python process. Rust uses rayon threads within a single process.
- The benchmark cleans all output files between runs (`/tmp/bench_*.csv`).
