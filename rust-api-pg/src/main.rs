use actix_web::{get, web, App, HttpResponse, HttpServer, Result};
use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use deadpool_postgres::{Config, Client, Pool};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::env;
use std::future::{ready, Future, Ready};
use std::pin::Pin;
use std::time::Instant;
use thiserror::Error;
use tokio_postgres::types::Type;
use tokio_postgres::Row;

#[derive(Error, Debug)]
enum ApiError {
    #[error("Database error: {0}")]
    Database(#[from] tokio_postgres::Error),
    #[error("Pool error: {0}")]
    Pool(#[from] deadpool_postgres::PoolError),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Invalid identifier: {0}")]
    InvalidIdentifier(String),
    #[error("Environment variable error: {0}")]
    EnvVar(#[from] env::VarError),
}

impl actix_web::ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ApiError::InvalidIdentifier(msg) => {
                HttpResponse::BadRequest().json(serde_json::json!({"error": msg}))
            }
            _ => {
                // Log the real error server-side only; never expose internal details to the client.
                log::error!("Internal error: {}", self);
                HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error": "Internal server error"}))
            }
        }
    }
}

// ── Request Timer Middleware ──────────────────────────────────────────

pub struct RequestTimer;

impl<S, B> Transform<S, ServiceRequest> for RequestTimer
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type InitError = ();
    type Transform = RequestTimerMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestTimerMiddleware { service }))
    }
}

pub struct RequestTimerMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for RequestTimerMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start = Instant::now();
        let method = req.method().to_string();
        let path = req.uri().path().to_string();
        let query = req.uri().query().map(|q| format!("?{}", q)).unwrap_or_default();

        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;
            let duration = start.elapsed();
            let status = res.status().as_u16();

            log::info!(
                "Request: {} {}{} | Status: {} | Duration: {:?}",
                method, path, query, status, duration
            );

            Ok(res)
        })
    }
}

/// Hard cap on rows returned per request to prevent DoS / memory exhaustion.
const MAX_ROW_LIMIT: i64 = 10_000;
const DEFAULT_ROW_LIMIT: i64 = 1_000;

#[derive(Deserialize)]
struct TableQuery {
    schema: String,
    table: String,
    /// Optional row limit (capped at MAX_ROW_LIMIT). Defaults to DEFAULT_ROW_LIMIT.
    limit: Option<i64>,
}

#[derive(Serialize)]
struct TableResponse {
    schema: String,
    table: String,
    row_count: usize,
    data: Vec<Map<String, Value>>,
}

fn is_valid_identifier(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

fn row_to_json_map(row: &Row) -> Result<Map<String, Value>, ApiError> {
    let mut map = Map::new();
    let columns = row.columns();

    for col in columns {
        let name = col.name();
        let value = match col.type_() {
            &Type::BOOL => row.try_get::<_, Option<bool>>(name).map(|v| match v {
                Some(b) => Value::Bool(b),
                None => Value::Null,
            }),
            &Type::INT2 => row.try_get::<_, Option<i16>>(name).map(|v| match v {
                Some(n) => Value::Number(n.into()),
                None => Value::Null,
            }),
            &Type::INT4 => row.try_get::<_, Option<i32>>(name).map(|v| match v {
                Some(n) => Value::Number(n.into()),
                None => Value::Null,
            }),
            &Type::INT8 => row.try_get::<_, Option<i64>>(name).map(|v| match v {
                Some(n) => Value::Number(n.into()),
                None => Value::Null,
            }),
            &Type::FLOAT4 => row.try_get::<_, Option<f32>>(name).map(|v| match v {
                Some(f) => serde_json::Number::from_f64(f as f64)
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
                None => Value::Null,
            }),
            &Type::FLOAT8 => row.try_get::<_, Option<f64>>(name).map(|v| match v {
                Some(f) => serde_json::Number::from_f64(f)
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
                None => Value::Null,
            }),
            &Type::TEXT | &Type::VARCHAR | &Type::BPCHAR | &Type::NAME => {
                row.try_get::<_, Option<String>>(name).map(|v| match v {
                    Some(s) => Value::String(s),
                    None => Value::Null,
                })
            }
            &Type::JSON | &Type::JSONB => {
                row.try_get::<_, Option<Value>>(name).map(|v| v.unwrap_or(Value::Null))
            }
            &Type::TIMESTAMP | &Type::TIMESTAMPTZ => {
                row.try_get::<_, Option<chrono::NaiveDateTime>>(name)
                    .map(|v: Option<chrono::NaiveDateTime>| match v {
                        Some(dt) => Value::String(dt.to_string()),
                        None => Value::Null,
                    })
            }
            &Type::DATE => {
                row.try_get::<_, Option<chrono::NaiveDate>>(name)
                    .map(|v: Option<chrono::NaiveDate>| match v {
                        Some(d) => Value::String(d.to_string()),
                        None => Value::Null,
                    })
            }
            &Type::UUID => {
                row.try_get::<_, Option<uuid::Uuid>>(name)
                    .map(|v: Option<uuid::Uuid>| match v {
                        Some(u) => Value::String(u.to_string()),
                        None => Value::Null,
                    })
            }
            _ => {
                // Fallback: try as string
                row.try_get::<_, Option<String>>(name).map(|v| match v {
                    Some(s) => Value::String(s),
                    None => Value::Null,
                })
            }
        }
        .unwrap_or_else(|_| Value::Null);

        map.insert(name.to_string(), value);
    }

    Ok(map)
}

#[get("/api/table")]
async fn get_table(
    query: web::Query<TableQuery>,
    pool: web::Data<Pool>,
) -> Result<HttpResponse, ApiError> {
    if !is_valid_identifier(&query.schema) {
        return Err(ApiError::InvalidIdentifier(
            "Invalid schema name. Only alphanumeric characters, underscores, and hyphens are allowed."
                .to_string(),
        ));
    }
    if !is_valid_identifier(&query.table) {
        return Err(ApiError::InvalidIdentifier(
            "Invalid table name. Only alphanumeric characters, underscores, and hyphens are allowed."
                .to_string(),
        ));
    }

    let client: Client = pool.get().await?;

    // Check if table exists
    let check_sql = "SELECT 1 FROM information_schema.tables 
                     WHERE table_schema = $1 AND table_name = $2";
    let check_row = client.query_opt(check_sql, &[&query.schema, &query.table]).await?;

    if check_row.is_none() {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": format!("Table {}.{} not found", query.schema, query.table)
        })));
    }

    let row_limit = query
        .limit
        .unwrap_or(DEFAULT_ROW_LIMIT)
        .clamp(1, MAX_ROW_LIMIT);

    let sql = format!(
        "SELECT * FROM \"{}\".\"{}\" LIMIT {}",
        query.schema, query.table, row_limit
    );

    let query_start = Instant::now();
    let rows = client.query(&sql, &[]).await?;
    let query_duration = query_start.elapsed();

    let mut data = Vec::with_capacity(rows.len());
    for row in &rows {
        let json_map = row_to_json_map(row)?;
        data.push(json_map);
    }

    log::info!(
        "[{}.{}] Query completed in {:?} | Rows fetched: {}",
        query.schema,
        query.table,
        query_duration,
        data.len()
    );

    let response = TableResponse {
        schema: query.schema.clone(),
        table: query.table.clone(),
        row_count: data.len(),
        data,
    };

    Ok(HttpResponse::Ok().json(response))
}

#[get("/api/health")]
async fn health_check(pool: web::Data<Pool>) -> Result<HttpResponse, ApiError> {
    let _client: Client = pool.get().await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({ "status": "healthy" })))
}

fn create_pool() -> Result<Pool, ApiError> {
    let mut cfg = Config::new();

    cfg.host = Some(env::var("DB_HOST")?);
    cfg.port = Some(env::var("DB_PORT")?.parse().unwrap_or(5432));
    cfg.dbname = Some(env::var("DB_NAME")?);
    cfg.user = Some(env::var("DB_USER")?);
    cfg.password = Some(env::var("DB_PASSWORD")?);

    let pool = cfg
        .create_pool(None, tokio_postgres::NoTls)
        .map_err(|e| ApiError::InvalidIdentifier(format!("Failed to create pool: {}", e)))?;

    Ok(pool)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let pool = match create_pool() {
        Ok(p) => {
            log::info!("Database pool created successfully");
            p
        }
        Err(e) => {
            log::error!("Failed to create database pool: {}", e);
            std::process::exit(1);
        }
    };

    let bind_address = env::var("SERVER_BIND")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string());

    log::warn!("Database connection is using NoTls — enable TLS for production deployments.");
    log::info!("Starting server at http://{}", bind_address);

    HttpServer::new(move || {
        App::new()
            .wrap(RequestTimer)
            .app_data(web::Data::new(pool.clone()))
            .service(get_table)
            .service(health_check)
    })
    .bind(&bind_address)?
    .run()
    .await
}
