"""
Test script for the BEFORE (vulnerable) FastAPI application.

What it proves:
  1. Valid CSV uploads are parsed and returned (200 OK).
  2. Pickle payloads are blindly deserialised (200 OK) — the vulnerability.
  3. Malicious pickle code executes during deserialisation (200 OK).
  4. Truly invalid files get a 400 error (graceful rejection).

Run with:
    python test_before.py
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
SERVER_PORT = 8001
BASE_URL = f"http://{SERVER_HOST}:{SERVER_PORT}"


def start_server():
    """Start the before FastAPI server as a subprocess."""
    server_script = Path(__file__).parent / "main.py"
    proc = subprocess.Popen(
        [sys.executable, str(server_script)],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    for _ in range(10):
        time.sleep(1)
        try:
            r = requests.get(f"{BASE_URL}/health", timeout=2)
            if r.status_code == 200:
                return proc
        except requests_exceptions.RequestException:
            pass
    proc.terminate()
    raise RuntimeError("Before FastAPI server did not start within 10 seconds")


def stop_server(proc: subprocess.Popen):
    """Gracefully terminate the server."""
    proc.terminate()
    proc.wait(timeout=5)


# ── Helpers ─────────────────────────────────────────────────────────────

def make_valid_csv(num_rows: int = 3) -> bytes:
    """Well-formed CSV with headers + data rows."""
    buf = io.StringIO()
    writer = csv.writer(buf)
    writer.writerow(["id", "name", "email"])
    for i in range(1, num_rows + 1):
        writer.writerow([str(i), f"User {i}", f"user{i}@example.com"])
    return buf.getvalue().encode("utf-8")


def make_random_binary(size: int = 256) -> bytes:
    """Opaque binary blob (not CSV, not pickle)."""
    import os
    return os.urandom(size)


# ── Tests ───────────────────────────────────────────────────────────────

def test_health():
    resp = requests.get(f"{BASE_URL}/health", timeout=5)
    assert resp.status_code == 200
    assert resp.json()["status"] == "ok"
    print("  ✓ Health check passed")


def test_upload_csv():
    """A normal CSV file should be parsed and returned."""
    csv_data = make_valid_csv(3)
    resp = requests.post(
        f"{BASE_URL}/upload",
        files={"file": ("data.csv", csv_data, "text/csv")},
        timeout=10,
    )
    assert resp.status_code == 200, f"Expected 200, got {resp.status_code}: {resp.text}"
    body = resp.json()
    assert body["status"] == "processed"
    assert body["format"] == "csv"
    assert body["rows"] == 4  # header + 3 data rows
    assert body["data"][0] == ["id", "name", "email"]
    assert body["data"][1] == ["1", "User 1", "user1@example.com"]
    print("  ✓ Valid CSV accepted (200 OK) — 4 rows returned")


def test_upload_benign_pickle():
    """A harmless pickle should be deserialised — shows the fallback path."""
    benign = {"message": "hello from pickle", "value": 42}
    payload = pickle.dumps(benign)
    resp = requests.post(
        f"{BASE_URL}/upload",
        files={"file": ("data.pkl", payload, "application/octet-stream")},
        timeout=10,
    )
    assert resp.status_code == 200, f"Expected 200, got {resp.status_code}"
    body = resp.json()
    assert body["status"] == "processed"
    assert body["format"] == "pickle"
    assert "hello from pickle" in body["data"]
    print("  ✓ Benign pickle accepted (200 OK) — pickle fallback works")


def test_upload_malicious_pickle():
    """
    Demonstrate the vulnerability: code inside a pickle executes during
    deserialisation.  We use a pickle that calls eval("2+2") — in a real
    attack an adversary would use os.system or similar.
    """
    class Malicious:
        def __reduce__(self):
            return (eval, ("2 + 2",))

    payload = pickle.dumps(Malicious())
    resp = requests.post(
        f"{BASE_URL}/upload",
        files={"file": ("evil.pkl", payload, "application/octet-stream")},
        timeout=10,
    )
    assert resp.status_code == 200
    body = resp.json()
    # eval("2+2") returns 4 — proof that arbitrary code executed
    assert body["data"] == "4", f"Expected '4', got {body['data']}"
    print("  ⚠  Malicious pickle ACCEPTED — arbitrary code executed!")


def test_upload_random_binary_rejected():
    """Random bytes (not CSV, not pickle) → 400 Bad Request."""
    payload = make_random_binary(256)
    resp = requests.post(
        f"{BASE_URL}/upload",
        files={"file": ("random.bin", payload, "application/octet-stream")},
        timeout=10,
    )
    assert resp.status_code == 400, f"Expected 400, got {resp.status_code}"
    assert "neither valid csv" in resp.text.lower()
    print("  ✓ Random binary correctly rejected (400 Bad Request)")


# ── Main ────────────────────────────────────────────────────────────────

def main():
    print("=" * 60)
    print("Testing BEFORE (vulnerable) FastAPI app")
    print("=" * 60)

    server = start_server()
    try:
        test_health()
        test_upload_csv()
        test_upload_benign_pickle()
        test_upload_malicious_pickle()
        test_upload_random_binary_rejected()
        print("\n✅ All before tests passed (vulnerability confirmed).")
    finally:
        stop_server(server)


if __name__ == "__main__":
    main()
