use reqwest::Client;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

const TOTAL_CALLS: u32 = 100;
const PARALLEL_WORKERS: u32 = 20;
const TARGET_URL: &str = "http://127.0.0.1:8080/greet";

#[tokio::main]
async fn main() {
    let client = Client::new();
    let counter = Arc::new(Mutex::new(0u32));

    println!(
        "⚡ Load test: {} parallel workers → {} total calls → {}\n",
        PARALLEL_WORKERS, TOTAL_CALLS, TARGET_URL
    );

    let start = Instant::now();
    let mut handles = Vec::with_capacity(PARALLEL_WORKERS as usize);

    for worker_id in 0..PARALLEL_WORKERS {
        let client = client.clone();
        let counter = Arc::clone(&counter);

        handles.push(tokio::spawn(async move {
            let name = format!("User{}", worker_id);
            let mut my_calls: u32 = 0;

            loop {
                // Atomically claim the next call number, or exit if limit reached
                let call_number;
                {
                    let mut count = counter.lock().await;
                    if *count >= TOTAL_CALLS {
                        break;
                    }
                    *count += 1;
                    call_number = *count;
                }

                my_calls += 1;
                let req_start = Instant::now();

                match client
                    .get(format!("{}/{}", TARGET_URL, name))
                    .send()
                    .await
                {
                    Ok(resp) => {
                        let elapsed_ms = req_start.elapsed().as_millis();
                        let status = resp.status();
                        let body = resp.text().await.unwrap_or_default();
                        println!(
                            "#{:03} [{}ms] HTTP {} — {}",
                            call_number, elapsed_ms, status, body
                        );
                    }
                    Err(e) => {
                        println!("#{:03} ERROR — {}", call_number, e);
                    }
                }
            }

            my_calls
        }));
    }

    // Wait for all workers and tally calls
    let mut total_calls_made: u32 = 0;
    for handle in handles {
        total_calls_made += handle.await.expect("worker panicked");
    }

    let elapsed = start.elapsed();

    println!("\n========================================");
    println!("  Load test complete!");
    println!("  Workers:      {}", PARALLEL_WORKERS);
    println!("  Total calls:  {}", total_calls_made);
    println!("  Total time:   {:.2} s", elapsed.as_secs_f64());
    println!(
        "  Throughput:   {:.2} calls/s",
        total_calls_made as f64 / elapsed.as_secs_f64()
    );
    println!("========================================");
}
