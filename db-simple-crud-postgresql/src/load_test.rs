use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use tracing::{info, warn};

use crate::db;

// ─── Load Test Configuration ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LoadTestConfig {
    pub num_clients: usize,
    pub num_operations: usize,
    pub read_ratio: f64,   // 0.0 to 1.0
    pub write_ratio: f64,  // 0.0 to 1.0
    pub update_ratio: f64, // 0.0 to 1.0
    pub delete_ratio: f64, // 0.0 to 1.0
    pub table: String,
}

impl Default for LoadTestConfig {
    fn default() -> Self {
        LoadTestConfig {
            num_clients: 50,
            num_operations: 1000,
            read_ratio: 0.6,
            write_ratio: 0.2,
            update_ratio: 0.15,
            delete_ratio: 0.05,
            table: "dm".to_string(),
        }
    }
}

// ─── Stats ────────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct OperationStats {
    total: u64,
    successes: u64,
    failures: u64,
    total_duration: Duration,
    min_duration: Duration,
    max_duration: Duration,
}

impl OperationStats {
    fn record(&mut self, duration: Duration, success: bool) {
        self.total += 1;
        if success {
            self.successes += 1;
        } else {
            self.failures += 1;
        }
        self.total_duration += duration;
        if self.min_duration > duration || self.min_duration.is_zero() {
            self.min_duration = duration;
        }
        if self.max_duration < duration {
            self.max_duration = duration;
        }
    }

    fn avg_duration(&self) -> Duration {
        if self.total == 0 {
            Duration::ZERO
        } else {
            self.total_duration / self.total as u32
        }
    }
}

// ─── CRUD Operation Helpers ───────────────────────────────────────────────────

fn random_record(table: &str) -> serde_json::Value {
    let mut rng = rand::rng();
    let study_id = format!("STUDY{:04}", rng.random_range(1..100));
    let usubjid = format!("USUBJ{:06}", rng.random_range(1..10000));
    let site = format!("SITE{:03}", rng.random_range(1..50));
    let subject = format!("SUBJ{:04}", rng.random_range(1..1000));

    match table {
        "ae" => serde_json::json!({
            "study": study_id,
            "site": site,
            "subject": subject,
            "visit": "VISIT1",
            "form": "AE",
            "domain": "AE",
            "aeseq": rng.random_range(1..100),
            "aeterm": "Headache",
            "aedecod": "Headache",
            "aebodsys": "Nervous system disorders",
            "aestdtc": "2024-01-15",
            "aesev": "MILD",
            "aerel": "POSSIBLE",
            "siteid": site,
            "studyid": study_id,
            "usubjid": usubjid,
        }),
        "cm" => serde_json::json!({
            "study": study_id,
            "site": site,
            "subject": subject,
            "visit": "VISIT1",
            "form": "CM",
            "domain": "CM",
            "cmseq": rng.random_range(1..100),
            "cmtrt": "Aspirin",
            "cmdecod": "Acetylsalicylic acid",
            "cmcat": "ANALGESIC",
            "cmstdtc": "2024-01-10",
            "cmdose": 100.0,
            "cmdosu": "mg",
            "cmroute": "ORAL",
            "siteid": site,
            "studyid": study_id,
            "usubjid": usubjid,
        }),
        "dm" => serde_json::json!({
            "study": study_id,
            "site": site,
            "subject": subject,
            "visit": "SCREENING",
            "form": "DM",
            "domain": "DM",
            "age": rng.random_range(18..85i64),
            "sex": if rng.random_bool(0.5) { "M" } else { "F" },
            "race": "WHITE",
            "country": "USA",
            "dmdtc": "2024-01-01",
            "arm": "ARM1",
            "siteid": site,
            "studyid": study_id,
            "usubjid": usubjid,
        }),
        "lb" => serde_json::json!({
            "study": study_id,
            "site": site,
            "subject": subject,
            "visit": "VISIT1",
            "form": "LB",
            "domain": "LB",
            "lbtestcd": "GLUC",
            "lbtest": "Glucose",
            "lborres": rng.random_range(70.0..120.0f64),
            "lborresu": "mg/dL",
            "lbstnrlo": 70.0,
            "lbstnrhi": 110.0,
            "lbdtc": "2024-01-15",
            "siteid": site,
            "studyid": study_id,
            "usubjid": usubjid,
        }),
        "tv" => serde_json::json!({
            "study": study_id,
            "site": site,
            "subject": subject,
            "visit": "VISIT1",
            "form": "TV",
            "domain": "TV",
            "visitnum": 1i64,
            "tvstrl": 1i64,
            "tvenrl": 10i64,
            "armcd": "ARM1",
            "studyid": study_id,
        }),
        "vs" => serde_json::json!({
            "study": study_id,
            "site": site,
            "subject": subject,
            "visit": "VISIT1",
            "form": "VS",
            "domain": "VS",
            "vstestcd": "SYSBP",
            "vstest": "Systolic Blood Pressure",
            "vsorres": rng.random_range(100.0..160.0f64),
            "vsorresu": "mmHg",
            "vsdtc": "2024-01-15",
            "siteid": site,
            "studyid": study_id,
            "usubjid": usubjid,
        }),
        _ => serde_json::json!({}),
    }
}

// ─── Functional Test ──────────────────────────────────────────────────────────

pub async fn run_functional_tests(pool: &deadpool_postgres::Pool) -> Result<()> {
    info!("=== Running Functional CRUD Tests ===");

    let tables = ["ae", "cm", "dm", "lb", "tv", "vs"];
    let key_columns: HashMap<&str, &[&str]> = HashMap::from([
        ("ae", db::AeRecord::KEY_COLUMNS),
        ("cm", db::CmRecord::KEY_COLUMNS),
        ("dm", db::DmRecord::KEY_COLUMNS),
        ("lb", db::LbRecord::KEY_COLUMNS),
        ("tv", db::TvRecord::KEY_COLUMNS),
        ("vs", db::VsRecord::KEY_COLUMNS),
    ]);

    for table in tables.iter() {
        info!("Testing table: {}", table);
        let keys = key_columns[table];

        // 1. CREATE
        let record = random_record(table);
        info!("  [CREATE] Inserting test record into {}", table);
        let inserted = db::insert_json(pool, table, &record).await?;
        assert!(inserted > 0, "CREATE failed for table {}", table);
        info!("  [CREATE] ✓ Inserted {} row(s)", inserted);

        // 2. READ
        info!("  [READ] Listing records from {}", table);
        let filters = HashMap::new();
        let (_records, total) = db::list_records(pool, table, &filters, 1, 10).await?;
        assert!(total > 0, "READ: No records found in table {}", table);
        info!("  [READ] ✓ Found {} record(s) total", total);

        // 3. UPDATE
        // Build update payload with key fields and a modified field
        let update_payload;
        if table == &"tv" {
            update_payload = serde_json::json!({
                "studyid": record["studyid"],
                "site": record["site"],
                "subject": record["subject"],
                "visit": record["visit"],
                "form": "TV_UPDATED",
            });
        } else if table == &"ae" {
            update_payload = serde_json::json!({
                "studyid": record["studyid"],
                "usubjid": record["usubjid"],
                "aeseq": record["aeseq"],
                "aesev": "SEVERE",
            });
        } else if table == &"cm" {
            update_payload = serde_json::json!({
                "studyid": record["studyid"],
                "usubjid": record["usubjid"],
                "cmseq": record["cmseq"],
                "cmdose": 200.0,
            });
        } else if table == &"lb" {
            update_payload = serde_json::json!({
                "studyid": record["studyid"],
                "usubjid": record["usubjid"],
                "lbtestcd": record["lbtestcd"],
                "lbdtc": record["lbdtc"],
                "lborres": 99.9,
            });
        } else if table == &"vs" {
            update_payload = serde_json::json!({
                "studyid": record["studyid"],
                "usubjid": record["usubjid"],
                "vstestcd": record["vstestcd"],
                "vsdtc": record["vsdtc"],
                "vsorres": 120.0,
            });
        } else {
            // dm
            update_payload = serde_json::json!({
                "studyid": record["studyid"],
                "usubjid": record["usubjid"],
                "age": 99,
            });
        }

        info!("  [UPDATE] Updating record in {}", table);
        let updated = db::update_json(pool, table, keys, &update_payload).await?;
        assert!(updated > 0, "UPDATE failed for table {}", table);
        info!("  [UPDATE] ✓ Updated {} row(s)", updated);

        // 4. DELETE
        let mut delete_payload = serde_json::json!({
            "studyid": record["studyid"],
            "usubjid": record["usubjid"],
        });
        if table == &"tv" {
            delete_payload = serde_json::json!({
                "studyid": record["studyid"],
                "site": record["site"],
                "subject": record["subject"],
                "visit": record["visit"],
            });
        } else if table == &"ae" {
            delete_payload = serde_json::json!({
                "studyid": record["studyid"],
                "usubjid": record["usubjid"],
                "aeseq": record["aeseq"],
            });
        } else if table == &"cm" {
            delete_payload = serde_json::json!({
                "studyid": record["studyid"],
                "usubjid": record["usubjid"],
                "cmseq": record["cmseq"],
            });
        } else if table == &"lb" {
            delete_payload = serde_json::json!({
                "studyid": record["studyid"],
                "usubjid": record["usubjid"],
                "lbtestcd": record["lbtestcd"],
                "lbdtc": record["lbdtc"],
            });
        } else if table == &"vs" {
            delete_payload = serde_json::json!({
                "studyid": record["studyid"],
                "usubjid": record["usubjid"],
                "vstestcd": record["vstestcd"],
                "vsdtc": record["vsdtc"],
            });
        }

        info!("  [DELETE] Deleting record from {}", table);
        let deleted = db::delete_by_key_json(pool, table, keys, &delete_payload).await?;
        assert!(deleted > 0, "DELETE failed for table {}", table);
        info!("  [DELETE] ✓ Deleted {} row(s)", deleted);
    }

    info!("=== All Functional Tests PASSED ===");
    Ok(())
}

// ─── Load Testing ─────────────────────────────────────────────────────────────

pub async fn run_load_test(pool: &deadpool_postgres::Pool, config: LoadTestConfig) -> Result<()> {
    info!("=== Load Test Configuration ===");
    info!("  Clients:    {}", config.num_clients);
    info!("  Operations: {}", config.num_operations);
    info!("  Table:      {}", config.table);
    info!(
        "  Read/Write/Update/Delete ratio: {:.0}/{:.0}/{:.0}/{:.0}",
        config.read_ratio * 100.0,
        config.write_ratio * 100.0,
        config.update_ratio * 100.0,
        config.delete_ratio * 100.0,
    );

    let pool = Arc::new(pool.clone());
    let config = Arc::new(config);

    let ops_per_client = config.num_operations / config.num_clients;
    let mut handles = Vec::new();

    let start = Instant::now();

    for client_id in 0..config.num_clients {
        let pool = pool.clone();
        let config = config.clone();
        let handle =
            tokio::spawn(
                async move { client_worker(client_id, pool, config, ops_per_client).await },
            );
        handles.push(handle);
    }

    let mut all_read_stats = OperationStats::default();
    let mut all_write_stats = OperationStats::default();
    let mut all_update_stats = OperationStats::default();
    let mut all_delete_stats = OperationStats::default();
    let mut total_ops: u64 = 0;
    let mut total_failures: u64 = 0;

    for handle in handles {
        match handle.await? {
            Ok(stats) => {
                all_read_stats.total += stats.read.total;
                all_read_stats.successes += stats.read.successes;
                all_read_stats.failures += stats.read.failures;
                all_read_stats.total_duration += stats.read.total_duration;
                if all_read_stats.min_duration > stats.read.min_duration
                    || all_read_stats.min_duration.is_zero()
                {
                    all_read_stats.min_duration = stats.read.min_duration;
                }
                if all_read_stats.max_duration < stats.read.max_duration {
                    all_read_stats.max_duration = stats.read.max_duration;
                }

                all_write_stats.total += stats.write.total;
                all_write_stats.successes += stats.write.successes;
                all_write_stats.failures += stats.write.failures;
                all_write_stats.total_duration += stats.write.total_duration;
                if all_write_stats.min_duration > stats.write.min_duration
                    || all_write_stats.min_duration.is_zero()
                {
                    all_write_stats.min_duration = stats.write.min_duration;
                }
                if all_write_stats.max_duration < stats.write.max_duration {
                    all_write_stats.max_duration = stats.write.max_duration;
                }

                all_update_stats.total += stats.update.total;
                all_update_stats.successes += stats.update.successes;
                all_update_stats.failures += stats.update.failures;
                all_update_stats.total_duration += stats.update.total_duration;
                if all_update_stats.min_duration > stats.update.min_duration
                    || all_update_stats.min_duration.is_zero()
                {
                    all_update_stats.min_duration = stats.update.min_duration;
                }
                if all_update_stats.max_duration < stats.update.max_duration {
                    all_update_stats.max_duration = stats.update.max_duration;
                }

                all_delete_stats.total += stats.delete.total;
                all_delete_stats.successes += stats.delete.successes;
                all_delete_stats.failures += stats.delete.failures;
                all_delete_stats.total_duration += stats.delete.total_duration;
                if all_delete_stats.min_duration > stats.delete.min_duration
                    || all_delete_stats.min_duration.is_zero()
                {
                    all_delete_stats.min_duration = stats.delete.min_duration;
                }
                if all_delete_stats.max_duration < stats.delete.max_duration {
                    all_delete_stats.max_duration = stats.delete.max_duration;
                }

                total_ops +=
                    stats.read.total + stats.write.total + stats.update.total + stats.delete.total;
                total_failures += stats.read.failures
                    + stats.write.failures
                    + stats.update.failures
                    + stats.delete.failures;
            }
            Err(e) => {
                warn!("Client worker failed: {}", e);
            }
        }
    }

    let elapsed = start.elapsed();

    info!("");
    info!("=== Load Test Results ===");
    info!("  Total time:        {:?}", elapsed);
    info!("  Total operations:  {}", total_ops);
    info!("  Total failures:    {}", total_failures);
    info!(
        "  Throughput:        {:.2} ops/sec",
        total_ops as f64 / elapsed.as_secs_f64()
    );
    info!("");
    print_stats("READ", &all_read_stats);
    print_stats("WRITE", &all_write_stats);
    print_stats("UPDATE", &all_update_stats);
    print_stats("DELETE", &all_delete_stats);

    Ok(())
}

fn print_stats(label: &str, stats: &OperationStats) {
    if stats.total == 0 {
        return;
    }
    info!(
        "  {}: total={}, success={}, fail={}, avg={:.2}ms, min={:.2}ms, max={:.2}ms",
        label,
        stats.total,
        stats.successes,
        stats.failures,
        stats.avg_duration().as_secs_f64() * 1000.0,
        stats.min_duration.as_secs_f64() * 1000.0,
        stats.max_duration.as_secs_f64() * 1000.0,
    );
}

struct ClientStats {
    read: OperationStats,
    write: OperationStats,
    update: OperationStats,
    delete: OperationStats,
}

async fn client_worker(
    _client_id: usize,
    pool: Arc<deadpool_postgres::Pool>,
    config: Arc<LoadTestConfig>,
    ops: usize,
) -> Result<ClientStats> {
    let mut rng = StdRng::from_os_rng();
    let mut stats = ClientStats {
        read: OperationStats::default(),
        write: OperationStats::default(),
        update: OperationStats::default(),
        delete: OperationStats::default(),
    };

    let total_ratio =
        config.read_ratio + config.write_ratio + config.update_ratio + config.delete_ratio;

    let keys: &[&str] = match config.table.as_str() {
        "ae" => db::AeRecord::KEY_COLUMNS,
        "cm" => db::CmRecord::KEY_COLUMNS,
        "dm" => db::DmRecord::KEY_COLUMNS,
        "lb" => db::LbRecord::KEY_COLUMNS,
        "tv" => db::TvRecord::KEY_COLUMNS,
        "vs" => db::VsRecord::KEY_COLUMNS,
        _ => &[],
    };

    for _ in 0..ops {
        let roll: f64 = rng.random_range(0.0..total_ratio);

        let op_start = Instant::now();

        if roll < config.read_ratio {
            // READ
            let filters = HashMap::new();
            let result = db::list_records(&pool, &config.table, &filters, 1, 10).await;
            let duration = op_start.elapsed();
            stats.read.record(duration, result.is_ok());
        } else if roll < config.read_ratio + config.write_ratio {
            // WRITE
            let record = random_record(&config.table);
            let result = db::insert_json(&pool, &config.table, &record).await;
            let duration = op_start.elapsed();

            if result.is_ok() {
                stats.write.record(duration, true);
                // Clean up: delete the record we just inserted
                let _ = db::delete_by_key_json(&pool, &config.table, keys, &record).await;
            } else {
                stats.write.record(duration, false);
            }
        } else if roll < config.read_ratio + config.write_ratio + config.update_ratio {
            // UPDATE - update a record that likely exists
            let record = random_record(&config.table);
            // Try to insert first, then update it
            let _ = db::insert_json(&pool, &config.table, &record).await;
            let update_payload = record.clone();
            let result = db::update_json(&pool, &config.table, keys, &update_payload).await;
            let duration = op_start.elapsed();
            stats.update.record(duration, result.is_ok());
            // Clean up
            let _ = db::delete_by_key_json(&pool, &config.table, keys, &record).await;
        } else {
            // DELETE - try to delete, ignore if not found (new random record may not exist)
            let record = random_record(&config.table);
            // Insert first to ensure something to delete
            let _ = db::insert_json(&pool, &config.table, &record).await;
            let result = db::delete_by_key_json(&pool, &config.table, keys, &record).await;
            let duration = op_start.elapsed();
            stats.delete.record(duration, result.is_ok());
        }
    }

    Ok(stats)
}
