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
    /// Clever Brand Kit badge modifier for the severity level.
    pub fn severity_class(&self) -> &'static str {
        match self.severity.as_str() {
            "critical" => "cc-badge--critical",
            "high" => "cc-badge--danger",
            "medium" => "cc-badge--warn",
            _ => "cc-badge--neutral",
        }
    }

    /// Clever Brand Kit badge modifier for the workflow status.
    pub fn status_class(&self) -> &'static str {
        match self.status.as_str() {
            "open" => "cc-badge--info",
            "investigating" => "cc-badge--warn",
            _ => "cc-badge--ok",
        }
    }

    /// First 8 characters of the UUID, for compact list rows.
    pub fn short_id(&self) -> String {
        self.id.to_string().chars().take(8).collect()
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

/// What Clever Cloud injects into the runtime environment.
/// Every field falls back to "—" when the variable is absent (local run).
#[derive(Debug, Clone)]
pub struct Platform {
    pub live: bool,
    pub app_name: String,
    pub app_id: String,
    pub instance: String,
    pub instance_type: String,
    pub commit: String,
    pub deployment: String,
}

impl Platform {
    pub fn from_env() -> Self {
        let g = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        let or_dash = |v: Option<String>| v.unwrap_or_else(|| "—".into());
        let cut = |v: Option<String>, n: usize| {
            or_dash(v.map(|s| s.chars().take(n).collect::<String>()))
        };
        let instance = match (g("INSTANCE_NUMBER"), g("CC_PRETTY_INSTANCE_NAME")) {
            (Some(n), Some(p)) => format!("#{n} · {p}"),
            (Some(n), None) => format!("#{n}"),
            _ => "—".into(),
        };
        Self {
            live: g("APP_ID").is_some(),
            app_name: or_dash(g("CC_APP_NAME")),
            app_id: or_dash(g("APP_ID")),
            instance,
            instance_type: or_dash(g("INSTANCE_TYPE")),
            commit: cut(g("CC_COMMIT_ID"), 7),
            deployment: cut(g("CC_DEPLOYMENT_ID"), 16),
        }
    }
}
