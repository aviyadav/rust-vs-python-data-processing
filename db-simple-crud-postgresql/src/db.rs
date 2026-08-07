use anyhow::{Context, Result};
use chrono::NaiveDate;
use deadpool_postgres::{Config, Pool, Runtime};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ─── Connection Pool ──────────────────────────────────────────────────────────

pub fn create_pool() -> Pool {
    let mut cfg = Config::new();
    cfg.host = Some("localhost".to_string());
    cfg.port = Some(5432);
    cfg.dbname = Some("benchmark_poc_db".to_string());
    cfg.user = Some("pocuser".to_string());
    cfg.password = Some("password".to_string());
    cfg.create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls)
        .expect("Failed to create connection pool")
}

pub async fn check_health(pool: &Pool) -> Result<()> {
    let client = pool.get().await?;
    client.simple_query("SELECT 1").await?;
    Ok(())
}

// ─── Serde Helpers ────────────────────────────────────────────────────────────

fn deserialize_optional_date<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(s) if !s.is_empty() => NaiveDate::parse_from_str(&s, "%Y-%m-%d")
            .map(Some)
            .map_err(serde::de::Error::custom),
        _ => Ok(None),
    }
}

// ─── AE (Adverse Events) ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct AeRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub study: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aeseq: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aeterm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aedecod: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aebodsys: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_optional_date")]
    pub aestdtc: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_optional_date")]
    pub aeendtc: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aesev: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aerel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aeout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ae_incident_group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub siteid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub studyid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usubjid: Option<String>,
}

impl AeRecord {
    pub const KEY_COLUMNS: &'static [&'static str] = &["STUDYID", "USUBJID", "AESEQ"];
}

// ─── CM (Concomitant Medications) ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct CmRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub study: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmseq: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmtrt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmdecod: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmcat: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_optional_date")]
    pub cmstdtc: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_optional_date")]
    pub cmendtc: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmdose: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmdosu: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmdosfrm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmroute: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmdosfrq: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub siteid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub studyid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usubjid: Option<String>,
}

impl CmRecord {
    pub const KEY_COLUMNS: &'static [&'static str] = &["STUDYID", "USUBJID", "CMSEQ"];
}

// ─── DM (Demographics) ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct DmRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub study: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub race: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_optional_date")]
    pub dmdtc: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub siteid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub studyid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usubjid: Option<String>,
}

impl DmRecord {
    pub const KEY_COLUMNS: &'static [&'static str] = &["STUDYID", "USUBJID"];
}

// ─── LB (Laboratory) ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct LbRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub study: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lbtestcd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lbtest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lborres: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lborresu: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lbstnrlo: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lbstnrhi: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_optional_date")]
    pub lbdtc: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub siteid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub studyid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usubjid: Option<String>,
}

impl LbRecord {
    pub const KEY_COLUMNS: &'static [&'static str] = &["STUDYID", "USUBJID", "LBTESTCD", "LBDTC"];
}

// ─── TV (Trial Visits) ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct TvRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub study: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visitnum: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tvstrl: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tvenrl: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub armcd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub studyid: Option<String>,
}

impl TvRecord {
    pub const KEY_COLUMNS: &'static [&'static str] = &["STUDYID", "SITE", "SUBJECT", "VISIT"];
}

// ─── VS (Vital Signs) ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct VsRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub study: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vstestcd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vstest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vsorres: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vsorresu: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_optional_date")]
    pub vsdtc: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub siteid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub studyid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usubjid: Option<String>,
}

impl VsRecord {
    pub const KEY_COLUMNS: &'static [&'static str] = &["STUDYID", "USUBJID", "VSTESTCD", "VSDTC"];
}

// ─── JSON Value to Postgres Param Conversion ──────────────────────────────────

/// Convert a serde_json::Value to a boxed Postgres ToSql parameter.
fn json_to_param(v: &serde_json::Value) -> Box<dyn tokio_postgres::types::ToSql + Sync + Send> {
    match v {
        serde_json::Value::Null => Box::new(None::<String>),
        serde_json::Value::Bool(b) => Box::new(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Box::new(i)
            } else if let Some(f) = n.as_f64() {
                Box::new(f)
            } else {
                Box::new(n.to_string())
            }
        }
        serde_json::Value::String(s) => {
            // Try to parse as date first
            if let Ok(date) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                Box::new(date)
            } else {
                Box::new(s.clone())
            }
        }
        _ => Box::new(v.to_string()),
    }
}

// ─── Generic CRUD Functions ───────────────────────────────────────────────────

/// Insert a record using a JSON value. Returns the number of rows inserted.
pub async fn insert_json(pool: &Pool, table: &str, record: &serde_json::Value) -> Result<u64> {
    let client = pool.get().await?;
    let obj = record.as_object().context("Record must be a JSON object")?;

    let columns: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
    let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("${}", i)).collect();
    let col_names: Vec<String> = columns
        .iter()
        .map(|c| format!("\"{}\"", c.to_uppercase()))
        .collect();

    let sql = format!(
        "INSERT INTO public.\"{}\" ({}) VALUES ({})",
        table,
        col_names.join(", "),
        placeholders.join(", ")
    );

    let params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> =
        obj.values().map(json_to_param).collect();

    let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
        .iter()
        .map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync))
        .collect();

    let n = client.execute(&sql, &param_refs).await?;
    Ok(n)
}

/// Update records matching key fields. `record` is a JSON object where key fields
/// are used for matching and all other fields are set.
pub async fn update_json(
    pool: &Pool,
    table: &str,
    key_cols: &[&str],
    record: &serde_json::Value,
) -> Result<u64> {
    let client = pool.get().await?;
    let obj = record.as_object().context("Record must be a JSON object")?;

    let key_set: std::collections::HashSet<&str> = key_cols.iter().copied().collect();

    let mut set_clauses: Vec<String> = Vec::new();
    let mut where_clauses: Vec<String> = Vec::new();
    let mut param_idx = 1usize;

    let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> = Vec::new();

    for (k, v) in obj.iter() {
        let col_upper = k.to_uppercase();
        if key_set.contains(col_upper.as_str()) {
            where_clauses.push(format!("\"{}\" = ${}", col_upper, param_idx));
        } else {
            set_clauses.push(format!("\"{}\" = ${}", col_upper, param_idx));
        }
        params.push(json_to_param(v));
        param_idx += 1;
    }

    if where_clauses.is_empty() {
        anyhow::bail!("No key columns provided for UPDATE — cannot identify which rows to update");
    }
    if set_clauses.is_empty() {
        anyhow::bail!("No data columns provided for UPDATE");
    }

    let sql = format!(
        "UPDATE public.\"{}\" SET {} WHERE {}",
        table,
        set_clauses.join(", "),
        where_clauses.join(" AND ")
    );

    let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
        .iter()
        .map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync))
        .collect();

    let n = client.execute(&sql, &param_refs).await?;
    Ok(n)
}

/// Delete records matching key fields from a JSON record.
pub async fn delete_by_key_json(
    pool: &Pool,
    table: &str,
    key_cols: &[&str],
    record: &serde_json::Value,
) -> Result<u64> {
    let client = pool.get().await?;
    let obj = record.as_object().context("Record must be a JSON object")?;

    let key_set: std::collections::HashSet<&str> = key_cols.iter().copied().collect();

    let mut where_clauses: Vec<String> = Vec::new();
    let mut param_idx = 1usize;
    let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> = Vec::new();

    for (k, v) in obj.iter() {
        let col_upper = k.to_uppercase();
        if key_set.contains(col_upper.as_str()) {
            where_clauses.push(format!("\"{}\" = ${}", col_upper, param_idx));
            params.push(json_to_param(v));
            param_idx += 1;
        }
    }

    if where_clauses.is_empty() {
        anyhow::bail!("No key columns provided for DELETE");
    }

    let sql = format!(
        "DELETE FROM public.\"{}\" WHERE {}",
        table,
        where_clauses.join(" AND ")
    );

    let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
        .iter()
        .map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync))
        .collect();

    let n = client.execute(&sql, &param_refs).await?;
    Ok(n)
}

/// List records from a table with optional filtering and pagination.
pub async fn list_records(
    pool: &Pool,
    table: &str,
    filters: &std::collections::HashMap<String, String>,
    page: u32,
    page_size: u32,
) -> Result<(Vec<serde_json::Value>, u64)> {
    let client = pool.get().await?;
    let offset = (page.saturating_sub(1)) * page_size;

    // Build WHERE clause from filters
    let mut where_parts: Vec<String> = Vec::new();
    let mut param_idx = 1usize;
    let filter_values: Vec<String> = filters.values().map(|v| v.to_string()).collect();
    for _ in filters.iter() {
        where_parts.push(format!("\"{}\" = ${}", "COL", param_idx));
        param_idx += 1;
    }

    // Rebuild with correct column names
    where_parts.clear();
    param_idx = 1;
    for (k, _v) in filters.iter() {
        where_parts.push(format!("\"{}\" = ${}", k.to_uppercase(), param_idx));
        param_idx += 1;
    }

    let where_clause = if where_parts.is_empty() {
        "1=1".to_string()
    } else {
        where_parts.join(" AND ")
    };

    // Count query
    let count_sql = format!(
        "SELECT COUNT(*) FROM public.\"{}\" WHERE {}",
        table, where_clause
    );

    let count_param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = filter_values
        .iter()
        .map(|v| v as &(dyn tokio_postgres::types::ToSql + Sync))
        .collect();

    let count_row = client.query_one(&count_sql, &count_param_refs).await?;
    let total: i64 = count_row.get(0);

    // Select query
    let select_sql = format!(
        "SELECT row_to_json(t) FROM (SELECT * FROM public.\"{}\" WHERE {} ORDER BY ctid LIMIT ${} OFFSET ${}) t",
        table, where_clause, param_idx, param_idx + 1
    );

    let mut all_params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> = Vec::new();
    for v in &filter_values {
        all_params.push(Box::new(v.clone()));
    }
    all_params.push(Box::new(page_size as i64));
    all_params.push(Box::new(offset as i64));

    let select_param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = all_params
        .iter()
        .map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync))
        .collect();

    let rows = client.query(&select_sql, &select_param_refs).await?;
    let records: Vec<serde_json::Value> = rows
        .iter()
        .filter_map(|r| r.try_get::<_, Option<serde_json::Value>>(0).ok().flatten())
        .collect();

    Ok((records, total as u64))
}
