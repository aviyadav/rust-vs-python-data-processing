//! Rust Security Validation Layer
//!
//! A lightweight Axum service that receives multipart file uploads,
//! enforces size limits, and validates content as strict CSV before
//! returning the parsed records back to the FastAPI gateway.
//!
//! Port: 3000  |  Endpoints: GET /health, POST /validate

use axum::{extract::Multipart, http::StatusCode, response::Json, routing::{get, post}, Router};
use serde::Serialize;
use std::net::SocketAddr;

// ── Response payload ────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ValidationResponse {
    status: String,
    records: Vec<Vec<String>>,
    row_count: usize,
}

// ── Health check ────────────────────────────────────────────────────────────

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

// ── Validation handler ──────────────────────────────────────────────────────

/// Accept a multipart upload, enforce a 10 MiB size cap, parse as strict CSV,
/// and return every record so the caller never needs to re-parse untrusted bytes.
async fn validate_upload(mut multipart: Multipart) -> Result<Json<ValidationResponse>, StatusCode> {
    // Iterate over multipart fields (we only care about the first file).
    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        // Read the entire field body into memory.
        let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;

        // ── Hard size cap: 10 MiB ──────────────────────────────────────
        if data.len() > 10_000_000 {
            return Err(StatusCode::PAYLOAD_TOO_LARGE);
        }

        // ── Strict CSV parsing ─────────────────────────────────────────
        // The csv crate rejects malformed rows by default (no flexible mode).
        let mut reader = csv::ReaderBuilder::new();
        reader.buffer_capacity(1024 * 8);
        let mut csv_reader = reader.from_reader(data.as_ref());

        let mut records: Vec<Vec<String>> = Vec::new();
        for result in csv_reader.records() {
            let record = result.map_err(|_| StatusCode::BAD_REQUEST)?;
            let fields: Vec<String> = record.iter().map(|f| f.to_string()).collect();
            records.push(fields);
        }

        return Ok(Json(ValidationResponse {
            status: "clean".to_string(),
            row_count: records.len(),
            records,
        }));
    }

    Err(StatusCode::BAD_REQUEST)
}

// ── Server entrypoint ───────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/health", get(health))
        .route("/validate", post(validate_upload));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("🛡  Rust validator listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
