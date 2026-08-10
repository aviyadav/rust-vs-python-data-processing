# rust-xp-sqlite

Rust experiments with SQLite using [`rusqlite`](https://crates.io/crates/rusqlite) — covering schema creation, inserts, joins, dynamic values, JSON/JSONB columns, `serde_json` integration, and async access patterns with `tokio`.

## Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain, edition 2024)
- No system SQLite required — `rusqlite` is built with the `bundled` feature, which compiles SQLite from source.

## Setup

Clone the repository and build:

```sh
cargo build
```

To verify everything compiles (including examples):

```sh
cargo check --examples
```

## Running the examples

Each experiment is a standalone example in `examples/`. Run them with:

```sh
cargo run -q --example c01-simple
cargo run -q --example c02-join
cargo run -q --example c03-values
cargo run -q --example c04-json
cargo run -q --example c05-jsonb
cargo run -q --example c06-serde-json
cargo run -q --example c07-async
```

Most examples use an in-memory database (`Connection::open_in_memory()`), so no cleanup is needed. `c07-async` writes to a file-based database `_my-db.db3` in the project root.

| Example            | Topic                                                                          |
| ------------------ | ------------------------------------------------------------------------------ |
| `c01-simple`       | Basic schema creation, insert, and select with a `STRICT` table                 |
| `c02-join`         | Multi-table schema (`org`, `person`), `RETURNING id`, and JOIN queries          |
| `c03-values`       | Dynamic updates built from `rusqlite::types::Value` name/value pairs            |
| `c04-json`         | Storing JSON as `TEXT` and querying with `json_extract` / `->>`                 |
| `c05-jsonb`        | Storing JSON as binary `JSONB` and updating with `jsonb_set`                    |
| `c06-serde-json`   | Binding `serde_json::Value` directly via rusqlite's `serde_json` feature        |
| `c07-async`        | Using SQLite connections from `tokio` tasks (file-backed DB)                    |

## Project structure

```
├── Cargo.toml
├── src/
│   ├── lib.rs          # Crate root; shared Result type
│   └── db_utils.rs     # create_schema() helper (org + person STRICT tables)
└── examples/           # c01..c07 experiments
```

## Key dependencies

- `rusqlite` (features: `bundled`, `serde_json`) — SQLite bindings; the `serde_json` feature enables binding `serde_json::Value` as SQL parameters.
- `serde` / `serde_json` — JSON construction with the `json!` macro.
- `tokio` (full) — async runtime for `c07-async`.
- `pretty-sqlite` (dev-dependency) — pretty-prints query result tables in the examples.

## Notes

- Tables are created in `STRICT` mode; the shared schema (`org`, `person`) lives in `src/db_utils.rs` and is reused by most examples.
- For production async/concurrent access, prefer a connection pool, queue, or mutex rather than spawning one connection per task (see the note in `c07-async.rs`).
