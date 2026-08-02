use warp::Filter;
use tokio::time::{sleep, Duration};
use serde::Serialize;
use utoipa::{OpenApi, ToSchema};

// ---------------------------------------------------------------------------
// Embedded Swagger UI page (loads swagger-ui-dist from CDN)
// ---------------------------------------------------------------------------
const SWAGGER_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>High-Traffic Service – Swagger UI</title>
  <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui.css">
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui-bundle.js" crossorigin></script>
  <script>
    SwaggerUIBundle({
      url: "/api-docs/openapi.json",
      dom_id: "#swagger-ui",
      presets: [SwaggerUIBundle.presets.apis, SwaggerUIBundle.SwaggerUIStandalonePreset],
      layout: "BaseLayout",
    });
  </script>
</body>
</html>"##;

// ---------------------------------------------------------------------------
// OpenAPI schema
// ---------------------------------------------------------------------------
#[derive(OpenApi)]
#[openapi(
    paths(greet),
    components(schemas(GreetResponse))
)]
struct ApiDoc;

// ---------------------------------------------------------------------------
// Response type
// ---------------------------------------------------------------------------
#[derive(Serialize, ToSchema)]
struct GreetResponse {
    /// The greeting message returned to the caller.
    message: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------
#[utoipa::path(
    get,
    path = "/greet/{name}",
    params(
        ("name" = String, Path, description = "Name to greet")
    ),
    responses(
        (status = 200, description = "Successful greeting", body = GreetResponse)
    )
)]
async fn greet(name: String) -> Result<impl warp::Reply, warp::Rejection> {
    // Simulate async I/O (e.g. talking to a database)
    sleep(Duration::from_millis(50)).await;

    let response = GreetResponse {
        message: format!("Hello, {}! From Rust service.", name),
    };
    Ok(warp::reply::json(&response))
}

// ---------------------------------------------------------------------------
// Entrypoint
// ---------------------------------------------------------------------------
#[tokio::main]
async fn main() {
    let openapi = ApiDoc::openapi();

    // Greet endpoint
    let greet_route = warp::path!("greet" / String)
        .and(warp::get())
        .and_then(greet)
        .boxed();

    // Serve the raw OpenAPI JSON
    let openapi_json_route = {
        let spec = openapi.clone();
        warp::path!("api-docs" / "openapi.json")
            .map(move || warp::reply::json(&spec))
            .boxed()
    };

    // Serve the Swagger UI HTML page
    let swagger_ui_route = warp::path!("swagger-ui")
        .map(|| warp::reply::html(SWAGGER_HTML))
        .boxed();

    // Compose all routes
    let routes = greet_route
        .or(openapi_json_route)
        .or(swagger_ui_route);

    println!("🚀 Rust service starting on http://127.0.0.1:8080");
    println!("📖 Swagger UI:   http://127.0.0.1:8080/swagger-ui");
    println!("📋 OpenAPI JSON: http://127.0.0.1:8080/api-docs/openapi.json");

    warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
}
