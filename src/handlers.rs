use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db;
use crate::models::{CreateIncidentForm, FilterQuery, Incident, Platform, Stats, UpdateStatusForm};

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
            AppError::NotFound => (StatusCode::NOT_FOUND, "Incident not found").into_response(),
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
    cc: Platform,
}

#[derive(Template)]
#[template(path = "detail.html")]
struct DetailTemplate {
    incident: Incident,
    cc: Platform,
}

#[derive(Template)]
#[template(path = "stats.html")]
struct StatsTemplate {
    stats: Stats,
    cc: Platform,
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
    let incidents = db::list_incidents(&pool, db_filter).await?;
    let stats = db::get_stats(&pool).await?;
    Ok(HtmlTemplate(IndexTemplate {
        incidents,
        filter,
        stats,
        cc: Platform::from_env(),
    }))
}

pub async fn new_incident_form() -> impl IntoResponse {
    HtmlTemplate(NewTemplate {
        error: String::new(),
        title: String::new(),
        service: String::new(),
        severity: "medium".into(),
        description: String::new(),
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
    let incident = db::get_incident(&pool, id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(HtmlTemplate(DetailTemplate {
        incident,
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
    let stats = db::get_stats(&pool).await?;
    Ok(HtmlTemplate(StatsTemplate {
        stats,
        cc: Platform::from_env(),
    }))
}
