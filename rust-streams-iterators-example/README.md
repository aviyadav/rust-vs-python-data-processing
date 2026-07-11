# Rust Streams & Iterators Example

A Rust application that demonstrates asynchronous stream processing with randomly generated log files. This project showcases how to handle large volumes of log data using Tokio async runtime and efficient I/O operations.

## Features

- **Log File Generation**: Generates realistic log files with timestamps and randomized content
- **Asynchronous Processing**: Processes logs using Tokio's async runtime for efficient I/O
- **Batched Thread Processing**: Distributes file generation across multiple threads in batches
- **Custom DateTime Handling**: Implements datetime utilities without external chrono dependency
- **Random Log Data**: Each log entry contains realistic application events with:
  - Log levels (INFO, WARN, ERROR, DEBUG, TRACE)
  - Module names (api, db, auth, cache, worker, gateway, storage)
  - Request IDs and latency metrics
  - Timestamped messages

## Project Structure

```
rust-streams-iterators-example/
├── Cargo.toml                 # Project manifest with dependencies
├── README.md                  # This file
├── src/
│   └── main.rs               # Main application code
└── logs/                      # Generated log files directory
```

## Dependencies

- **rand**: Random number generation for log content
- **tokio**: Async runtime with full feature set for concurrent operations

## Building

```bash
cargo build
```

For release optimizations:

```bash
cargo build --release
```

## Running

```bash
cargo run
```

This will:
1. Generate 10 log files (configurable in `main()`)
2. Each log file contains 10-1000 random log lines
3. Process all generated files asynchronously
4. Print log content to stdout

## Log File Format

Generated log files follow this naming convention:
- `YYYY-MM-DD_HH.log` (e.g., `2024-01-01_00.log`)

Each log line format:
```
2024-01-01 00:45:23 [INFO] [api] [req-54321] request processed successfully (latency: 123ms)
```

Components:
- **Timestamp**: ISO 8601 format with seconds precision
- **Log Level**: One of INFO, WARN, ERROR, DEBUG, TRACE
- **Module**: API component generating the log
- **Request ID**: Unique identifier (10000-99999)
- **Message**: Descriptive event message
- **Latency**: Response time in milliseconds (1-500ms)

## Key Functions

### `generate_log_files(total_files: u32)`
Generates log files with one file per hour. Uses thread batching for parallel generation:
- Spawns multiple threads in batches of 100 files
- Each file contains 10-1000 random lines
- Files named by date and hour

### `process_logs_streams(path: &str)`
Asynchronously reads and processes log files using Tokio:
- Uses `tokio::fs::File` for async I/O
- Streams lines efficiently with `AsyncBufReader`
- Prints each line to stdout

### DateTime Utilities
Implements a minimal date-time system without external dependencies:
- `chrono_like_offset()`: Returns Unix hour offset for 2024-01-01
- `unix_to_ymdhms()`: Converts Unix timestamp to date/time components
- `civil_from_days()`: Gregorian calendar conversion algorithm

## Configuration

To change the number of generated files, modify line 35 in `src/main.rs`:
```rust
generate_log_files(10);  // Change 10 to desired number
```

To change the line count range per file (currently 10-1000), modify line 80 in `src/main.rs`:
```rust
let line_count: usize = rng.gen_range(10..=1000);  // Adjust range as needed
```

## Performance Considerations

- **Batching**: Files are generated in parallel batches to optimize thread utilization
- **Async I/O**: Reading uses Tokio for non-blocking I/O
- **Stream Processing**: Lines are processed sequentially to manage memory efficiently
- **Random Generation**: Thread-local RNG instances to avoid contention

## Output Example

```
Time taken: 45.23ms
2024-01-01 00:12:34 [INFO] [db] [req-45678] database query executed (latency: 234ms)
2024-01-01 00:25:45 [WARN] [cache] [req-78901] cache miss for key (latency: 45ms)
2024-01-01 00:33:12 [ERROR] [auth] [req-23456] authentication failed (latency: 89ms)
...
```

## Development

This project is useful for:
- Learning async Rust with Tokio
- Understanding stream processing patterns
- Testing log processing pipelines
- Benchmarking I/O performance
- Demonstrating producer-consumer patterns

## License

MIT

## Notes

- The application uses `unwrap()` for error handling, suitable for examples but should use proper error handling in production
- DateTime calculation uses epoch-based offset compatible with Unix timestamps
- The `#![allow(dead_code)]` attribute suppresses warnings for currently unused helper functions
