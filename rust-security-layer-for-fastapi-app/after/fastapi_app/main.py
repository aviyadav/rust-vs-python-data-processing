"""
After: Secure FastAPI Application
==================================
FastAPI delegates all file validation to a co-located Rust security layer.
The Python process never touches raw bytes from untrusted uploads; the Rust
validator confirms the payload is well-formed CSV within size limits first.

Architecture
------------
  Client  →  FastAPI (:8000)  →  Rust validator (:3000)  →  FastAPI →  Client
"""

import httpx
import uvicorn
from fastapi import FastAPI, HTTPException, UploadFile
from fastapi.responses import JSONResponse

app = FastAPI(title="After — Rust-validated upload")

RUST_VALIDATOR_URL = "http://127.0.0.1:3000/validate"
RUST_HEALTH_URL = "http://127.0.0.1:3000/health"


@app.post("/upload")
async def upload(file: UploadFile):
    """
    Forward the uploaded file to the Rust validator.  The FastAPI process
    never calls file.read(), pickle, eval, or any dangerous deserialiser.
    """

    # ── Build the httpx file tuple, guarding against None values ────
    filename = file.filename or "uploaded_file"
    content_type = file.content_type or "application/octet-stream"
    file_obj = file.file  # SpooledTemporaryFile / TemporaryFile — read from 0

    try:
        async with httpx.AsyncClient(timeout=30.0) as client:
            response = await client.post(
                RUST_VALIDATOR_URL,
                files={"file": (filename, file_obj, content_type)},
            )
    except httpx.ConnectError:
        raise HTTPException(
            status_code=502,
            detail="Rust validator is not running on port 3000. Start it first.",
        )
    except httpx.TimeoutException:
        raise HTTPException(
            status_code=504,
            detail="Rust validator timed out.",
        )

    if response.status_code == 413:
        raise HTTPException(status_code=413, detail="File too large (max 10 MiB).")
    if response.status_code != 200:
        detail = response.text[:300] or "(empty body from validator)"
        raise HTTPException(
            status_code=400,
            detail=f"File rejected by security layer: {detail}",
        )

    # Rust already parsed the CSV — return the validated records directly.
    return response.json()


@app.get("/health")
async def health():
    """Health-check; also verifies the Rust validator is reachable."""
    try:
        async with httpx.AsyncClient(timeout=2.0) as client:
            resp = await client.get(RUST_HEALTH_URL)
        rust_status = "reachable" if resp.status_code == 200 else "unhealthy"
    except Exception:
        rust_status = "unreachable"

    return {"status": "ok", "rust_validator": rust_status}


if __name__ == "__main__":
    uvicorn.run(app, host="127.0.0.1", port=8000)
