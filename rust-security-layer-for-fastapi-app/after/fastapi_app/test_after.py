"""
Test script for the AFTER (secure) FastAPI application.

Requires the Rust validator to be running on port 3000 first:
    cd after/rust_validator && cargo run

What it proves:
  1. Valid CSV uploads pass through the Rust validator → 200 OK.
  2. Malicious pickle payloads are rejected → 400 Bad Request.
  3. Non-CSV binary data is rejected → 400 Bad Request.
  4. Malformed CSV (uneven rows) is rejected → 400 Bad Request.

Run with:
    python test_after.py
"""

import csv
import io
import pickle
import subprocess
import sys
import time
from pathlib import Path

import requests
from requests import exceptions as requests_exceptions

SERVER_HOST = "127.0.0.1"
SERVER_PORT = 8000
BASE_URL = f"http://{SERVER_HOST}:{SERVER_PORT}"


def start_server():
    """Start the after FastAPI server as a subprocess."""
    server_script = Path(__file__).parent / "main.py"
    proc = subprocess.Popen(
        [sys.executable, str(server_script)],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    # Retry until the server responds (up to 10 seconds).
    for _ in range(10):
        time.sleep(1)
        try:
            r = requests.get(f"{BASE_URL}/health", timeout=2)
            if r.status_code == 200:
                return proc
        except requests_exceptions.RequestException:
            pass
    proc.terminate()
    raise RuntimeError("After FastAPI server did not start within 10 seconds")


def stop_server(proc: subprocess.Popen):
    """Gracefully terminate the server."""
    proc.terminate()
    proc.wait(timeout=5)


def test_rust_validator_reachable():
    """Verify the Rust validator is up (port 3000)."""
    try:
        resp = requests.get("http://127.0.0.1:3000/health", timeout=3)
        assert resp.status_code == 200
        print("  ✓ Rust validator health check passed")
    except (requests.ConnectionError, requests.Timeout, requests_exceptions.RequestException):
        print("  ✗ Rust validator is NOT running on port 3000!")
        print("    Start it first: cd after/rust_validator && cargo run")
        sys.exit(1)


def test_health():
    """Verify the FastAPI server is reachable and sees the Rust validator."""
    resp = requests.get(f"{BASE_URL}/health", timeout=5)
    assert resp.status_code == 200
    body = resp.json()
    assert body["status"] == "ok"
    assert body["rust_validator"] == "reachable"
    print("  ✓ FastAPI health check passed (Rust validator reachable)")


def make_valid_csv(num_rows: int = 5) -> bytes:
    """Produce a well-formed CSV file with headers + N data rows."""
    buf = io.StringIO()
    writer = csv.writer(buf)
    writer.writerow(["id", "name", "email"])
    for i in range(1, num_rows + 1):
        writer.writerow([str(i), f"User {i}", f"user{i}@example.com"])
    return buf.getvalue().encode("utf-8")


def make_malformed_csv() -> bytes:
    """Produce CSV where rows have different column counts (strict mode rejects)."""
    buf = io.StringIO()
    writer = csv.writer(buf)
    writer.writerow(["id", "name", "email"])      # 3 columns
    writer.writerow(["1", "User 1"])               # 2 columns → malformed
    writer.writerow(["2", "User 2", "a@b.com"])   # 3 columns again
    return buf.getvalue().encode("utf-8")


def make_pickle_payload() -> bytes:
    """Create a pickle payload (the same kind the before app blindly accepts)."""
    return pickle.dumps({"evil": True})


def test_upload_valid_csv():
    """A well-formed CSV should pass the Rust validator."""
    csv_data = make_valid_csv(3)

    resp = requests.post(
        f"{BASE_URL}/upload",
        files={"file": ("data.csv", csv_data, "text/csv")},
        timeout=10,
    )

    assert resp.status_code == 200, f"Expected 200, got {resp.status_code}: {resp.text}"
    body = resp.json()
    assert body["status"] == "clean"
    assert body["row_count"] == 3
    assert len(body["records"]) == 3
    assert body["records"][0] == ["1", "User 1", "user1@example.com"]
    print("  ✓ Valid CSV accepted (200 OK) — records returned")


def test_upload_pickle_rejected():
    """A pickle should be rejected — it is not valid CSV."""
    payload = make_pickle_payload()

    resp = requests.post(
        f"{BASE_URL}/upload",
        files={"file": ("evil.pkl", payload, "application/octet-stream")},
        timeout=10,
    )

    assert resp.status_code == 400, f"Expected 400, got {resp.status_code}"
    assert "rejected" in resp.text.lower()
    print("  ✓ Pickle payload REJECTED (400 Bad Request)")


def test_upload_malformed_csv_rejected():
    """Malformed CSV (uneven columns) should be rejected in strict mode."""
    csv_data = make_malformed_csv()

    resp = requests.post(
        f"{BASE_URL}/upload",
        files={"file": ("bad.csv", csv_data, "text/csv")},
        timeout=10,
    )

    assert resp.status_code == 400, f"Expected 400, got {resp.status_code}"
    print("  ✓ Malformed CSV REJECTED (400 Bad Request)")


def test_upload_binary_rejected():
    """Random binary bytes should be rejected."""
    import os
    binary_data = os.urandom(1024)

    resp = requests.post(
        f"{BASE_URL}/upload",
        files={"file": ("random.bin", binary_data, "application/octet-stream")},
        timeout=10,
    )

    assert resp.status_code == 400, f"Expected 400, got {resp.status_code}"
    print("  ✓ Random binary REJECTED (400 Bad Request)")


def main():
    print("=" * 60)
    print("Testing AFTER (secure) FastAPI app")
    print("=" * 60)

    test_rust_validator_reachable()

    server = start_server()
    try:
        test_health()
        test_upload_valid_csv()
        test_upload_pickle_rejected()
        test_upload_malformed_csv_rejected()
        test_upload_binary_rejected()
        print("\n✅ All after tests passed (security layer working).")
    finally:
        stop_server(server)


if __name__ == "__main__":
    main()
