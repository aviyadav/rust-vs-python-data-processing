use std::time::Duration;

use db_client_multithreaded::metrics::Metrics;
use sqlx::PgPool;

const DB_URL: &str = "postgres://demouser:password@localhost:5432/demodb";

static METRICS: Metrics = Metrics::new();

async fn db_then_compute(pool: PgPool) {
    METRICS.inc_tasks_spawned();

    let num: (i32,) = sqlx::query_as("SELECT 1").fetch_one(&pool).await.unwrap();
    METRICS.inc_db_queries();
    println!(
        "The num was returned: {}, but now, the CPU block will kill multithreading",
        num.0
    );
    println!("Start compute...");

    pool.close().await;
    loop {}
}

async fn http_then_compute() {
    METRICS.inc_tasks_spawned();

    let res = reqwest::get("https://google.com").await.unwrap();
    METRICS.inc_http_requests();
    let status = res.status();
    println!(
        "The num was returned: {}, but the CPU block will not kill multithreading",
        status.as_u16()
    );
    loop {}
}

#[tokio::main]
async fn main() {
    let pool = sqlx::PgPool::connect(DB_URL).await.unwrap();

    tokio::spawn(db_then_compute(pool));
    tokio::spawn(http_then_compute());

    loop {
        METRICS.inc_observer_ticks();
        let snap = METRICS.snapshot();
        println!(
            "Observer tick #{:03} | tasks: {}, db_queries: {}, http: {}",
            snap.observer_ticks, snap.tasks_spawned, snap.db_queries, snap.http_requests,
        );
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
