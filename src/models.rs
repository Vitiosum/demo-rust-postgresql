use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Incident {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub service: String,
    pub severity: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Incident {
    pub fn severity_class(&self) -> &'static str {
        match self.severity.as_str() {
            "critical" => "badge-critical",
            "high" => "badge-high",
            "medium" => "badge-medium",
            _ => "badge-low",
        }
    }

    pub fn status_class(&self) -> &'static str {
        match self.status.as_str() {
            "open" => "badge-open",
            "investigating" => "badge-investigating",
            _ => "badge-resolved",
        }
    }

    pub fn created_str(&self) -> String {
        self.created_at.format("%Y-%m-%d %H:%M UTC").to_string()
    }

    pub fn updated_str(&self) -> String {
        self.updated_at.format("%Y-%m-%d %H:%M UTC").to_string()
    }

    pub fn has_description(&self) -> bool {
        self.description.as_ref().map_or(false, |d| !d.is_empty())
    }

    pub fn description_text(&self) -> &str {
        self.description.as_deref().unwrap_or("")
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateIncidentForm {
    pub title: String,
    pub description: String,
    pub service: String,
    pub severity: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusForm {
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct FilterQuery {
    pub status: Option<String>,
}

#[derive(Debug)]
pub struct Stats {
    pub total: i64,
    pub open: i64,
    pub investigating: i64,
    pub resolved: i64,
    pub critical: i64,
}
