mod config;
mod db;
mod leetcode;
mod models;
mod piston;
mod roast;
mod routes;
mod runner;
mod sync;

use axum::Router;
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cfg = Arc::new(config::Config::from_env()?);
    tracing::info!("connecting to database");
    let pool = db::init(&cfg.database_url).await.map_err(|e| {
        tracing::error!("failed to connect to database: {}", e);
        e
    })?;

    sync::seed_initial_models(&pool).await?;

    {
        let pool = pool.clone();
        let cfg = cfg.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(86400));
            loop {
                interval.tick().await;
                sync::sync_models(&pool, &cfg).await.ok();
            }
        });
    }
    {
        let pool = pool.clone();
        let cfg = cfg.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(21600));
            loop {
                interval.tick().await;
                sync::run_benchmark_batch(&pool, cfg.clone()).await.ok();
            }
        });
    }

    // Build a CORS layer that is restricted to configured origins in production.
    // If no ALLOWED_ORIGINS env var is set we fall back to permissive (dev convenience only).
    let cors = if cfg.allowed_origins.is_empty() {
        tracing::warn!("ALLOWED_ORIGINS not set — using permissive CORS (dev mode)");
        CorsLayer::permissive()
    } else {
        let origins: Vec<axum::http::HeaderValue> = cfg
            .allowed_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        tracing::info!("CORS restricted to: {:?}", cfg.allowed_origins);
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::OPTIONS,
            ])
            .allow_headers([axum::http::header::CONTENT_TYPE])
    };

    let app = Router::new()
        .nest("/", routes::router(pool.clone(), cfg.clone()))
        .layer(cors);

    let addr = format!("0.0.0.0:{}", cfg.port);
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
