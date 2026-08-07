use std::sync::atomic::{AtomicU64, Ordering};

/// Thread-safe metrics counters for the multithreaded DB client demo.
pub struct Metrics {
    db_queries: AtomicU64,
    http_requests: AtomicU64,
    tasks_spawned: AtomicU64,
    observer_ticks: AtomicU64,
}

/// A point-in-time snapshot of all metrics.
#[derive(Debug, Clone, Copy)]
pub struct Snapshot {
    pub db_queries: u64,
    pub http_requests: u64,
    pub tasks_spawned: u64,
    pub observer_ticks: u64,
}

impl Metrics {
    pub const fn new() -> Self {
        Self {
            db_queries: AtomicU64::new(0),
            http_requests: AtomicU64::new(0),
            tasks_spawned: AtomicU64::new(0),
            observer_ticks: AtomicU64::new(0),
        }
    }

    pub fn inc_db_queries(&self) {
        self.db_queries.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_http_requests(&self) {
        self.http_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_tasks_spawned(&self) {
        self.tasks_spawned.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_observer_ticks(&self) {
        self.observer_ticks.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            db_queries: self.db_queries.load(Ordering::Relaxed),
            http_requests: self.http_requests.load(Ordering::Relaxed),
            tasks_spawned: self.tasks_spawned.load(Ordering::Relaxed),
            observer_ticks: self.observer_ticks.load(Ordering::Relaxed),
        }
    }
}
