use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, OpenApi, ToSchema};

use crate::db;

// ─── API State ────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub pool: deadpool_postgres::Pool,
}

// ─── Response Types ───────────────────────────────────────────────────────────

#[derive(Serialize, ToSchema)]
pub struct ListResponse {
    pub data: Vec<serde_json::Value>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Serialize, ToSchema)]
pub struct AffectedResponse {
    pub affected_rows: u64,
}

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

// ─── Query Parameters ─────────────────────────────────────────────────────────

#[derive(Deserialize, IntoParams)]
pub struct ListQuery {
    #[param(default = 1)]
    pub page: Option<u32>,
    #[param(default = 20)]
    pub page_size: Option<u32>,
    pub study: Option<String>,
    pub site: Option<String>,
    pub subject: Option<String>,
    pub visit: Option<String>,
    pub domain: Option<String>,
    pub studyid: Option<String>,
    pub usubjid: Option<String>,
    pub siteid: Option<String>,
}

impl ListQuery {
    fn to_filters(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        if let Some(ref v) = self.study {
            map.insert("STUDY".to_string(), v.clone());
        }
        if let Some(ref v) = self.site {
            map.insert("SITE".to_string(), v.clone());
        }
        if let Some(ref v) = self.subject {
            map.insert("SUBJECT".to_string(), v.clone());
        }
        if let Some(ref v) = self.visit {
            map.insert("VISIT".to_string(), v.clone());
        }
        if let Some(ref v) = self.domain {
            map.insert("DOMAIN".to_string(), v.clone());
        }
        if let Some(ref v) = self.studyid {
            map.insert("STUDYID".to_string(), v.clone());
        }
        if let Some(ref v) = self.usubjid {
            map.insert("USUBJID".to_string(), v.clone());
        }
        if let Some(ref v) = self.siteid {
            map.insert("SITEID".to_string(), v.clone());
        }
        map
    }
}

// ─── Generic handler helpers ──────────────────────────────────────────────────

async fn handle_list(table: &str, state: &AppState, query: ListQuery) -> Response {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);
    match db::list_records(&state.pool, table, &query.to_filters(), page, page_size).await {
        Ok((data, total)) => Json(ListResponse {
            data,
            total,
            page,
            page_size,
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

async fn handle_create(table: &str, state: &AppState, body: serde_json::Value) -> Response {
    match db::insert_json(&state.pool, table, &body).await {
        Ok(n) => (
            StatusCode::CREATED,
            Json(AffectedResponse { affected_rows: n }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

async fn handle_update(
    table: &str,
    keys: &[&str],
    state: &AppState,
    body: serde_json::Value,
) -> Response {
    match db::update_json(&state.pool, table, keys, &body).await {
        Ok(n) => (StatusCode::OK, Json(AffectedResponse { affected_rows: n })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

async fn handle_delete(
    table: &str,
    keys: &[&str],
    state: &AppState,
    body: serde_json::Value,
) -> Response {
    match db::delete_by_key_json(&state.pool, table, keys, &body).await {
        Ok(n) => (StatusCode::OK, Json(AffectedResponse { affected_rows: n })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

// ─── Health Check ─────────────────────────────────────────────────────────────

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/api/health",
    responses(
        (status = 200, description = "Database connection OK"),
        (status = 503, description = "Database connection failed")
    )
)]
pub async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    match db::check_health(&state.pool).await {
        Ok(()) => (StatusCode::OK, "OK".to_string()),
        Err(e) => (StatusCode::SERVICE_UNAVAILABLE, format!("ERROR: {}", e)),
    }
}

// ─── AE Handlers ──────────────────────────────────────────────────────────────

#[utoipa::path(get, path = "/api/ae", params(ListQuery),
    responses((status = 200, description = "List AE records", body = ListResponse)))]
pub async fn list_ae(State(state): State<AppState>, Query(q): Query<ListQuery>) -> Response {
    handle_list("ae", &state, q).await
}

#[utoipa::path(post, path = "/api/ae", request_body = serde_json::Value,
    responses((status = 201, description = "Record created", body = AffectedResponse)))]
pub async fn create_ae(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    handle_create("ae", &state, body).await
}

#[utoipa::path(put, path = "/api/ae", request_body = serde_json::Value,
    responses((status = 200, description = "Records updated", body = AffectedResponse)))]
pub async fn update_ae(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    handle_update("ae", db::AeRecord::KEY_COLUMNS, &state, body).await
}

#[utoipa::path(delete, path = "/api/ae", request_body = serde_json::Value,
    responses((status = 200, description = "Records deleted", body = AffectedResponse)))]
pub async fn delete_ae(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    handle_delete("ae", db::AeRecord::KEY_COLUMNS, &state, body).await
}

// ─── CM Handlers ──────────────────────────────────────────────────────────────

#[utoipa::path(get, path = "/api/cm", params(ListQuery),
    responses((status = 200, description = "List CM records", body = ListResponse)))]
pub async fn list_cm(State(state): State<AppState>, Query(q): Query<ListQuery>) -> Response {
    handle_list("cm", &state, q).await
}

#[utoipa::path(post, path = "/api/cm", request_body = serde_json::Value,
    responses((status = 201, description = "Record created", body = AffectedResponse)))]
pub async fn create_cm(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    handle_create("cm", &state, body).await
}

#[utoipa::path(put, path = "/api/cm", request_body = serde_json::Value,
    responses((status = 200, description = "Records updated", body = AffectedResponse)))]
pub async fn update_cm(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    handle_update("cm", db::CmRecord::KEY_COLUMNS, &state, body).await
}

#[utoipa::path(delete, path = "/api/cm", request_body = serde_json::Value,
    responses((status = 200, description = "Records deleted", body = AffectedResponse)))]
pub async fn delete_cm(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    handle_delete("cm", db::CmRecord::KEY_COLUMNS, &state, body).await
}

// ─── DM Handlers ──────────────────────────────────────────────────────────────

#[utoipa::path(get, path = "/api/dm", params(ListQuery),
    responses((status = 200, description = "List DM records", body = ListResponse)))]
pub async fn list_dm(State(state): State<AppState>, Query(q): Query<ListQuery>) -> Response {
    handle_list("dm", &state, q).await
}

#[utoipa::path(post, path = "/api/dm", request_body = serde_json::Value,
    responses((status = 201, description = "Record created", body = AffectedResponse)))]
pub async fn create_dm(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    handle_create("dm", &state, body).await
}

#[utoipa::path(put, path = "/api/dm", request_body = serde_json::Value,
    responses((status = 200, description = "Records updated", body = AffectedResponse)))]
pub async fn update_dm(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    handle_update("dm", db::DmRecord::KEY_COLUMNS, &state, body).await
}

#[utoipa::path(delete, path = "/api/dm", request_body = serde_json::Value,
    responses((status = 200, description = "Records deleted", body = AffectedResponse)))]
pub async fn delete_dm(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    handle_delete("dm", db::DmRecord::KEY_COLUMNS, &state, body).await
}

// ─── LB Handlers ──────────────────────────────────────────────────────────────

#[utoipa::path(get, path = "/api/lb", params(ListQuery),
    responses((status = 200, description = "List LB records", body = ListResponse)))]
pub async fn list_lb(State(state): State<AppState>, Query(q): Query<ListQuery>) -> Response {
    handle_list("lb", &state, q).await
}

#[utoipa::path(post, path = "/api/lb", request_body = serde_json::Value,
    responses((status = 201, description = "Record created", body = AffectedResponse)))]
pub async fn create_lb(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    handle_create("lb", &state, body).await
}

#[utoipa::path(put, path = "/api/lb", request_body = serde_json::Value,
    responses((status = 200, description = "Records updated", body = AffectedResponse)))]
pub async fn update_lb(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    handle_update("lb", db::LbRecord::KEY_COLUMNS, &state, body).await
}

#[utoipa::path(delete, path = "/api/lb", request_body = serde_json::Value,
    responses((status = 200, description = "Records deleted", body = AffectedResponse)))]
pub async fn delete_lb(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    handle_delete("lb", db::LbRecord::KEY_COLUMNS, &state, body).await
}

// ─── TV Handlers ──────────────────────────────────────────────────────────────

#[utoipa::path(get, path = "/api/tv", params(ListQuery),
    responses((status = 200, description = "List TV records", body = ListResponse)))]
pub async fn list_tv(State(state): State<AppState>, Query(q): Query<ListQuery>) -> Response {
    handle_list("tv", &state, q).await
}

#[utoipa::path(post, path = "/api/tv", request_body = serde_json::Value,
    responses((status = 201, description = "Record created", body = AffectedResponse)))]
pub async fn create_tv(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    handle_create("tv", &state, body).await
}

#[utoipa::path(put, path = "/api/tv", request_body = serde_json::Value,
    responses((status = 200, description = "Records updated", body = AffectedResponse)))]
pub async fn update_tv(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    handle_update("tv", db::TvRecord::KEY_COLUMNS, &state, body).await
}

#[utoipa::path(delete, path = "/api/tv", request_body = serde_json::Value,
    responses((status = 200, description = "Records deleted", body = AffectedResponse)))]
pub async fn delete_tv(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    handle_delete("tv", db::TvRecord::KEY_COLUMNS, &state, body).await
}

// ─── VS Handlers ──────────────────────────────────────────────────────────────

#[utoipa::path(get, path = "/api/vs", params(ListQuery),
    responses((status = 200, description = "List VS records", body = ListResponse)))]
pub async fn list_vs(State(state): State<AppState>, Query(q): Query<ListQuery>) -> Response {
    handle_list("vs", &state, q).await
}

#[utoipa::path(post, path = "/api/vs", request_body = serde_json::Value,
    responses((status = 201, description = "Record created", body = AffectedResponse)))]
pub async fn create_vs(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    handle_create("vs", &state, body).await
}

#[utoipa::path(put, path = "/api/vs", request_body = serde_json::Value,
    responses((status = 200, description = "Records updated", body = AffectedResponse)))]
pub async fn update_vs(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    handle_update("vs", db::VsRecord::KEY_COLUMNS, &state, body).await
}

#[utoipa::path(delete, path = "/api/vs", request_body = serde_json::Value,
    responses((status = 200, description = "Records deleted", body = AffectedResponse)))]
pub async fn delete_vs(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    handle_delete("vs", db::VsRecord::KEY_COLUMNS, &state, body).await
}

// ─── OpenAPI Swagger UI ───────────────────────────────────────────────────────

const SWAGGER_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Clinical Trial CRUD API - Swagger UI</title>
    <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js" crossorigin></script>
    <script>
        SwaggerUIBundle({
            url: "/api-docs/openapi.json",
            dom_id: '#swagger-ui',
            presets: [SwaggerUIBundle.presets.apis, SwaggerUIBundle.SwaggerUIStandalonePreset],
            layout: "BaseLayout",
        });
    </script>
</body>
</html>"#;

async fn swagger_ui() -> Html<&'static str> {
    Html(SWAGGER_HTML)
}

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

// ─── OpenAPI Documentation ────────────────────────────────────────────────────

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Clinical Trial CRUD API",
        version = "1.0.0",
        description = "REST API for CRUD operations on clinical trial PostgreSQL tables (AE, CM, DM, LB, TV, VS)"
    ),
    paths(
        health_check,
        list_ae, create_ae, update_ae, delete_ae,
        list_cm, create_cm, update_cm, delete_cm,
        list_dm, create_dm, update_dm, delete_dm,
        list_lb, create_lb, update_lb, delete_lb,
        list_tv, create_tv, update_tv, delete_tv,
        list_vs, create_vs, update_vs, delete_vs,
    ),
    components(
        schemas(ListResponse, AffectedResponse, ErrorResponse)
    ),
    tags(
        (name = "AE", description = "Adverse Events"),
        (name = "CM", description = "Concomitant Medications"),
        (name = "DM", description = "Demographics"),
        (name = "LB", description = "Laboratory"),
        (name = "TV", description = "Trial Visits"),
        (name = "VS", description = "Vital Signs"),
    )
)]
pub struct ApiDoc;

// ─── Router ───────────────────────────────────────────────────────────────────

pub fn build_router(state: AppState) -> axum::Router {
    axum::Router::new()
        .route("/swagger-ui", axum::routing::get(swagger_ui))
        .route("/api-docs/openapi.json", axum::routing::get(openapi_json))
        .route("/api/health", axum::routing::get(health_check))
        .route(
            "/api/ae",
            axum::routing::get(list_ae)
                .post(create_ae)
                .put(update_ae)
                .delete(delete_ae),
        )
        .route(
            "/api/cm",
            axum::routing::get(list_cm)
                .post(create_cm)
                .put(update_cm)
                .delete(delete_cm),
        )
        .route(
            "/api/dm",
            axum::routing::get(list_dm)
                .post(create_dm)
                .put(update_dm)
                .delete(delete_dm),
        )
        .route(
            "/api/lb",
            axum::routing::get(list_lb)
                .post(create_lb)
                .put(update_lb)
                .delete(delete_lb),
        )
        .route(
            "/api/tv",
            axum::routing::get(list_tv)
                .post(create_tv)
                .put(update_tv)
                .delete(delete_tv),
        )
        .route(
            "/api/vs",
            axum::routing::get(list_vs)
                .post(create_vs)
                .put(update_vs)
                .delete(delete_vs),
        )
        .with_state(state)
}
