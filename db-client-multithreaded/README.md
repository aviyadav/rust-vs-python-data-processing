# db-client-multithreaded

A Rust demo exploring how CPU-bound work in async tasks interacts with tokio's
multithreaded runtime — showing which worker threads block and which keep running.

## Project structure

```
src/
├── lib.rs          # Crate root (exports `pub mod metrics`)
├── metrics.rs      # Thread-safe atomic metrics counters
├── main.rs         # Binary: db-client-multithreaded
└── bin/
    ├── main2.rs    # Binary: main2
    └── second.rs   # Binary: second
```

## Binaries

### `db-client-multithreaded` (`src/main.rs`)

Spawns two async tasks alongside an observer loop. Both tasks do I/O first (a DB
query and an HTTP request), then enter an infinite `loop {}` (CPU busy-wait).
Custom `Metrics` (atomic counters) track tasks, queries, requests, and observer
ticks.

- Uses `#[tokio::main]` with defaults.
- Demonstrates that CPU-blocking a worker thread doesn't stop the observer from
  printing on other workers.

### `main2` (`src/bin/main2.rs`)

Same idea but built with a manual `tokio::runtime::Builder` (no `#[tokio::main]`
macro). The CPU-bound loop is time-limited to 3 seconds so you can see task
completion. Prints detailed tokio runtime metrics at each stage:

- Number of workers, alive tasks, global queue depth.
- Per-worker park count and total busy duration.

This exposes exactly which worker absorbed the CPU time (look for ~3 s of busy
duration on one worker).

### `second` (`src/bin/second.rs`)

A minimal binary that exercises the shared `Metrics` module without DB or HTTP
dependencies — ticks 5 times and exits.

## Prerequisites

- **PostgreSQL** running on `localhost:5432` with:
  - database: `demodb`
  - user: `demouser`
  - password: `password`

  Or edit `DB_URL` in `src/main.rs` and `src/bin/main2.rs` to match your setup.

- Internet access (for the `reqwest::get("https://google.com")` calls).

## Dependencies

| Crate    | Version | Features                                |
| -------- | ------- | --------------------------------------- |
| `sqlx`   | 0.9.0   | `postgres`, `runtime-tokio`             |
| `tokio`  | 1.53.1  | `rt-multi-thread`, `macros`, `time`     |
| `reqwest`| 0.12    | `rustls-tls` (no OpenSSL required)      |

## Build & run

### Build everything

```sh
cargo build
```

### Build a specific binary

```sh
cargo build --bin db-client-multithreaded
cargo build --bin main2
cargo build --bin second
```

### Run

```sh
cargo run --bin db-client-multithreaded
cargo run --bin main2
cargo run --bin second
```

### Lint

```sh
cargo clippy
```

## What the output shows

### `db-client-multithreaded`

```
Observer tick #001 | tasks: 0, db_queries: 0, http: 0
The num was returned: 1, but now, the CPU block will kill multithreading
Start compute...
The num was returned: 200, but the CPU block will not kill multithreading
Observer tick #002 | tasks: 2, db_queries: 1, http: 1
Observer tick #003 | tasks: 2, db_queries: 1, http: 1
...
```

The observer keeps printing once per second *despite* two workers being stuck in
`loop {}`.

### `main2`

```
>[main starts]> Tokio threads: 8 workers, 0 tasks alive, ...
...
>[finished db_the_compute]> ... total busy 3.000358929s (worker 0)
```

Worker 0 spent ~3 s busy — the CPU-bound loop was contained there while other
workers stayed available.
