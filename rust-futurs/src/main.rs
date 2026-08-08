use std::time::{Duration, Instant};
use tokio::time::sleep;

/// A simple async fn: it *returns* a future without doing any work yet.
async fn greet(name: &str) -> String {
    format!("hello, {name}")
}

/// Simulates slow I/O (e.g., a network request).
async fn fetch_user(id: u32) -> String {
    println!("  [user {id}] started");
    sleep(Duration::from_millis(500)).await; // yields to the runtime, no blocking!
    println!("  [user {id}] done");
    format!("User#{id}")
}

#[tokio::main]
async fn main() {
    // ── 1. Futures are LAZY ────────────────────────────────────────────────
    // Calling an async fn only *builds* the future; its body hasn't run yet.
    let future = greet("Raunak");
    println!("1. Future created — but `greet` hasn't run yet.");

    // Awaiting drives the future to completion and yields its Output value.
    let msg = future.await;
    println!("   {msg}"); // hello, Raunak

    // ── 2. SEQUENTIAL awaiting ─────────────────────────────────────────────
    println!("\n2. Sequential: two fetches, one after another (takes ~1s)");
    let start = Instant::now();
    let a = fetch_user(1).await;
    let b = fetch_user(2).await;
    println!("   got [{a}, {b}] in {:?}", start.elapsed());

    // ── 3. CONCURRENT futures with join! ───────────────────────────────────
    // join! polls both futures on this task, so their sleeps overlap.
    println!("\n3. Concurrent with join!: same two fetches overlap (takes ~0.5s)");
    let start = Instant::now();
    let (a, b) = tokio::join!(fetch_user(3), fetch_user(4));
    println!("   got [{a}, {b}] in {:?}", start.elapsed());

    // ── 4. SPAWNING tasks on the runtime ───────────────────────────────────
    // spawn hands the future to the runtime so it runs on its own task.
    println!("\n4. Spawned task runs in the background on the Tokio runtime");
    let handle = tokio::spawn(async {
        sleep(Duration::from_millis(200)).await;
        "background task finished".to_string()
    });

    // We can do other work while it runs...
    sleep(Duration::from_millis(50)).await;
    println!("   main task doing other work meanwhile...");

    // ...and collect the result later. The JoinHandle is itself a future.
    let result = handle.await.expect("task panicked");
    println!("   {result}");

    // ── 5. RACING futures with select! ─────────────────────────────────────
    println!("\n5. select! races two futures and takes whichever finishes first");
    let fast = async {
        sleep(Duration::from_millis(100)).await;
        "fast won"
    };
    let slow = async {
        sleep(Duration::from_millis(900)).await;
        "slow won"
    };
    tokio::select! {
        winner = fast => println!("   {winner}"),
        winner = slow => println!("   {winner}"),
    }

    println!("\nDone — every `.await` above was a point where the task could");
    println!("yield, letting the single runtime thread interleave all the work.");
}
