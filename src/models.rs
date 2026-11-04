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

    /// Radio callsign on the radar and the strip: three letters from the service
    /// (first letter, then consonants), three digits from the UUID. "checkout" → "CHC412".
    pub fn callsign(&self) -> String {
        let letters: Vec<char> = self
            .service
            .chars()
            .filter(char::is_ascii_alphabetic)
            .map(|c| c.to_ascii_uppercase())
            .collect();
        let is_vowel = |c: &char| "AEIOUY".contains(*c);
        let mut code: String = letters.first().into_iter().collect();
        let rest = letters.iter().skip(1);
        code.extend(rest.clone().filter(|c| !is_vowel(c)).take(2));
        let missing = 3 - code.len();
        code.extend(rest.filter(|c| is_vowel(c)).take(missing));
        while code.len() < 3 {
            code.push('X');
        }
        let b = self.id.as_bytes();
        format!("{code}{:03}", u16::from_be_bytes([b[0], b[1]]) % 1000)
    }

    /// Transponder code: four octal digits from the UUID, never in the 7xxx
    /// emergency range… unless the SLA is breached, then it squawks 7700.
    pub fn squawk(&self, sla_minutes: &i64) -> String {
        if self.sla_breached(sla_minutes) {
            return "7700".into();
        }
        let b = self.id.as_bytes();
        format!("{}{}{}{}", 1 + b[2] % 6, b[3] % 8, b[4] % 8, b[5] % 8)
    }

    /// Compact age for the radar data block and the strip: 35', 3h, 4j.
    pub fn age_compact(&self) -> String {
        let mins = (Utc::now() - self.created_at).num_minutes().max(0);
        match mins {
            0..=59 => format!("{mins}'"),
            60..=1_439 => format!("{}h", mins / 60),
            _ => format!("{}j", mins / 1_440),
        }
    }

    /// Creation time as printed on a flight strip, e.g. 1402Z.
    pub fn created_hm(&self) -> String {
        self.created_at.format("%H%MZ").to_string()
    }

    /// What the status means on the approach.
    pub fn status_gloss(&self) -> &'static str {
        match self.status.as_str() {
            "open" => "en approche",
            "investigating" => "guidé vers la piste",
            _ => "posé",
        }
    }
}

// ---------------------------------------------------------------------------
// Radar: every unresolved incident is a plot, resolved ones are parked by the runway
// ---------------------------------------------------------------------------

/// Radar scope geometry, in SVG user units (viewBox 0 0 660 660).
pub const RADAR_CENTER: f64 = 330.0;

/// Distance to the runway: the more severe, the closer.
pub fn ring_radius(severity: &str) -> f64 {
    match severity {
        "critical" => 80.0,
        "high" => 142.0,
        "medium" => 204.0,
        _ => 262.0,
    }
}

/// Where each ring starts placing its plots, so that the rings do not stack labels.
fn ring_offset(severity: &str) -> f64 {
    match severity {
        "critical" => 40.0,
        "high" => 118.0,
        "medium" => 205.0,
        _ => 298.0,
    }
}

#[derive(Debug, Clone)]
pub struct Dot {
    pub x: i32,
    pub y: i32,
}

/// One aircraft on the scope.
#[derive(Debug, Clone)]
pub struct Blip {
    pub id: String,
    pub title: String,
    pub severity: String,
    pub status: String,
    pub callsign: String,
    /// Second line of the data block, e.g. "CRIT 35'".
    pub info: String,
    pub sla: bool,
    /// Bearing in degrees, clockwise from north: drives the sweep afterglow delay.
    pub bearing: i32,
    pub x: i32,
    pub y: i32,
    /// Data block anchor and position, and the end of its leader line.
    pub anchor: &'static str,
    pub lx: i32,
    pub ly: i32,
    pub kx: i32,
    pub ky: i32,
    /// History dots (behind the aircraft) and heading vector (towards the runway).
    pub trail: Vec<Dot>,
    pub vx: i32,
    pub vy: i32,
}

#[derive(Debug, Clone)]
pub struct Radar {
    pub blips: Vec<Blip>,
    /// Resolved incidents parked by the runway (at most 8 drawn).
    pub parked: Vec<Dot>,
    pub parked_total: usize,
}

impl Radar {
    pub fn build(incidents: &[Incident], sla_minutes: i64) -> Self {
        let c = RADAR_CENTER;
        let polar = |r: f64, deg: f64| {
            let a = deg.to_radians();
            (
                (c + r * a.sin()).round() as i32,
                (c - r * a.cos()).round() as i32,
            )
        };
        let mut blips = Vec::new();
        for sev in ["critical", "high", "medium", "low"] {
            let group: Vec<&Incident> = incidents
                .iter()
                .filter(|i| i.severity == sev && i.status != "resolved")
                .collect();
            let n = group.len().max(1) as f64;
            let step = 360.0 / n;
            for (k, inc) in group.iter().enumerate() {
                let b = inc.id.as_bytes();
                // Spread evenly around the ring, with a little deterministic jitter.
                let jitter = (f64::from(b[6]) / 255.0 - 0.5) * step.min(60.0) * 0.35;
                let bearing = (ring_offset(sev) + k as f64 * step + jitter).rem_euclid(360.0);
                let r = ring_radius(sev) + (f64::from(b[7]) / 255.0 - 0.5) * 14.0;
                let (x, y) = polar(r, bearing);
                let trail = (1..=3)
                    .map(|s| {
                        let (tx, ty) = polar(r + 8.0 * f64::from(s), bearing - 2.5 * f64::from(s));
                        Dot { x: tx, y: ty }
                    })
                    .collect();
                let (vx, vy) = if inc.status == "investigating" {
                    polar(r - 30.0, bearing)
                } else {
                    (x, y)
                };
                let left = f64::from(x) > c + 150.0;
                let lx = if left { x - 16 } else { x + 16 };
                // Near the bottom of the scope the data block sits above the plot.
                let low = f64::from(y) > c + 210.0;
                let ly = if low { y - 46 } else { y - 12 };
                let sla = inc.sla_breached(&sla_minutes);
                let short = match sev {
                    "critical" => "CRIT",
                    "high" => "HIGH",
                    "medium" => "MED",
                    _ => "LOW",
                };
                blips.push(Blip {
                    id: inc.id.to_string(),
                    title: inc.title.clone(),
                    severity: inc.severity.clone(),
                    status: inc.status.clone(),
                    callsign: inc.callsign(),
                    info: format!("{short} {}", inc.age_compact()),
                    sla,
                    bearing: bearing.round() as i32,
                    x,
                    y,
                    anchor: if left { "end" } else { "start" },
                    lx,
                    ly,
                    kx: if left { lx + 3 } else { lx - 3 },
                    ky: if low { ly + 32 } else { ly + 3 },
                    trail,
                    vx,
                    vy,
                });
            }
        }
        let parked_total = incidents.iter().filter(|i| i.status == "resolved").count();
        let parked = (0..parked_total.min(8))
            .map(|k| Dot {
                // Two rows of four on the apron, east of the runway.
                x: (c + 16.0 + 11.0 * (k % 4) as f64) as i32,
                y: (c + 12.0 + 11.0 * (k / 4) as f64) as i32,
            })
            .collect();
        Self {
            blips,
            parked,
            parked_total,
        }
    }
}

// ---------------------------------------------------------------------------
// Controller console: the SQLx calls played for the current page
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct QueryLog {
    /// The SQLx call, e.g. "query_as::<Incident>().fetch_all".
    pub call: &'static str,
    /// The SQL sent to PostgreSQL, whitespace collapsed for display.
    pub sql: String,
    /// Bound parameters, e.g. "$1 = 'open'" (empty when none).
    pub binds: String,
    pub rows: usize,
    pub micros: u128,
}

impl QueryLog {
    pub fn new(
        call: &'static str,
        sql: &str,
        binds: String,
        rows: usize,
        elapsed: std::time::Duration,
    ) -> Self {
        Self {
            call,
            sql: sql.split_whitespace().collect::<Vec<_>>().join(" "),
            binds,
            rows,
            micros: elapsed.as_micros(),
        }
    }

    pub fn ms(&self) -> String {
        format!("{:.2}", self.micros as f64 / 1000.0)
    }
}

/// Pool and server facts shown in the console header.
#[derive(Debug, Clone)]
pub struct DbInfo {
    pub version: String,
    pub pool_open: u32,
    pub pool_idle: usize,
    pub pool_max: u32,
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

#[cfg(test)]
pub mod tests {
    use super::*;
    use chrono::Duration;

    /// An incident created `minutes_ago` minutes ago, with a fixed UUID seed.
    pub fn incident(
        seed: u8,
        service: &str,
        severity: &str,
        status: &str,
        minutes_ago: i64,
    ) -> Incident {
        let created = Utc::now() - Duration::minutes(minutes_ago);
        Incident {
            id: Uuid::from_bytes([seed; 16]),
            title: format!("Incident {seed}"),
            description: None,
            service: service.into(),
            severity: severity.into(),
            status: status.into(),
            created_at: created,
            updated_at: created,
        }
    }

    #[test]
    fn callsign_takes_service_letters_and_three_digits() {
        let c = incident(7, "checkout", "high", "open", 5).callsign();
        assert_eq!(&c[..3], "CHC");
        assert_eq!(c.len(), 6);
        assert!(c[3..].chars().all(|ch| ch.is_ascii_digit()));
        assert_eq!(
            &incident(7, "auth", "low", "open", 5).callsign()[..3],
            "ATH"
        );
        assert_eq!(&incident(7, "ci", "low", "open", 5).callsign()[..3], "CIX");
    }

    #[test]
    fn squawk_is_octal_and_7700_only_when_sla_breached() {
        let calm = incident(0xff, "api", "critical", "open", 5).squawk(&30);
        assert_eq!(calm.len(), 4);
        assert!(calm.chars().all(|c| ('0'..='7').contains(&c)));
        assert!(!calm.starts_with('7'));
        let late = incident(0xff, "api", "critical", "open", 45);
        assert!(late.sla_breached(&30));
        assert_eq!(late.squawk(&30), "7700");
    }

    #[test]
    fn sla_only_counts_unresolved_critical_and_high() {
        assert!(incident(1, "a", "high", "investigating", 31).sla_breached(&30));
        assert!(!incident(1, "a", "high", "resolved", 300).sla_breached(&30));
        assert!(!incident(1, "a", "medium", "open", 300).sla_breached(&30));
        assert!(!incident(1, "a", "critical", "open", 10).sla_breached(&30));
    }

    #[test]
    fn age_compact_uses_minutes_hours_days() {
        assert_eq!(incident(1, "a", "low", "open", 35).age_compact(), "35'");
        assert_eq!(incident(1, "a", "low", "open", 185).age_compact(), "3h");
        assert_eq!(
            incident(1, "a", "low", "open", 3 * 1440 + 5).age_compact(),
            "3j"
        );
    }

    #[test]
    fn radar_puts_severe_incidents_closer_and_parks_resolved_ones() {
        let incidents = vec![
            incident(1, "checkout", "critical", "investigating", 40),
            incident(2, "website", "low", "open", 10),
            incident(3, "reporting", "medium", "resolved", 600),
        ];
        let radar = Radar::build(&incidents, 30);
        assert_eq!(
            radar.blips.len(),
            2,
            "resolved incidents are not in the air"
        );
        assert_eq!(radar.parked_total, 1);
        assert_eq!(radar.parked.len(), 1);

        let dist = |b: &Blip| (f64::from(b.x) - RADAR_CENTER).hypot(f64::from(b.y) - RADAR_CENTER);
        let critical = radar
            .blips
            .iter()
            .find(|b| b.severity == "critical")
            .unwrap();
        let low = radar.blips.iter().find(|b| b.severity == "low").unwrap();
        assert!(dist(critical) < dist(low));
        assert!(critical.sla && !low.sla);
        // Investigating: a heading vector towards the runway; open: none.
        assert!((critical.vx, critical.vy) != (critical.x, critical.y));
        assert_eq!((low.vx, low.vy), (low.x, low.y));
        // Everything stays inside the 660 × 660 scope.
        for b in &radar.blips {
            assert!((0..660).contains(&b.x) && (0..660).contains(&b.y));
        }
    }

    #[test]
    fn query_log_collapses_whitespace() {
        let q = QueryLog::new(
            "query().fetch_one",
            "SELECT 1\n       FROM   incidents",
            String::new(),
            1,
            std::time::Duration::from_micros(1500),
        );
        assert_eq!(q.sql, "SELECT 1 FROM incidents");
        assert_eq!(q.ms(), "1.50");
    }
}
