use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::models::{CreateIncidentForm, Incident, Stats};

pub async fn list_incidents(
    pool: &PgPool,
    status_filter: Option<&str>,
) -> Result<Vec<Incident>, sqlx::Error> {
    if let Some(status) = status_filter {
        sqlx::query_as::<_, Incident>(
            r#"SELECT id, title, description, service, severity, status, created_at, updated_at
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
                 created_at DESC"#,
        )
        .bind(status)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, Incident>(
            r#"SELECT id, title, description, service, severity, status, created_at, updated_at
               FROM incidents
               ORDER BY
                 status = 'resolved',
                 CASE severity
                   WHEN 'critical' THEN 1
                   WHEN 'high'     THEN 2
                   WHEN 'medium'   THEN 3
                   ELSE 4
                 END,
                 created_at DESC"#,
        )
        .fetch_all(pool)
        .await
    }
}

pub async fn get_incident(pool: &PgPool, id: Uuid) -> Result<Option<Incident>, sqlx::Error> {
    sqlx::query_as::<_, Incident>(
        r#"SELECT id, title, description, service, severity, status, created_at, updated_at
           FROM incidents WHERE id = $1"#,
    )
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

    sqlx::query_as::<_, Incident>(
        r#"INSERT INTO incidents (title, description, service, severity)
           VALUES ($1, $2, $3, $4)
           RETURNING id, title, description, service, severity, status, created_at, updated_at"#,
    )
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
    sqlx::query_as::<_, Incident>(
        r#"UPDATE incidents
           SET status = $1, updated_at = NOW()
           WHERE id = $2
           RETURNING id, title, description, service, severity, status, created_at, updated_at"#,
    )
    .bind(status)
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn get_stats(pool: &PgPool) -> Result<Stats, sqlx::Error> {
    let row = sqlx::query(
        r#"SELECT
               COUNT(*)                                      AS total,
               COUNT(*) FILTER (WHERE status = 'open')       AS open_count,
               COUNT(*) FILTER (WHERE status = 'investigating') AS investigating_count,
               COUNT(*) FILTER (WHERE status = 'resolved')   AS resolved_count,
               COUNT(*) FILTER (WHERE severity = 'critical') AS critical_count,
               COUNT(*) FILTER (WHERE severity = 'high')     AS high_count,
               COUNT(*) FILTER (WHERE severity = 'medium')   AS medium_count,
               COUNT(*) FILTER (WHERE severity = 'low')      AS low_count
           FROM incidents"#,
    )
    .fetch_one(pool)
    .await?;

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
