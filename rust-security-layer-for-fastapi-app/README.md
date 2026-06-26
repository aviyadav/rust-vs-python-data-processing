# Rust Security Layer for FastAPI

> **Demonstration**: Wrapping a dangerous `pickle.loads()` FastAPI endpoint with a Rust-based validation layer that enforces strict CSV parsing and size limits before the Python process ever touches untrusted bytes.

---

## Overview

| Aspect | **Before** | **After** |
|--------|-----------|----------|
| **CSV handling** | Parsed in Python, but also falls back to pickle | Rust validates CSV → returns clean records |
| **Pickle handling** | `pickle.loads()` on unrecognised formats → 💀 RCE | Rejected at Rust layer (not valid CSV) |
| **Size limit** | None (OOM / DoS possible) | 10 MiB hard cap in Rust |
| **Input validation** | None — tries pickle on anything non-CSV | Strict CSV parsing, rejects malformed / binary |
| **Error handling** | 400 for truly invalid files, but pickle path is the vulnerability | 400 / 413 / 502 / 504 depending on failure mode |
| **Architecture** | Monolithic FastAPI (:8001) | FastAPI (:8000) → HTTP → Rust (Axum, :3000) |

### Architecture diagram

```
 BEFORE (insecure)                       AFTER (secure)
 ─────────────────                       ──────────────

  Client                                  Client
    │                                        │
    │  POST /upload (CSV or pickle)          │  POST /upload (CSV only)
    ▼                                        ▼
 ┌──────────────┐                         ┌──────────────┐  POST /validate
 │ FastAPI :8001│                         │ FastAPI :8000│ ─────────────► ┌─────────────────┐
 │              │                         │              │                 │ Rust (Axum)     │
 │ ① try CSV   │                         │ httpx proxy  │ ◄─────────────── │ :3000           │
 │ ② fallback  │  💀 pickle.loads()      │ (no .read()) │  {status,        │                 │
 │    pickle    │                         │              │   records}       │ strict CSV only │
 └──────────────┘                         └──────────────┘                  │ ≤ 10 MiB        │
                                                                           └─────────────────┘
```

---

## Project Structure

```
rust-security-layer-to-fastapi-app/
├── before/                              # "Before" — the vulnerable version
│   ├── main.py                          #   FastAPI: CSV parser + pickle fallback
│   ├── requirements.txt                 #   fastapi, uvicorn, requests, python-multipart
│   └── test_before.py                   #   Tests: CSV, benign pickle, malicious pickle, binary
│
├── after/                               # "After" — the secure version
│   ├── rust_validator/                  #   Rust security layer (Axum 0.7)
│   │   ├── Cargo.toml                   #     Dependencies: axum, tokio, csv, serde
│   │   └── src/
│   │       └── main.rs                  #     GET /health  +  POST /validate
│   └── fastapi_app/                     #   FastAPI gateway
│       ├── main.py                      #     Proxies uploads to Rust via httpx
│       ├── requirements.txt             #     fastapi, uvicorn, httpx, requests
│       ├── test_after.py                #     Tests: CSV passes, pickle/binary/malformed rejected
│       └── debug_rust.py                #     Standalone Rust validator debug tool
│
├── before-rust-fastapi-code.py          # Original reference snippet (before)
├── after-rust-fastapi-code.py           # Original reference snippet (after)
├── validator.rs                         # Original Rust validator reference
├── run_all.bat                          # Windows: one-command build + test + cleanup
├── run_all.sh                           # Linux/macOS: one-command build + test + cleanup
└── README.md                            # ← this file
```

---

## Quick Start

### Prerequisites

- **Rust** (1.75+) — [rustup.rs](https://rustup.rs)
- **Python** 3.10+
- **pip**

### One-command full test

**Windows:**
```batch
run_all.bat
```

**Linux / macOS:**
```bash
chmod +x run_all.sh
./run_all.sh
```

This single script: installs Python deps → builds Rust validator → runs **before** tests → starts Rust → runs **after** tests → cleans up.

---

### Step-by-step manual run

#### 1. Install Python dependencies

```bash
pip install -r before/requirements.txt
pip install -r after/fastapi_app/requirements.txt
```

#### 2. Build the Rust validator

```bash
cd after/rust_validator
cargo build --release
```

Binary: `after/rust_validator/target/release/rust-validator` (`.exe` on Windows).

#### 3. Run the BEFORE tests (vulnerable app)

```bash
python before/test_before.py
```

Expected output:

```
============================================================
Testing BEFORE (vulnerable) FastAPI app
============================================================
  ✓ Health check passed
  ✓ Valid CSV accepted (200 OK) — 4 rows returned
  ✓ Benign pickle accepted (200 OK) — pickle fallback works
  ⚠  Malicious pickle ACCEPTED — arbitrary code executed!
  ✓ Random binary correctly rejected (400 Bad Request)

✅ All before tests passed (vulnerability confirmed).
```

#### 4. Start the Rust validator (separate terminal)

```bash
cd after/rust_validator
cargo run
# → 🛡  Rust validator listening on http://127.0.0.1:3000
```

#### 5. Test the Rust validator directly (optional debug step)

```bash
python after/fastapi_app/debug_rust.py
```

This bypasses FastAPI entirely and tests the Rust validator in isolation.

#### 6. Run the AFTER tests (secure app)

```bash
python after/fastapi_app/test_after.py
```

Expected output:

```
============================================================
Testing AFTER (secure) FastAPI app
============================================================
  ✓ Rust validator health check passed
  ✓ FastAPI health check passed (Rust validator reachable)
  ✓ Valid CSV accepted (200 OK) — records returned
  ✓ Pickle payload REJECTED (400 Bad Request)
  ✓ Malformed CSV REJECTED (400 Bad Request)
  ✓ Random binary REJECTED (400 Bad Request)

✅ All after tests passed (security layer working).
```

---

## What the Tests Prove

### Before (vulnerable)

| Test | Result | What it means |
|------|--------|---------------|
| Valid CSV (3 rows + header) | ✅ 200 OK — 4 rows returned | CSV is the intended format and works normally |
| Benign pickle (`{"message": "hello"}`) | ✅ 200 OK | Pickle fallback deserialises unknown formats |
| Malicious pickle (`eval("2+2")`) | ✅ 200 OK (returns `4`) | **Arbitrary code executed** — `eval()` was called during `pickle.loads()` |
| Random binary (256 bytes) | ❌ 400 — graceful rejection | Neither CSV nor valid pickle → proper error |

### After (secure) — Rust validator running required

| Test | Result | What it means |
|------|--------|---------------|
| Valid CSV (3 rows) | ✅ 200 OK | Clean data passes through, records returned |
| Pickle payload | ❌ 400 Rejected | Binary/non-CSV blocked at Rust layer |
| Malformed CSV (uneven columns) | ❌ 400 Rejected | Strict mode catches structural issues |
| Random binary | ❌ 400 Rejected | Any non-CSV data is rejected |

---

## Key Security Design Decisions

### 1. Rust validates before Python touches bytes

The after FastAPI app **never calls `file.read()`**. It passes `file.file` (a `SpooledTemporaryFile`) directly to `httpx`, which streams it to the Rust validator. The raw bytes never enter Python's object space. If Rust returns anything other than 200, FastAPI raises an error immediately.

```python
# after/fastapi_app/main.py — the secure path
file_obj = file.file               # never read() in Python
response = await client.post(       # streamed directly to Rust
    RUST_VALIDATOR_URL,
    files={"file": (filename, file_obj, content_type)},
)
if response.status_code != 200:
    raise HTTPException(400, ...)   # blocked before touching raw bytes
return response.json()              # only parsed JSON, never raw bytes
```

### 2. Strict CSV parsing (no "flexible" mode)

The Rust validator uses the `csv` crate with **default** settings:

```rust
let mut reader = csv::ReaderBuilder::new();
reader.buffer_capacity(1024 * 8);
let mut csv_reader = reader.from_reader(data.as_ref());
```

- **No `flexible()`** — every row must have exactly the same number of fields.
- **No custom deserialisation** — only `String` fields are extracted.
- **Bounded buffer** — 8 KiB IO buffer prevents unbounded memory use during parsing.

### 3. Hard size cap before parsing

```rust
if data.len() > 10_000_000 {
    return Err(StatusCode::PAYLOAD_TOO_LARGE);  // → 413
}
```

The 10 MiB limit is enforced **before** the CSV parser is invoked, preventing resource-exhaustion attacks.

### 4. Separate process boundary

The Rust validator runs as a separate OS process on a different port (3000). Even if a vulnerability were discovered in the Rust CSV parser, it would be contained within the Rust process — it cannot directly corrupt the Python FastAPI process memory.

### 5. No dangerous Python deserialisation

The after FastAPI app uses **zero** of the following:
- `pickle.loads()` / `pickle.load()`
- `yaml.load()` (unsafe YAML)
- `eval()` / `exec()`
- `marshal.loads()`
- Any other code-executing deserialiser

It only calls `response.json()` on data already validated by Rust.

### 6. Defensive `None` handling in the proxy

```python
filename = file.filename or "uploaded_file"
content_type = file.content_type or "application/octet-stream"
```

Both values can be `None` from certain clients; the app provides safe defaults before passing them to httpx.

---

## API Reference

### Rust Validator (port 3000)

| Endpoint | Method | Input | Success | Errors |
|----------|--------|-------|---------|--------|
| `/health` | GET | — | `200`: `{"status":"ok"}` | — |
| `/validate` | POST | `multipart/form-data` (field: `file`) | `200`: `{"status":"clean","records":[...],"row_count":N}` | `400` invalid CSV / no file, `413` file > 10 MiB |

### FastAPI — Before (port 8001)

| Endpoint | Method | Input | Success | Errors |
|----------|--------|-------|---------|--------|
| `/health` | GET | — | `200`: `{"status":"ok"}` | — |
| `/upload` | POST | `multipart/form-data` (CSV or pickle) | `200`: `{"status":"processed","format":"csv\|pickle","data":...}` | `400` neither CSV nor pickle |

### FastAPI — After (port 8000)

| Endpoint | Method | Input | Success | Errors |
|----------|--------|-------|---------|--------|
| `/health` | GET | — | `200`: `{"status":"ok","rust_validator":"reachable\|unreachable"}` | — |
| `/upload` | POST | `multipart/form-data` (CSV only) | `200`: `{"status":"clean","records":[...],"row_count":N}` | `400` rejected by Rust, `413` too large, `502` Rust down, `504` Rust timeout |

---

## Development

### Running the Rust validator in dev mode

```bash
cd after/rust_validator
cargo run
# → 🛡  Rust validator listening on http://127.0.0.1:3000
```

### Running either FastAPI app in dev mode

```bash
# Before (vulnerable) — port 8001
cd before
uvicorn main:app --reload --host 127.0.0.1 --port 8001

# After (secure) — port 8000 (requires Rust validator on :3000)
cd after/fastapi_app
uvicorn main:app --reload --host 127.0.0.1 --port 8000
```

### Debugging the Rust validator directly

```bash
cd after/fastapi_app
python debug_rust.py
```

Runs four standalone tests against the Rust validator with no FastAPI in the middle — useful for isolating whether a failure is in Rust or in the proxy chain.

### Manual curl tests

```bash
# ── Rust validator directly ──────────────────────────────

# Create a test CSV
printf "id,name,email\n1,Alice,alice@ex.com\n2,Bob,bob@ex.com" > /tmp/test.csv

# Valid CSV → 200 with records
curl -s -X POST http://127.0.0.1:3000/validate \
  -F "file=@/tmp/test.csv;type=text/csv" | python -m json.tool

# Binary junk → 400
curl -s -o /dev/null -w "%{http_code}\n" -X POST http://127.0.0.1:3000/validate \
  -F "file=@/dev/urandom;type=application/octet-stream"

# Pickle payload → 400
python -c "import pickle; open('/tmp/e.pkl','wb').write(pickle.dumps({'x':1}))"
curl -s -X POST http://127.0.0.1:3000/validate -F "file=@/tmp/e.pkl"

# Empty file → 400 (no CSV records)
curl -s -X POST http://127.0.0.1:3000/validate -F "file=@/dev/null;type=text/csv"

# ── Before FastAPI app ───────────────────────────────────

# CSV → 200
curl -s -X POST http://127.0.0.1:8001/upload \
  -F "file=@/tmp/test.csv;type=text/csv" | python -m json.tool

# Pickle → 200 (vulnerability!)
python -c "import pickle; open('/tmp/e.pkl','wb').write(pickle.dumps({'x':1}))"
curl -s -X POST http://127.0.0.1:8001/upload -F "file=@/tmp/e.pkl"

# ── After FastAPI app (requires Rust on :3000) ───────────

# CSV → 200
curl -s -X POST http://127.0.0.1:8000/upload \
  -F "file=@/tmp/test.csv;type=text/csv" | python -m json.tool

# Pickle → 400 (blocked by Rust!)
curl -s -X POST http://127.0.0.1:8000/upload -F "file=@/tmp/e.pkl"

# Health check
curl -s http://127.0.0.1:8000/health | python -m json.tool
```

**Windows PowerShell equivalents:**

```powershell
@"
id,name,email
1,Alice,alice@ex.com
2,Bob,bob@ex.com
"@ | Set-Content test.csv

curl.exe -s -X POST http://127.0.0.1:3000/validate -F "file=@test.csv;type=text/csv"
curl.exe -s http://127.0.0.1:8000/health
```

---

## License

This project is a demonstration / educational reference. Use it to understand how a Rust security layer can harden a Python web application against deserialisation attacks.
