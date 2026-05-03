# PostgreSQL Table API

A lightweight Rust REST API that returns any PostgreSQL table's contents as JSON.

## Features

- Query any table by schema and table name via a single endpoint
- Automatic per-query timing and row-count logging
- Request-level timing middleware (method, path, status, duration)
- Row limit enforced on every query (default 1 000, max 10 000) to prevent memory exhaustion
- Internal errors are never exposed to API clients — logged server-side only
- Input validation on all identifier parameters to block SQL injection

## Prerequisites

- **Rust** (latest stable) — [Install Rust](https://www.rust-lang.org/tools/install)
- **PostgreSQL** server running and accessible
- A PostgreSQL database with at least one table

## Setup

### 1. Clone or create the project

```bash
cd postgres-table-api
```

### 2. Configure environment variables

Copy the example `.env` file and edit it with your database credentials:

```bash
cp .env.example .env
```

Edit `.env`:

```env
DB_HOST=localhost
DB_PORT=5432
DB_NAME=your_database
DB_USER=your_username
DB_PASSWORD=your_password
SERVER_BIND=127.0.0.1:8080
```

### 3. Build the project

```bash
cargo build --release
```

### 4. Run the server

```bash
RUST_LOG=info cargo run --release
```

The server will start at `http://127.0.0.1:8080` (or whatever `SERVER_BIND` you configured).

## API Endpoints

### Health Check

```bash
curl http://localhost:8080/api/health
```

**Response:**
```json
{ "status": "healthy" }
```

### Get Table as JSON

```bash
curl "http://localhost:8080/api/table?schema=public&table=users"
```

Fetch the first 500 rows:

```bash
curl "http://localhost:8080/api/table?schema=public&table=users&limit=500"
```

**Server log output (example):**
```
[INFO]  [public.users] Query completed in 3.456ms | Rows fetched: 42
[INFO]  Request: GET /api/table?schema=public&table=users | Status: 200 | Duration: 4.123ms
```

**Query Parameters:**

| Parameter | Description                                          | Required | Default |
|-----------|------------------------------------------------------|----------|---------|
| `schema`  | PostgreSQL schema name                               | Yes      | —       |
| `table`   | Table name inside that schema                        | Yes      | —       |
| `limit`   | Maximum rows to return (1 – 10 000)                  | No       | 1 000   |

Only alphanumeric characters, underscores (`_`), and hyphens (`-`) are accepted for `schema` and `table`.

**Success Response (200):**
```json
{
  "schema": "public",
  "table": "users",
  "row_count": 3,
  "data": [
    { "id": 1, "name": "Alice", "email": "alice@example.com" },
    { "id": 2, "name": "Bob", "email": "bob@example.com" },
    { "id": 3, "name": "Charlie", "email": "charlie@example.com" }
  ]
}
```

**Error Responses:**
- `400 Bad Request` — Invalid schema or table name (characters outside the allowed set)
- `404 Not Found` — Table does not exist in the given schema
- `500 Internal Server Error` — Database or server error (details logged server-side only)

## Supported Data Types

The API handles the following PostgreSQL types natively:

| PostgreSQL Type                 | JSON Output                |
|--------------------------------|----------------------------|
| `BOOLEAN`                      | `true` / `false`           |
| `SMALLINT`, `INTEGER`, `BIGINT`| Number                     |
| `REAL`, `DOUBLE PRECISION`     | Number                     |
| `TEXT`, `VARCHAR`, `CHAR`      | String                     |
| `JSON`, `JSONB`                | Nested JSON object/array   |
| `TIMESTAMP`, `TIMESTAMPTZ`     | ISO 8601 string            |
| `DATE`                         | ISO 8601 date string       |
| `UUID`                         | String                     |
| Other types                    | String fallback            |

## Development

### Run in debug mode

```bash
cargo run
```

### Run tests

```bash
cargo test
```

### Check code formatting & linting

```bash
cargo fmt
cargo clippy
```

### Logging & Request Timing

Every request is timed by the built-in `RequestTimer` middleware. Additionally, each database query logs its own execution time and the number of rows returned.

Example output:
```
[INFO]  [public.users] Query completed in 3.456ms | Rows fetched: 42
[INFO]  Request: GET /api/table?schema=public&table=users | Status: 200 | Duration: 4.123ms
[INFO]  Request: GET /api/health | Status: 200 | Duration: 0.891ms
```

Control verbosity with `RUST_LOG`:

```bash
RUST_LOG=info cargo run --release
RUST_LOG=debug cargo run          # more verbose
```

## Project Structure

```
.
├── Cargo.toml
├── .env.example
├── README.md
└── src
    └── main.rs
```

## Security Notes

- **Input validation** — Schema and table names are validated against an allowlist (alphanumeric, `_`, `-`) before being interpolated into SQL, preventing SQL injection via identifier names.
- **Row limit** — Every query is capped at 10 000 rows (default 1 000) to prevent DoS via memory exhaustion.
- **Error isolation** — Internal errors (database messages, stack context) are never returned to API clients; they are logged server-side only.
- **Unencrypted connection** — The server currently uses `NoTls`. A warning is printed at startup. For production, configure a TLS connector (e.g. `postgres-openssl` or `postgres-native-tls`).
- **Authentication** — No authentication is included. In production, add an auth layer (API keys, JWT, mutual TLS) in front of this service.
- **Connection pooling** — `deadpool-postgres` is used for efficient connection reuse.
