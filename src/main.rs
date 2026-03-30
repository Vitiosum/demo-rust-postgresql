use axum::{
    routing::{get, post},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod db;
mod handlers;
mod models;

#[tokio::main]
async fn main() {
    // Load .env if present (local dev only)
    dotenvy::dotenv().ok();

    // Tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "incident_tracker=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Database
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to PostgreSQL");

    // Run migrations automatically at startup
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    tracing::info!("Database ready");

    // Router
    let app = Router::new()
        .route("/",                         get(handlers::list_incidents))
        .route("/incidents/new",            get(handlers::new_incident_form))
        .route("/incidents",                post(handlers::create_incident))
        .route("/incidents/:id",            get(handlers::incident_detail))
        .route("/incidents/:id/status",     post(handlers::update_status))
        .route("/health",                   get(handlers::health))
        .route("/stats",                    get(handlers::stats))
        .layer(TraceLayer::new_for_http())
        .with_state(pool);

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
        .await
        .expect("Server error");
}
