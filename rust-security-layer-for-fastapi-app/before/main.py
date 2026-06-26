"""
Before: Vulnerable FastAPI Application
======================================
This is the "before" version — FastAPI handles file uploads directly with
no security layer.  The endpoint accepts CSV (parsed safely) but also
falls back to `pickle.loads()` for unrecognised formats, which is
trivially exploitable for remote code execution.

The vulnerability: an attacker can upload a crafted pickle file and
execute arbitrary Python code on the server.

DO NOT expose this app on a network you do not control.
"""

import csv
import io
import pickle

import uvicorn
from fastapi import FastAPI, HTTPException, UploadFile

app = FastAPI(title="Before — insecure upload")


@app.post("/upload")
async def upload(file: UploadFile):
    """
    Accept an uploaded file.

    - If it looks like valid CSV → parse and return the records.
    - Otherwise → attempt pickle deserialisation (DANGEROUS).

    The pickle fallback is the vulnerability: any attacker who sends a
    malicious pickle payload gets arbitrary code execution inside the
    Python process.
    """
    content = await file.read()

    # ── Try CSV first (the "normal" path) ─────────────────────────
    try:
        text = content.decode("utf-8")
    except UnicodeDecodeError:
        text = None

    if text is not None:
        try:
            csv_reader = csv.reader(io.StringIO(text))
            records = [row for row in csv_reader]
            if records:
                return {
                    "status": "processed",
                    "format": "csv",
                    "rows": len(records),
                    "data": records,
                }
        except csv.Error:
            pass  # Not valid CSV — fall through to pickle

    # ── Fallback: pickle deserialisation (💀 VULNERABLE) ──────────
    # Any unrecognised file is blindly unpickled.  A malicious pickle
    # can call os.system, open reverse shells, exfiltrate data, etc.
    try:
        data = pickle.loads(content)  # 💀 Arbitrary code execution
    except (pickle.UnpicklingError, EOFError, ValueError) as exc:
        raise HTTPException(
            status_code=400,
            detail=f"File is neither valid CSV nor a recognised pickle: {exc}",
        )

    return {
        "status": "processed",
        "format": "pickle",
        "data": str(data),
    }


@app.get("/health")
async def health():
    return {"status": "ok"}


if __name__ == "__main__":
    uvicorn.run(app, host="127.0.0.1", port=8001)
