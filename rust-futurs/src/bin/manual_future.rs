use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::Duration;

/// Shared state between the future and the thread that completes it.
struct Shared {
    done: bool,
    waker: Option<Waker>,
}
/// A future that resolves after `delay`, driven by a background thread.
struct Delay {
    shared: Arc<Mutex<Shared>>,
}
impl Delay {
    fn new(delay: Duration) -> Self {
        let shared = Arc::new(Mutex::new(Shared {
            done: false,
            waker: None,
        }));
        // Background thread: after the delay, mark done and wake the runtime.
        let bg = Arc::clone(&shared);
        thread::spawn(move || {
            thread::sleep(delay);
            let mut s = bg.lock().unwrap();
            s.done = true;
            if let Some(waker) = s.waker.take() {
                waker.wake(); // <-- tells the runtime: "poll me again"
            }
        });
        Delay { shared }
    }
}
impl Future for Delay {
    type Output = &'static str;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut s = self.shared.lock().unwrap();
        if s.done {
            Poll::Ready("finished") // done — hand back the value
        } else {
            s.waker = Some(cx.waker().clone()); // stash the waker...
            Poll::Pending // ...and report "not yet"
        }
    }
}
#[tokio::main]
async fn main() {
    // Even a hand-rolled future is awaited exactly like any other.
    let result = Delay::new(Duration::from_millis(500)).await;
    println!("{result}"); // finished  (after ~500ms)
}
