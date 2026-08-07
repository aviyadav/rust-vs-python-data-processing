use sqlx::PgPool;
use std::sync::Mutex;
use tokio::time::Duration;

const DB_URL: &str = "postgres://demouser:password@localhost:5432/demodb";

static METRICS_LOCK: Mutex<()> = Mutex::new(());

/* Tokio runtime introspection - use the `tracing` crate for more in-depth info */
fn show_metrics(msg: &str) {
    // serialise output so that we can call this from multiple threads at once
    let _lock = METRICS_LOCK.lock().unwrap();

    let handle = tokio::runtime::Handle::current();
    let metrics = handle.metrics();
    let thread_id = std::thread::current().id();
    println!(
        ">[{}]> Tokio threads: {} workers, {} tasks alive, queue depth {}, this thread is {:?} running on {:?} as {}",
        msg,
        metrics.num_workers(),
        metrics.num_alive_tasks(),
        metrics.global_queue_depth(),
        thread_id,
        handle.runtime_flavor(),
        handle.name().unwrap_or("??"),
    );

    for i in 0..metrics.num_workers() {
        println!(
            "{} - parked {} times, total busy {:?}",
            i,
            metrics.worker_park_count(i),
            metrics.worker_total_busy_duration(i)
        );
    }
    println!("<<<");
}

async fn db_then_compute(pool: PgPool) {
    show_metrics("before DB query");
    let num: (i32,) = sqlx::query_as("SELECT 1").fetch_one(&pool).await.unwrap();
    show_metrics("after DB query");

    println!(
        "The num was returned: {}, but now, the CPU block will kill multithreading",
        num.0
    );
    println!("Start compute...");
    // This would "fix" the issue:
    // pool.close().await;

    // This has been extended to limit the busy-wait so we can see what would happen if it _did_ finish
    let start = std::time::Instant::now();
    loop {
        let elapsed = std::time::Instant::now() - start;
        if elapsed > std::time::Duration::from_secs(3) {
            println!("3s elapsed, bailing");
            break;
        }
    }
    println!("done");
    show_metrics("finished db_the_compute");
}

/// This doesn't cause the issue
async fn http_then_compute() {
    show_metrics("before HTTP reqwest");
    let res = reqwest::get("https://google.de").await.unwrap();
    show_metrics("after HTTP reqwest");
    let status = res.status();
    println!(
        "The num was returned: {}, but the CPU block will not kill multithreading",
        status.as_u16()
    );
    loop {}
}

fn main() {
    use tokio::runtime;

    let rt = runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        show_metrics("main starts");
        let pool = sqlx::PgPool::connect(DB_URL).await.unwrap();

        show_metrics("connect complete, spawning DB task");
        tokio::spawn(db_then_compute(pool));
        show_metrics("DB task complete");
        //tokio::spawn(http_then_compute());
        loop {
            println!("Observer thread, the sleep will never return");
            show_metrics("sleep loop");
            let t = tokio::time::sleep(Duration::from_secs_f32(0.25));
            t.await;
        }
    });
}
