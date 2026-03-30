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
use crate::models::{CreateIncidentForm, FilterQuery, Incident, Stats, UpdateStatusForm};

// ---------------------------------------------------------------------------
// Template → Response bridge
// ---------------------------------------------------------------------------

struct HtmlTemplate<T>(T);

impl<T: Template> IntoResponse for HtmlTemplate<T> {
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template render error: {e}"),
            )
                .into_response(),
        }
    }
}

// ---------------------------------------------------------------------------
// Application error
// ---------------------------------------------------------------------------

pub enum AppError {
    Database(sqlx::Error),
    NotFound,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Database(e) => {
                tracing::error!("Database error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
            AppError::NotFound => {
                (StatusCode::NOT_FOUND, "Incident not found").into_response()
            }
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
}

#[derive(Template)]
#[template(path = "new.html")]
struct NewTemplate {
    error: String, // empty string = no error
}

#[derive(Template)]
#[template(path = "detail.html")]
struct DetailTemplate {
    incident: Incident,
}

#[derive(Template)]
#[template(path = "stats.html")]
struct StatsTemplate {
    stats: Stats,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn list_incidents(
    State(pool): State<PgPool>,
    Query(query): Query<FilterQuery>,
) -> Result<impl IntoResponse, AppError> {
    let filter = query.status.unwrap_or_default();
    let db_filter = if filter.is_empty() { None } else { Some(filter.as_str()) };
    let incidents = db::list_incidents(&pool, db_filter).await?;
    Ok(HtmlTemplate(IndexTemplate { incidents, filter }))
}

pub async fn new_incident_form() -> impl IntoResponse {
    HtmlTemplate(NewTemplate { error: String::new() })
}

pub async fn create_incident(
    State(pool): State<PgPool>,
    Form(form): Form<CreateIncidentForm>,
) -> Result<Response, AppError> {
    if form.title.trim().is_empty() || form.service.trim().is_empty() {
        return Ok(HtmlTemplate(NewTemplate {
            error: "Title and service are required.".to_string(),
        })
        .into_response());
    }

    if !["low", "medium", "high", "critical"].contains(&form.severity.as_str()) {
        return Ok(HtmlTemplate(NewTemplate {
            error: "Invalid severity value.".to_string(),
        })
        .into_response());
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
    Ok(HtmlTemplate(DetailTemplate { incident }))
}

pub async fn update_status(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Form(form): Form<UpdateStatusForm>,
) -> Result<impl IntoResponse, AppError> {
    if !["open", "investigating", "resolved"].contains(&form.status.as_str()) {
        return Err(AppError::NotFound);
    }
    db::update_incident_status(&pool, id, &form.status)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Redirect::to(&format!("/incidents/{}", id)))
}

pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

pub async fn stats(State(pool): State<PgPool>) -> Result<impl IntoResponse, AppError> {
    let stats = db::get_stats(&pool).await?;
    Ok(HtmlTemplate(StatsTemplate { stats }))
}
