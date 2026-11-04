use axum::{
    http::{
        header::{
            CACHE_CONTROL, CONTENT_SECURITY_POLICY, CONTENT_TYPE, REFERRER_POLICY,
            X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
        },
        HeaderValue,
    },
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::net::SocketAddr;
use std::time::Duration;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod db;
mod handlers;
mod models;

/// Clever Brand Kit stylesheet, embedded in the binary (no static-file dependency).
async fn brand_css() -> impl IntoResponse {
    (
        [
            (CONTENT_TYPE, "text/css; charset=utf-8"),
            (CACHE_CONTROL, "public, max-age=86400"),
        ],
        include_str!("../static/cc-brand.css"),
    )
}

/// Control tower styles (radar, strips, console, licence), embedded like the kit.
async fn tower_css() -> impl IntoResponse {
    (
        [
            (CONTENT_TYPE, "text/css; charset=utf-8"),
            (CACHE_CONTROL, "public, max-age=3600"),
        ],
        include_str!("../static/tower.css"),
    )
}

/// Routes and security headers (TLS/HSTS are handled by the Clever Cloud proxy).
/// CSP allows the inline style attributes of the radar, Google Fonts and the
/// data: SVG favicon/masks of the brand kit; no script at all.
fn app(pool: PgPool) -> Router {
    Router::new()
        .route("/", get(handlers::list_incidents))
        .route("/incidents/new", get(handlers::new_incident_form))
        .route("/incidents", post(handlers::create_incident))
        // Axum 0.8 path-parameter syntax: `{id}` (the former `:id` panics at startup)
        .route("/incidents/{id}", get(handlers::incident_detail))
        .route("/incidents/{id}/status", post(handlers::update_status))
        .route("/health", get(handlers::health))
        .route("/stats", get(handlers::stats))
        .route("/cc-brand.css", get(brand_css))
        .route("/tower.css", get(tower_css))
        .layer(TraceLayer::new_for_http())
        .layer(SetResponseHeaderLayer::if_not_present(
            X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "default-src 'self'; \
                 style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; \
                 font-src https://fonts.gstatic.com; \
                 img-src 'self' data:; \
                 frame-ancestors 'none'",
            ),
        ))
        .with_state(pool)
}

#[tokio::main]
async fn main() {
    // Load .env if present (local dev only)
    dotenvy::dotenv().ok();

    // Tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "incident_tracker=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Database — try DATABASE_URL first, then Clever Cloud's POSTGRESQL_ADDON_URI
    let database_url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("POSTGRESQL_ADDON_URI"))
        .expect("DATABASE_URL or POSTGRESQL_ADDON_URI must be set");

    // Pool size: DB_POOL_MAX × instances must stay below the add-on's connection
    // limit (PostgreSQL DEV plan = 5 connections). Default 2 leaves room for a
    // second instance during a redeploy and for an admin psql session.
    let pool_max: u32 = std::env::var("DB_POOL_MAX")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2);

    let pool = PgPoolOptions::new()
        .max_connections(pool_max)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&database_url)
        .await
        .expect("Failed to connect to PostgreSQL");

    // Run migrations automatically at startup
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    tracing::info!("Database ready");

    // PostgreSQL version for the controller console (read once).
    db::load_server_version(&pool).await;

    let app = app(pool.clone());

    // Port: Clever Cloud injects PORT automatically
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("PORT must be a valid number");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Server error");

    // Let in-flight requests finish, then release the PostgreSQL connections.
    pool.close().await;
    tracing::info!("Shutdown complete");
}

/// Resolves on Ctrl+C or SIGTERM (sent by Clever Cloud on redeploy/stop).
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received, draining in-flight requests");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, http::StatusCode};
    use tower::ServiceExt;

    /// A pool that never connects: enough for the routes that do not touch PostgreSQL.
    fn lazy_pool() -> PgPool {
        PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(200))
            .connect_lazy("postgres://nobody@127.0.0.1:1/none")
            .unwrap()
    }

    async fn get(path: &str) -> axum::response::Response {
        app(lazy_pool())
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn tower_css_is_served_with_security_headers() {
        let res = get("/tower.css").await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers()[CONTENT_TYPE], "text/css; charset=utf-8");
        assert_eq!(res.headers()[X_CONTENT_TYPE_OPTIONS], "nosniff");
        assert!(res.headers()[CONTENT_SECURITY_POLICY]
            .to_str()
            .unwrap()
            .starts_with("default-src 'self'"));
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        assert!(std::str::from_utf8(&body)
            .unwrap()
            .contains(".scope__sweep"));
    }

    #[tokio::test]
    async fn brand_kit_is_still_served() {
        assert_eq!(get("/cc-brand.css").await.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn incident_routes_use_axum_08_path_syntax() {
        // `{id}` routes are registered (a `:id` route would panic when building the router)
        // and reject a malformed id before any query.
        assert_eq!(
            get("/incidents/not-a-uuid").await.status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn health_is_503_without_postgresql() {
        assert_eq!(
            get("/health").await.status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }
}
