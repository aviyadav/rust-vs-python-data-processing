use std::time::Duration;

use db_client_multithreaded::metrics::Metrics;

static METRICS: Metrics = Metrics::new();

#[tokio::main]
async fn main() {
    // Simulate some work with metrics tracking
    METRICS.inc_tasks_spawned();
    METRICS.inc_db_queries();

    for _ in 0..5 {
        METRICS.inc_observer_ticks();
        let snap = METRICS.snapshot();
        println!(
            "[second] tick #{} | tasks: {}, db_queries: {}, http: {}",
            snap.observer_ticks, snap.tasks_spawned, snap.db_queries, snap.http_requests,
        );
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    println!("[second] done.");
}
