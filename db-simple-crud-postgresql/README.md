# Clinical Trial CRUD API

Rust-based CRUD application for PostgreSQL clinical trial tables with concurrent load testing and REST API with OpenAPI/Swagger documentation.

## Prerequisites

- **Rust** 1.80+ (edition 2021)
- **PostgreSQL** with the `benchmark_poc_db` database and tables from `db/tables.sql`

## Database Configuration

| Setting  | Value              |
|----------|--------------------|
| Host     | `localhost`        |
| Port     | `5432`             |
| Database | `benchmark_poc_db` |
| User     | `pocuser`          |
| Password | `password`         |

### Collation Version Mismatch

You may see this warning in the logs:

```
tokio_postgres::connection: WARNING: database "benchmark_poc_db" has a collation version mismatch
```

This occurs when PostgreSQL was upgraded or the database was restored from a different environment. It has **no functional impact** — all CRUD operations work correctly. However, string sorting and text indexes could be affected in edge cases.

**Fix** — run as a superuser (`postgres`):

```bash
psql -U postgres -d benchmark_poc_db -c "ALTER DATABASE benchmark_poc_db REFRESH COLLATION VERSION;"
```

**Suppress in application logs** (optional) — filter the warning in `src/main.rs`:

```rust
tracing_subscriber::fmt()
    .with_env_filter(
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info,tokio_postgres::connection=error")),
    )
    .init();
```

## Tables

The application operates on 6 clinical trial (SDTM-like) tables:

| Table | Description            | Approx. Rows |
|-------|------------------------|-------------|
| `ae`  | Adverse Events         | 150,019     |
| `cm`  | Concomitant Medications| 50,087      |
| `dm`  | Demographics           | 20,001      |
| `lb`  | Laboratory             | 1,612,117   |
| `tv`  | Trial Visits           | 2,945       |
| `vs`  | Vital Signs            | 2,686,861   |

**Note:** These tables have no primary keys. The CRUD operations use composite key columns for row identification:

| Table | Key Columns                               |
|-------|-------------------------------------------|
| `ae`  | `STUDYID`, `USUBJID`, `AESEQ`            |
| `cm`  | `STUDYID`, `USUBJID`, `CMSEQ`            |
| `dm`  | `STUDYID`, `USUBJID`                      |
| `lb`  | `STUDYID`, `USUBJID`, `LBTESTCD`, `LBDTC`|
| `tv`  | `STUDYID`, `SITE`, `SUBJECT`, `VISIT`     |
| `vs`  | `STUDYID`, `USUBJID`, `VSTESTCD`, `VSDTC` |

## Project Structure

```
simple-crud-postgresql/
├── Cargo.toml
├── README.md
├── db/
│   └── tables.sql          # Table DDL definitions
└── src/
    ├── main.rs              # CLI entry point
    ├── db.rs                # Connection pool, models, CRUD functions
    ├── api.rs               # Axum REST API + OpenAPI + Swagger UI
    └── load_test.rs         # Functional tests + load testing framework
```

## Dependencies

| Crate             | Purpose                          |
|-------------------|----------------------------------|
| `axum 0.8`        | HTTP framework                   |
| `tokio-postgres`  | Async PostgreSQL driver          |
| `deadpool-postgres`| Connection pooling               |
| `utoipa 5`        | OpenAPI spec generation          |
| `serde` / `serde_json` | Serialization              |
| `chrono`          | Date/time handling               |
| `clap`            | CLI argument parsing             |
| `rand`            | Random data generation for tests |
| `tracing`         | Structured logging               |

## Build

```bash
cd simple-crud-postgresql
cargo build --release
```

## Run

### 1. Functional Tests

Runs create, read, update, and delete operations against all 6 tables sequentially.

```bash
cargo run -- test
```

**Expected output:**

```
=== Running Functional CRUD Tests ===
Testing table: ae
  [CREATE] Inserting test record into ae
  [CREATE] ✓ Inserted 1 row(s)
  [READ] Listing records from ae
  [READ] ✓ Found 150019 record(s) total
  [UPDATE] Updating record in ae
  [UPDATE] ✓ Updated 1 row(s)
  [DELETE] Deleting record from ae
  [DELETE] ✓ Deleted 1 row(s)
... (repeats for cm, dm, lb, tv, vs)
=== All Functional Tests PASSED ===
```

### 2. Load Testing

Configurable concurrent load testing with tunable read/write/update/delete ratios.

```bash
# Basic load test with defaults
cargo run -- load-test

# Custom load test
cargo run -- load-test \
  --clients 50 \
  --operations 1000 \
  --table dm \
  --read-ratio 0.5 \
  --write-ratio 0.3 \
  --update-ratio 0.15 \
  --delete-ratio 0.05
```

**Parameters:**

| Argument        | Default | Description                        |
|-----------------|---------|------------------------------------|
| `--clients`     | 50      | Number of concurrent clients       |
| `--operations`  | 1000    | Total operations across all clients|
| `--table`       | dm      | Table to test (`ae`/`cm`/`dm`/`lb`/`tv`/`vs`) |
| `--read-ratio`  | 0.6     | Fraction of read operations        |
| `--write-ratio` | 0.2     | Fraction of write operations       |
| `--update-ratio`| 0.15    | Fraction of update operations      |
| `--delete-ratio`| 0.05    | Fraction of delete operations      |

**Sample results (dm table, 50 clients, 1000 ops):**

```
=== Load Test Results ===
  Total time:        447.211555ms
  Total operations:  1000
  Total failures:    0
  Throughput:        2236.08 ops/sec

  READ:   total=484, success=484, fail=0, avg=14.00ms, min=3.38ms, max=40.84ms
  WRITE:  total=306, success=306, fail=0, avg=11.74ms, min=1.01ms, max=36.62ms
  UPDATE: total=158, success=158, fail=0, avg=24.33ms, min=10.30ms, max=50.84ms
  DELETE: total=52,  success=52,  fail=0, avg=24.65ms, min=0.00ms, max=49.27ms
```

### 3. REST API Server

Starts the API server with Swagger UI.

```bash
cargo run -- server --host 0.0.0.0 --port 8080
```

Then open **http://localhost:8080/swagger-ui** for interactive API documentation.

---

## REST API Endpoints

Base URL: `http://localhost:8080`

### Health Check

**`GET /api/health`**

```bash
curl -s http://localhost:8080/api/health
```

**Response:** `OK`

---

### List Records (with pagination & filtering)

**`GET /api/{table}`**

Query parameters: `page`, `page_size`, `study`, `site`, `subject`, `visit`, `domain`, `studyid`, `usubjid`, `siteid`

```bash
# List first 2 DM records
curl -s "http://localhost:8080/api/dm?page=1&page_size=2" | python3 -m json.tool

# Filter by STUDYID
curl -s "http://localhost:8080/api/dm?studyid=STUDY-001&page_size=3" | python3 -m json.tool

# Filter by USUBJID
curl -s "http://localhost:8080/api/ae?usubjid=STUDY-001-SITE-001-0095314c&page_size=2" | python3 -m json.tool
```

**Response:**
```json
{
    "data": [
        {
            "AGE": 33,
            "ARM": "Active 20mg",
            "COUNTRY": "BH",
            "DMDTC": "2024-09-18",
            "DOMAIN": "DM",
            "FORM": "DM",
            "RACE": "ASIAN",
            "SEX": "F",
            "SITE": "SITE-001",
            "SITEID": "SITE-001",
            "STUDY": "STUDY-001",
            "STUDYID": "STUDY-001",
            "SUBJECT": "STUDY-001-SITE-001-0095314c",
            "USUBJID": "STUDY-001-SITE-001-0095314c",
            "VISIT": "SCREENING"
        }
    ],
    "total": 20001,
    "page": 1,
    "page_size": 2
}
```

---

### Create Record

**`POST /api/{table}`**

#### DM (Demographics)

```bash
curl -s -X POST http://localhost:8080/api/dm \
  -H "Content-Type: application/json" \
  -d '{
    "study": "STUDY-API",
    "site": "SITE-01",
    "subject": "API-SUBJ-001",
    "visit": "SCREENING",
    "form": "DM",
    "domain": "DM",
    "age": 42,
    "sex": "M",
    "race": "ASIAN",
    "country": "IND",
    "dmdtc": "2024-06-15",
    "arm": "ARM1",
    "siteid": "SITE-01",
    "studyid": "STUDY-API",
    "usubjid": "API-USUBJ-001"
  }'
```

#### AE (Adverse Events)

```bash
curl -s -X POST http://localhost:8080/api/ae \
  -H "Content-Type: application/json" \
  -d '{
    "study": "STUDY-API",
    "site": "SITE-01",
    "subject": "API-SUBJ-001",
    "visit": "VISIT1",
    "form": "AE",
    "domain": "AE",
    "aeseq": 1,
    "aeterm": "Headache",
    "aedecod": "Headache",
    "aebodsys": "Nervous system disorders",
    "aestdtc": "2024-01-15",
    "aeendtc": "2024-01-16",
    "aesev": "MODERATE",
    "aerel": "POSSIBLE",
    "aeout": "RECOVERED",
    "siteid": "SITE-01",
    "studyid": "STUDY-API",
    "usubjid": "API-USUBJ-001"
  }'
```

#### CM (Concomitant Medications)

```bash
curl -s -X POST http://localhost:8080/api/cm \
  -H "Content-Type: application/json" \
  -d '{
    "study": "STUDY-API",
    "site": "SITE-01",
    "subject": "API-SUBJ-001",
    "visit": "VISIT1",
    "form": "CM",
    "domain": "CM",
    "cmseq": 1,
    "cmtrt": "Aspirin",
    "cmdecod": "Acetylsalicylic acid",
    "cmcat": "ANALGESIC",
    "cmstdtc": "2024-01-10",
    "cmendtc": "2024-01-20",
    "cmdose": 100,
    "cmdosu": "mg",
    "cmdosfrm": "TABLET",
    "cmroute": "ORAL",
    "cmdosfrq": "QD",
    "siteid": "SITE-01",
    "studyid": "STUDY-API",
    "usubjid": "API-USUBJ-001"
  }'
```

#### LB (Laboratory)

```bash
curl -s -X POST http://localhost:8080/api/lb \
  -H "Content-Type: application/json" \
  -d '{
    "study": "STUDY-API",
    "site": "SITE-01",
    "subject": "API-SUBJ-001",
    "visit": "VISIT1",
    "form": "LB",
    "domain": "LB",
    "lbtestcd": "GLUC",
    "lbtest": "Glucose",
    "lborres": 95.5,
    "lborresu": "mg/dL",
    "lbstnrlo": 70,
    "lbstnrhi": 110,
    "lbdtc": "2024-01-15",
    "siteid": "SITE-01",
    "studyid": "STUDY-API",
    "usubjid": "API-USUBJ-001"
  }'
```

#### TV (Trial Visits)

```bash
curl -s -X POST http://localhost:8080/api/tv \
  -H "Content-Type: application/json" \
  -d '{
    "study": "STUDY-API",
    "site": "SITE-01",
    "subject": "API-SUBJ-001",
    "visit": "VISIT1",
    "form": "TV",
    "domain": "TV",
    "visitnum": 1,
    "tvstrl": 1,
    "tvenrl": 10,
    "armcd": "ARM1",
    "studyid": "STUDY-API"
  }'
```

#### VS (Vital Signs)

```bash
curl -s -X POST http://localhost:8080/api/vs \
  -H "Content-Type: application/json" \
  -d '{
    "study": "STUDY-API",
    "site": "SITE-01",
    "subject": "API-SUBJ-001",
    "visit": "VISIT1",
    "form": "VS",
    "domain": "VS",
    "vstestcd": "SYSBP",
    "vstest": "Systolic Blood Pressure",
    "vsorres": 120,
    "vsorresu": "mmHg",
    "vsdtc": "2024-01-15",
    "siteid": "SITE-01",
    "studyid": "STUDY-API",
    "usubjid": "API-USUBJ-001"
  }'
```

**Response (all create endpoints):**
```json
{"affected_rows": 1}
```

---

### Update Record

**`PUT /api/{table}`**

The request body must include the key columns to identify which rows to update, plus the fields to modify.

#### DM Update (key: `studyid` + `usubjid`)

```bash
curl -s -X PUT http://localhost:8080/api/dm \
  -H "Content-Type: application/json" \
  -d '{
    "studyid": "STUDY-API",
    "usubjid": "API-USUBJ-001",
    "age": 99,
    "race": "WHITE"
  }'
```

#### AE Update (key: `studyid` + `usubjid` + `aeseq`)

```bash
curl -s -X PUT http://localhost:8080/api/ae \
  -H "Content-Type: application/json" \
  -d '{
    "studyid": "STUDY-API",
    "usubjid": "API-USUBJ-001",
    "aeseq": 1,
    "aesev": "SEVERE",
    "aeout": "NOT RECOVERED"
  }'
```

#### CM Update (key: `studyid` + `usubjid` + `cmseq`)

```bash
curl -s -X PUT http://localhost:8080/api/cm \
  -H "Content-Type: application/json" \
  -d '{
    "studyid": "STUDY-API",
    "usubjid": "API-USUBJ-001",
    "cmseq": 1,
    "cmdose": 200,
    "cmdosu": "mg"
  }'
```

#### LB Update (key: `studyid` + `usubjid` + `lbtestcd` + `lbdtc`)

```bash
curl -s -X PUT http://localhost:8080/api/lb \
  -H "Content-Type: application/json" \
  -d '{
    "studyid": "STUDY-API",
    "usubjid": "API-USUBJ-001",
    "lbtestcd": "GLUC",
    "lbdtc": "2024-01-15",
    "lborres": 110.5,
    "lbstnrhi": 120
  }'
```

#### TV Update (key: `studyid` + `site` + `subject` + `visit`)

```bash
curl -s -X PUT http://localhost:8080/api/tv \
  -H "Content-Type: application/json" \
  -d '{
    "studyid": "STUDY-API",
    "site": "SITE-01",
    "subject": "API-SUBJ-001",
    "visit": "VISIT1",
    "tvenrl": 20,
    "armcd": "ARM2"
  }'
```

#### VS Update (key: `studyid` + `usubjid` + `vstestcd` + `vsdtc`)

```bash
curl -s -X PUT http://localhost:8080/api/vs \
  -H "Content-Type: application/json" \
  -d '{
    "studyid": "STUDY-API",
    "usubjid": "API-USUBJ-001",
    "vstestcd": "SYSBP",
    "vsdtc": "2024-01-15",
    "vsorres": 130,
    "vsorresu": "mmHg"
  }'
```

**Response (all update endpoints):**
```json
{"affected_rows": 1}
```

---

### Delete Record

**`DELETE /api/{table}`**

The request body must include the key columns to identify which rows to delete.

```bash
# DM Delete (key: studyid + usubjid)
curl -s -X DELETE http://localhost:8080/api/dm \
  -H "Content-Type: application/json" \
  -d '{
    "studyid": "STUDY-API",
    "usubjid": "API-USUBJ-001"
  }'

# AE Delete (key: studyid + usubjid + aeseq)
curl -s -X DELETE http://localhost:8080/api/ae \
  -H "Content-Type: application/json" \
  -d '{
    "studyid": "STUDY-API",
    "usubjid": "API-USUBJ-001",
    "aeseq": 1
  }'

# CM Delete (key: studyid + usubjid + cmseq)
curl -s -X DELETE http://localhost:8080/api/cm \
  -H "Content-Type: application/json" \
  -d '{
    "studyid": "STUDY-API",
    "usubjid": "API-USUBJ-001",
    "cmseq": 1
  }'

# LB Delete (key: studyid + usubjid + lbtestcd + lbdtc)
curl -s -X DELETE http://localhost:8080/api/lb \
  -H "Content-Type: application/json" \
  -d '{
    "studyid": "STUDY-API",
    "usubjid": "API-USUBJ-001",
    "lbtestcd": "GLUC",
    "lbdtc": "2024-01-15"
  }'

# TV Delete (key: studyid + site + subject + visit)
curl -s -X DELETE http://localhost:8080/api/tv \
  -H "Content-Type: application/json" \
  -d '{
    "studyid": "STUDY-API",
    "site": "SITE-01",
    "subject": "API-SUBJ-001",
    "visit": "VISIT1"
  }'

# VS Delete (key: studyid + usubjid + vstestcd + vsdtc)
curl -s -X DELETE http://localhost:8080/api/vs \
  -H "Content-Type: application/json" \
  -d '{
    "studyid": "STUDY-API",
    "usubjid": "API-USUBJ-001",
    "vstestcd": "SYSBP",
    "vsdtc": "2024-01-15"
  }'
```

**Response (all delete endpoints):**
```json
{"affected_rows": 1}
```

---

### OpenAPI / Swagger

| Endpoint                    | Description                |
|-----------------------------|----------------------------|
| `GET /api-docs/openapi.json`| OpenAPI 3.1.0 specification |
| `GET /swagger-ui`           | Interactive Swagger UI     |

```bash
# Get OpenAPI spec
curl -s http://localhost:8080/api-docs/openapi.json | python3 -m json.tool | head -30
```

Open **http://localhost:8080/swagger-ui** in a browser to explore and test all endpoints interactively.

---

## Load Test Results Summary

| Test  | Table  | Rows      | Clients | Ops   | Failures | Throughput  |
|-------|--------|-----------|---------|-------|----------|-------------|
| Small | `dm`   | 20,001    | 20      | 200   | 0        | 1,631/s     |
| Medium| `dm`   | 20,001    | 50      | 1,000 | 0        | 2,236/s     |
| Large | `lb`   | 1,612,117 | 30      | 300   | 0        | 23.6/s      |

> **Note:** The `lb` table (1.6M rows) throughput is lower due to full table scans on unindexed filter columns. Adding indexes on `STUDYID`, `USUBJID`, etc. would significantly improve performance on large tables.

## Error Response Format

All errors return with appropriate HTTP status codes:

```json
{
    "error": "No key columns provided for UPDATE — cannot identify which rows to update"
}
```
