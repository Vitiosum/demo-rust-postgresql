use sqlx::{PgPool, Row};
use std::sync::OnceLock;
use uuid::Uuid;

use crate::models::{CreateIncidentForm, DbInfo, Incident, Stats};

// The SQL lives in constants so that the controller console shows, word for
// word, what SQLx sends to PostgreSQL.

pub const LIST_ALL_SQL: &str = r#"SELECT id, title, description, service, severity, status, created_at, updated_at
       FROM incidents
       ORDER BY
         status = 'resolved',
         CASE severity
           WHEN 'critical' THEN 1
           WHEN 'high'     THEN 2
           WHEN 'medium'   THEN 3
           ELSE 4
         END,
         created_at DESC"#;

pub const LIST_BY_STATUS_SQL: &str = r#"SELECT id, title, description, service, severity, status, created_at, updated_at
       FROM incidents
       WHERE status = $1
       ORDER BY
         status = 'resolved',
         CASE severity
           WHEN 'critical' THEN 1
           WHEN 'high'     THEN 2
           WHEN 'medium'   THEN 3
           ELSE 4
         END,
         created_at DESC"#;

pub const GET_SQL: &str = r#"SELECT id, title, description, service, severity, status, created_at, updated_at
       FROM incidents WHERE id = $1"#;

pub const INSERT_SQL: &str = r#"INSERT INTO incidents (title, description, service, severity)
       VALUES ($1, $2, $3, $4)
       RETURNING id, title, description, service, severity, status, created_at, updated_at"#;

pub const UPDATE_STATUS_SQL: &str = r#"UPDATE incidents
       SET status = $1, updated_at = NOW()
       WHERE id = $2
       RETURNING id, title, description, service, severity, status, created_at, updated_at"#;

pub const STATS_SQL: &str = r#"SELECT
       COUNT(*)                                         AS total,
       COUNT(*) FILTER (WHERE status = 'open')          AS open_count,
       COUNT(*) FILTER (WHERE status = 'investigating') AS investigating_count,
       COUNT(*) FILTER (WHERE status = 'resolved')      AS resolved_count,
       COUNT(*) FILTER (WHERE severity = 'critical')    AS critical_count,
       COUNT(*) FILTER (WHERE severity = 'high')        AS high_count,
       COUNT(*) FILTER (WHERE severity = 'medium')      AS medium_count,
       COUNT(*) FILTER (WHERE severity = 'low')         AS low_count
   FROM incidents"#;

pub fn list_sql(status_filter: Option<&str>) -> &'static str {
    if status_filter.is_some() {
        LIST_BY_STATUS_SQL
    } else {
        LIST_ALL_SQL
    }
}

pub async fn list_incidents(
    pool: &PgPool,
    status_filter: Option<&str>,
) -> Result<Vec<Incident>, sqlx::Error> {
    let query = sqlx::query_as::<_, Incident>(list_sql(status_filter));
    match status_filter {
        Some(status) => query.bind(status).fetch_all(pool).await,
        None => query.fetch_all(pool).await,
    }
}

pub async fn get_incident(pool: &PgPool, id: Uuid) -> Result<Option<Incident>, sqlx::Error> {
    sqlx::query_as::<_, Incident>(GET_SQL)
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn create_incident(
    pool: &PgPool,
    form: &CreateIncidentForm,
) -> Result<Incident, sqlx::Error> {
    let description: Option<&str> = if form.description.trim().is_empty() {
        None
    } else {
        Some(form.description.as_str())
    };

    sqlx::query_as::<_, Incident>(INSERT_SQL)
        .bind(&form.title)
        .bind(description)
        .bind(&form.service)
        .bind(&form.severity)
        .fetch_one(pool)
        .await
}

pub async fn update_incident_status(
    pool: &PgPool,
    id: Uuid,
    status: &str,
) -> Result<Option<Incident>, sqlx::Error> {
    sqlx::query_as::<_, Incident>(UPDATE_STATUS_SQL)
        .bind(status)
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn get_stats(pool: &PgPool) -> Result<Stats, sqlx::Error> {
    let row = sqlx::query(STATS_SQL).fetch_one(pool).await?;

    Ok(Stats {
        total: row.try_get::<i64, _>("total")?,
        open: row.try_get::<i64, _>("open_count")?,
        investigating: row.try_get::<i64, _>("investigating_count")?,
        resolved: row.try_get::<i64, _>("resolved_count")?,
        critical: row.try_get::<i64, _>("critical_count")?,
        high: row.try_get::<i64, _>("high_count")?,
        medium: row.try_get::<i64, _>("medium_count")?,
        low: row.try_get::<i64, _>("low_count")?,
    })
}

/// PostgreSQL version, read once at startup (`SHOW server_version`).
static SERVER_VERSION: OnceLock<String> = OnceLock::new();

pub async fn load_server_version(pool: &PgPool) {
    let version = sqlx::query_scalar::<_, String>("SHOW server_version")
        .fetch_one(pool)
        .await
        .ok()
        .and_then(|v| v.split_whitespace().next().map(str::to_owned))
        .unwrap_or_else(|| "?".into());
    let _ = SERVER_VERSION.set(version);
}

/// Pool and server facts for the controller console.
pub fn info(pool: &PgPool) -> DbInfo {
    DbInfo {
        version: SERVER_VERSION.get().cloned().unwrap_or_else(|| "?".into()),
        pool_open: pool.size(),
        pool_idle: pool.num_idle(),
        pool_max: pool.options().get_max_connections(),
    }
}
