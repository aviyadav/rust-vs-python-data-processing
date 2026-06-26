"""
Debug script — test the Rust validator directly (no FastAPI involved).

Use this to isolate whether the problem is in Rust or in the Python→Rust chain.

Usage:
    python debug_rust.py

The Rust validator must be running on port 3000 first:
    cd after/rust_validator && cargo run
"""

import csv
import io
import sys

import requests

RUST_VALIDATE = "http://127.0.0.1:3000/validate"
RUST_HEALTH = "http://127.0.0.1:3000/health"


def make_csv(num_rows: int = 3) -> bytes:
    buf = io.StringIO()
    writer = csv.writer(buf)
    writer.writerow(["id", "name", "email"])
    for i in range(1, num_rows + 1):
        writer.writerow([str(i), f"User {i}", f"user{i}@example.com"])
    return buf.getvalue().encode("utf-8")


def test_health():
    print("[1] Checking Rust validator health...")
    try:
        resp = requests.get(RUST_HEALTH, timeout=3)
        assert resp.status_code == 200
        assert resp.json()["status"] == "ok"
        print("    ✓ Rust validator is healthy")
    except Exception as e:
        print(f"    ✗ FAILED: {e}")
        sys.exit(1)


def test_valid_csv():
    print("[2] Uploading valid CSV (3 rows)...")
    csv_data = make_csv(3)
    print(f"    CSV payload: {len(csv_data)} bytes")
    print(f"    First 80 chars: {csv_data[:80]!r}")

    try:
        resp = requests.post(
            RUST_VALIDATE,
            files={"file": ("test.csv", csv_data, "text/csv")},
            timeout=10,
        )
    except requests.ConnectionError:
        print("    ✗ FAILED: Cannot connect to Rust validator on port 3000")
        sys.exit(1)

    print(f"    Status: {resp.status_code}")
    print(f"    Body: {resp.text[:500]}")
    if resp.status_code == 200:
        body = resp.json()
        print(f"    Parsed: status={body.get('status')}, row_count={body.get('row_count')}")
        print(f"    Records: {body.get('records')}")
        print("    ✓ Valid CSV accepted")
    else:
        print("    ✗ FAILED: expected 200")
        sys.exit(1)


def test_pickle_rejected():
    import pickle
    print("[3] Uploading pickle payload (should be rejected)...")
    payload = pickle.dumps({"evil": True})

    try:
        resp = requests.post(
            RUST_VALIDATE,
            files={"file": ("evil.pkl", payload, "application/octet-stream")},
            timeout=10,
        )
    except requests.ConnectionError:
        print("    ✗ FAILED: Cannot connect to Rust validator")
        sys.exit(1)

    print(f"    Status: {resp.status_code}")
    if resp.status_code == 400:
        print("    ✓ Pickle correctly rejected (400)")
    else:
        print(f"    ✗ FAILED: expected 400, got {resp.status_code}: {resp.text[:200]}")
        sys.exit(1)


def test_empty_file():
    print("[4] Uploading empty file...")
    try:
        resp = requests.post(
            RUST_VALIDATE,
            files={"file": ("empty.csv", b"", "text/csv")},
            timeout=10,
        )
    except requests.ConnectionError:
        print("    ✗ FAILED: Cannot connect")
        sys.exit(1)

    print(f"    Status: {resp.status_code}")
    if resp.status_code == 400:
        print("    ✓ Empty file correctly rejected (no CSV records)")
    elif resp.status_code == 200:
        body = resp.json()
        print(f"    Body: {body}")
        if body.get("row_count") == 0 and body.get("status") == "clean":
            print("    ✓ Empty file accepted with 0 rows (valid behavior)")
        else:
            print("    (unexpected but not an error)")
    else:
        print(f"    (unexpected status {resp.status_code}: {resp.text[:200]})")


if __name__ == "__main__":
    print("=" * 60)
    print("Debug: testing Rust validator directly on port 3000")
    print("=" * 60)
    test_health()
    test_valid_csv()
    test_pickle_rejected()
    test_empty_file()
    print("\n✅ All direct Rust validator tests passed.")
