# High-Traffic REST Service

A sample Rust REST service built with **warp**, featuring auto-generated **OpenAPI 3** documentation via **utoipa** and an interactive **Swagger UI**. Includes a separate load-test harness that hits the service with 20 parallel workers until 100 total calls are made.

## Project layout

```
high-traffic-service/
├── Cargo.toml
├── README.md
└── src/
    ├── main.rs             ← service entrypoint
    └── bin/
        └── loadtest.rs     ← load-test binary
```

## Endpoints

| Method | Path                       | Description                              |
|--------|----------------------------|------------------------------------------|
| GET    | `/greet/{name}`            | Returns a JSON greeting for `{name}`     |
| GET    | `/api-docs/openapi.json`   | Raw OpenAPI 3 specification (JSON)       |
| GET    | `/swagger-ui`              | Interactive Swagger UI (CDN-based)      |

## Dependencies

| Crate              | Purpose                          |
|--------------------|----------------------------------|
| `warp` 0.3         | Async web framework              |
| `tokio` 1 (full)   | Async runtime                    |
| `utoipa` 5         | OpenAPI spec generation          |
| `serde` / `serde_json` | JSON serialization           |
| `reqwest` 0.12     | HTTP client (load-test binary)   |

## Quick start

### 1. Build

```sh
cargo build
```

To build only the service (skip the load-test binary and its reqwest dependency), comment out the `[[bin]]` table for `loadtest` in `Cargo.toml`.

### 2. Run the service

```sh
cargo run --bin high-traffic-service
```

The server listens on **http://127.0.0.1:8080**. You should see:

```
🚀 Rust service starting on http://127.0.0.1:8080
📖 Swagger UI:   http://127.0.0.1:8080/swagger-ui
📋 OpenAPI JSON: http://127.0.0.1:8080/api-docs/openapi.json
```

### 3. Explore the API

- Open **http://127.0.0.1:8080/swagger-ui** in a browser for the interactive docs.
- Call the greet endpoint directly:

```sh
curl http://127.0.0.1:8080/greet/World
# → {"message":"Hello, World! From Rust service."}
```

### 4. Run the load test

Keep the service running, then in a second terminal:

```sh
cargo run --bin loadtest
```

This fires **20 parallel async workers** that call `GET /greet/User{n}` repeatedly until exactly **100 total calls** have been made. Each call is logged with its latency and status, followed by a summary:

```
⚡ Load test: 20 parallel workers → 100 total calls → http://127.0.0.1:8080/greet

#001 [52ms] HTTP 200 — {"message":"Hello, User0! From Rust service."}
#002 [51ms] HTTP 200 — {"message","Hello, User1! From Rust service."}
...
========================================
  Load test complete!
  Workers:      20
  Total calls:  100
  Total time:   0.34 s
  Throughput:   292.40 calls/s
========================================
```

### Load-test tuning

Edit the constants at the top of `src/bin/loadtest.rs` to change the workload:

```rust
const TOTAL_CALLS: u32 = 100;       // stop after this many calls
const PARALLEL_WORKERS: u32 = 20;   // concurrent workers
const TARGET_URL: &str = "http://127.0.0.1:8080/greet";
```

## Requirements

- Rust **1.75** or later (edition 2021)
- An internet connection for the first `cargo build` (dependency download)
