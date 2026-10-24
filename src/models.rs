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

    /// Human-readable age, e.g. "il y a 12 min".
    pub fn age_str(&self) -> String {
        let secs = (Utc::now() - self.created_at).num_seconds().max(0);
        match secs {
            0..=59 => "à l'instant".into(),
            60..=3_599 => format!("il y a {} min", secs / 60),
            3_600..=86_399 => format!("il y a {} h", secs / 3_600),
            _ => format!("il y a {} j", secs / 86_400),
        }
    }

    /// Critical or high incident still unresolved after the SLA (SLA_CRITICAL_MINUTES).
    pub fn sla_breached(&self, sla_minutes: &i64) -> bool {
        matches!(self.severity.as_str(), "critical" | "high")
            && self.status != "resolved"
            && (Utc::now() - self.created_at).num_minutes() >= *sla_minutes
    }

    /// Position in the open → investigating → resolved workflow.
    pub fn status_step(&self) -> u8 {
        match self.status.as_str() {
            "open" => 0,
            "investigating" => 1,
            _ => 2,
        }
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
    pub high: i64,
    pub medium: i64,
    pub low: i64,
}

impl Stats {
    /// Share of the total, in percent, for proportional bars.
    pub fn pct(&self, n: &i64) -> i64 {
        if self.total == 0 {
            0
        } else {
            n * 100 / self.total
        }
    }

    pub fn active(&self) -> i64 {
        self.open + self.investigating
    }
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
    /// Demo settings, set in the Clever Cloud console (environment variables tab).
    pub env_name: String,
    pub team: String,
    pub sla_minutes: i64,
    pub announcement: String,
}

impl Platform {
    pub fn from_env() -> Self {
        let g = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        let or_dash = |v: Option<String>| v.unwrap_or_else(|| "—".into());
        let cut =
            |v: Option<String>, n: usize| or_dash(v.map(|s| s.chars().take(n).collect::<String>()));
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
            env_name: g("APP_ENV").unwrap_or_else(|| "local".into()),
            team: g("TEAM_NAME").unwrap_or_else(|| "Équipe SRE".into()),
            sla_minutes: g("SLA_CRITICAL_MINUTES")
                .and_then(|v| v.trim().parse().ok())
                .filter(|m: &i64| *m > 0)
                .unwrap_or(30),
            announcement: g("ANNOUNCEMENT").unwrap_or_default(),
        }
    }
}
