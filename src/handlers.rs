use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use sqlx::PgPool;
use std::time::Instant;
use uuid::Uuid;

use crate::db;
use crate::models::{
    CreateIncidentForm, DbInfo, FilterQuery, Incident, Platform, QueryLog, Radar, Stats,
    UpdateStatusForm,
};

// ---------------------------------------------------------------------------
// Template → Response bridge
// ---------------------------------------------------------------------------

struct HtmlTemplate<T>(T);

impl<T: Template> IntoResponse for HtmlTemplate<T> {
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("Template render error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Application error
// ---------------------------------------------------------------------------

pub enum AppError {
    Database(sqlx::Error),
    NotFound,
    BadRequest(&'static str),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Database(e) => {
                tracing::error!("Database error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
            // Same status as before, with the tower around it instead of a bare line of text.
            AppError::NotFound => (
                StatusCode::NOT_FOUND,
                HtmlTemplate(NotFoundTemplate {
                    cc: Platform::from_env(),
                }),
            )
                .into_response(),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Database(e)
    }
}

// ---------------------------------------------------------------------------
// Template structs
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    incidents: Vec<Incident>,
    filter: String, // "" = all, "open", "investigating", "resolved"
    stats: Stats,
    radar: Radar,
    queries: Vec<QueryLog>,
    db: DbInfo,
    now: String,
    cc: Platform,
}

#[derive(Template)]
#[template(path = "new.html")]
struct NewTemplate {
    error: String, // empty string = no error
    title: String,
    service: String,
    severity: String,
    description: String,
    insert_sql: String,
    cc: Platform,
}

#[derive(Template)]
#[template(path = "detail.html")]
struct DetailTemplate {
    incident: Incident,
    queries: Vec<QueryLog>,
    db: DbInfo,
    update_sql: String,
    cc: Platform,
}

#[derive(Template)]
#[template(path = "stats.html")]
struct StatsTemplate {
    stats: Stats,
    queries: Vec<QueryLog>,
    db: DbInfo,
    cc: Platform,
}

#[derive(Template)]
#[template(path = "not_found.html")]
struct NotFoundTemplate {
    cc: Platform,
}

/// SQL shown before it runs (form submit, status buttons), whitespace collapsed.
fn compact(sql: &str) -> String {
    sql.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn list_incidents(
    State(pool): State<PgPool>,
    Query(query): Query<FilterQuery>,
) -> Result<impl IntoResponse, AppError> {
    let filter = query.status.unwrap_or_default();
    let db_filter = if filter.is_empty() {
        None
    } else {
        Some(filter.as_str())
    };
    let cc = Platform::from_env();

    // Each SQLx call is timed and logged for the controller console.
    let t = Instant::now();
    let incidents = db::list_incidents(&pool, db_filter).await?;
    let binds = db_filter.map_or(String::new(), |s| format!("$1 = '{s}'"));
    let mut queries = vec![QueryLog::new(
        "query_as::<Incident>().fetch_all",
        db::list_sql(db_filter),
        binds,
        incidents.len(),
        t.elapsed(),
    )];
    let t = Instant::now();
    let stats = db::get_stats(&pool).await?;
    queries.push(QueryLog::new(
        "query().fetch_one",
        db::STATS_SQL,
        String::new(),
        1,
        t.elapsed(),
    ));

    Ok(HtmlTemplate(IndexTemplate {
        radar: Radar::build(&incidents, cc.sla_minutes),
        incidents,
        filter,
        stats,
        queries,
        db: db::info(&pool),
        now: chrono::Utc::now().format("%H%MZ").to_string(),
        cc,
    }))
}

pub async fn new_incident_form() -> impl IntoResponse {
    HtmlTemplate(NewTemplate {
        error: String::new(),
        title: String::new(),
        service: String::new(),
        severity: "medium".into(),
        description: String::new(),
        insert_sql: compact(db::INSERT_SQL),
        cc: Platform::from_env(),
    })
}

/// Server-side limits, aligned with the schema (title VARCHAR(255),
/// service VARCHAR(100)) so that an oversized field is a form error, not a 500.
const TITLE_MAX: usize = 255;
const SERVICE_MAX: usize = 100;
const DESCRIPTION_MAX: usize = 10_000;
const SEVERITIES: [&str; 4] = ["low", "medium", "high", "critical"];

/// Re-display the creation form with an error message, keeping what was typed.
fn form_error(form: &CreateIncidentForm, message: &str) -> Response {
    HtmlTemplate(NewTemplate {
        error: message.to_string(),
        title: form.title.clone(),
        service: form.service.clone(),
        // An invalid value falls back to the default choice.
        severity: if SEVERITIES.contains(&form.severity.as_str()) {
            form.severity.clone()
        } else {
            "medium".into()
        },
        description: form.description.clone(),
        insert_sql: compact(db::INSERT_SQL),
        cc: Platform::from_env(),
    })
    .into_response()
}

pub async fn create_incident(
    State(pool): State<PgPool>,
    Form(form): Form<CreateIncidentForm>,
) -> Result<Response, AppError> {
    if form.title.trim().is_empty() || form.service.trim().is_empty() {
        return Ok(form_error(
            &form,
            "Le titre et le service sont obligatoires.",
        ));
    }

    // PostgreSQL rejects NUL bytes in text columns: a form error, not a 500.
    if [&form.title, &form.service, &form.description]
        .iter()
        .any(|f| f.contains('\0'))
    {
        return Ok(form_error(&form, "Caractère invalide dans le formulaire."));
    }

    if form.title.chars().count() > TITLE_MAX {
        return Ok(form_error(
            &form,
            "Le titre ne doit pas dépasser 255 caractères.",
        ));
    }

    if form.service.chars().count() > SERVICE_MAX {
        return Ok(form_error(
            &form,
            "Le service ne doit pas dépasser 100 caractères.",
        ));
    }

    if form.description.chars().count() > DESCRIPTION_MAX {
        return Ok(form_error(
            &form,
            "La description ne doit pas dépasser 10 000 caractères.",
        ));
    }

    if !SEVERITIES.contains(&form.severity.as_str()) {
        return Ok(form_error(&form, "Sévérité invalide."));
    }

    let incident = db::create_incident(&pool, &form).await?;
    Ok(Redirect::to(&format!("/incidents/{}", incident.id)).into_response())
}

pub async fn incident_detail(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let t = Instant::now();
    let incident = db::get_incident(&pool, id)
        .await?
        .ok_or(AppError::NotFound)?;
    let queries = vec![QueryLog::new(
        "query_as::<Incident>().fetch_optional",
        db::GET_SQL,
        format!("$1 = '{id}'"),
        1,
        t.elapsed(),
    )];
    Ok(HtmlTemplate(DetailTemplate {
        incident,
        queries,
        db: db::info(&pool),
        update_sql: compact(db::UPDATE_STATUS_SQL),
        cc: Platform::from_env(),
    }))
}

pub async fn update_status(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Form(form): Form<UpdateStatusForm>,
) -> Result<impl IntoResponse, AppError> {
    if !["open", "investigating", "resolved"].contains(&form.status.as_str()) {
        return Err(AppError::BadRequest("Statut invalide"));
    }
    db::update_incident_status(&pool, id, &form.status)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Redirect::to(&format!("/incidents/{}", id)))
}

/// Health check used by Clever Cloud (CC_HEALTH_CHECK_PATH=/health): 200 only
/// when PostgreSQL answers, 503 otherwise.
pub async fn health(State(pool): State<PgPool>) -> impl IntoResponse {
    match sqlx::query("SELECT 1").execute(&pool).await {
        Ok(_) => (StatusCode::OK, "OK"),
        Err(e) => {
            tracing::error!("Health check failed: {e}");
            (StatusCode::SERVICE_UNAVAILABLE, "DB unavailable")
        }
    }
}

pub async fn stats(State(pool): State<PgPool>) -> Result<impl IntoResponse, AppError> {
    let t = Instant::now();
    let stats = db::get_stats(&pool).await?;
    let queries = vec![QueryLog::new(
        "query().fetch_one",
        db::STATS_SQL,
        String::new(),
        1,
        t.elapsed(),
    )];
    Ok(HtmlTemplate(StatsTemplate {
        stats,
        queries,
        db: db::info(&pool),
        cc: Platform::from_env(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tests::incident;

    fn platform(announcement: &str) -> Platform {
        Platform {
            live: false,
            app_name: "—".into(),
            app_id: "—".into(),
            instance: "—".into(),
            instance_type: "—".into(),
            commit: "—".into(),
            deployment: "—".into(),
            env_name: "production".into(),
            team: "SRE Paris".into(),
            sla_minutes: 30,
            announcement: announcement.into(),
        }
    }

    fn db_info() -> DbInfo {
        DbInfo {
            version: "17.6".into(),
            pool_open: 1,
            pool_idle: 1,
            pool_max: 2,
        }
    }

    #[test]
    fn radar_page_shows_plots_strips_queries_and_demo_variables() {
        let incidents = vec![
            incident(1, "checkout", "critical", "investigating", 40),
            incident(2, "website", "low", "resolved", 600),
        ];
        let html = IndexTemplate {
            radar: Radar::build(&incidents, 30),
            incidents,
            filter: String::new(),
            stats: Stats {
                total: 2,
                open: 0,
                investigating: 1,
                resolved: 1,
                critical: 1,
                high: 0,
                medium: 0,
                low: 1,
            },
            queries: vec![QueryLog::new(
                "query_as::<Incident>().fetch_all",
                db::LIST_ALL_SQL,
                String::new(),
                2,
                std::time::Duration::from_micros(900),
            )],
            db: db_info(),
            now: "1400Z".into(),
            cc: platform("Maintenance ce soir à 22 h"),
        }
        .render()
        .unwrap();

        assert!(html.contains("1 incident en vol."));
        assert!(html.contains(r#"class="blip is-critical is-investigating is-sla""#));
        assert!(html.contains(">7700<"), "SLA breach squawks 7700");
        assert!(html.contains("SLA dépassé"));
        assert!(
            html.contains("1 posé<"),
            "resolved incident parked by the runway"
        );
        assert!(html.contains("FROM incidents ORDER BY status = &#x27;resolved&#x27;"));
        assert!(html.contains("Astreinte\u{a0}: SRE Paris"));
        assert!(
            html.contains("Maintenance ce soir à 22 h"),
            "ANNOUNCEMENT shown as NOTAM"
        );
        assert!(html.contains("scope__env is-production"));
        assert!(html.contains("Running on Clever Cloud"));
        assert!(html.contains("Cette app tourne sur Clever Cloud. Vous aussi, certifiez-vous."));
        assert!(html.contains("https://academy.clever.cloud/"));
    }

    #[test]
    fn calm_sky_when_nothing_is_open() {
        let html = IndexTemplate {
            radar: Radar::build(&[], 30),
            incidents: vec![],
            filter: String::new(),
            stats: Stats {
                total: 0,
                open: 0,
                investigating: 0,
                resolved: 0,
                critical: 0,
                high: 0,
                medium: 0,
                low: 0,
            },
            queries: vec![],
            db: db_info(),
            now: "1400Z".into(),
            cc: platform(""),
        }
        .render()
        .unwrap();
        assert!(html.contains("Ciel dégagé."));
        assert!(html.contains("Aucun incident enregistré"));
        assert!(!html.contains("NOTAM"));
    }

    #[test]
    fn detail_page_offers_the_three_statuses_and_the_update() {
        let html = DetailTemplate {
            incident: incident(1, "checkout", "high", "resolved", 90),
            queries: vec![],
            db: db_info(),
            update_sql: compact(db::UPDATE_STATUS_SQL),
            cc: platform(""),
        }
        .render()
        .unwrap();
        for status in ["open", "investigating", "resolved"] {
            assert!(html.contains(&format!(r#"name="status" value="{status}""#)));
        }
        assert!(
            html.contains("approach__scene is-step-2"),
            "resolved: landed"
        );
        assert!(html.contains("UPDATE incidents SET status = $1, updated_at = NOW() WHERE id = $2"));
    }
}
