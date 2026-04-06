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
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cfg = Arc::new(config::Config::from_env()?);
    let pool = db::init(&cfg.database_url).await?;

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

    let app = Router::new()
        .nest("/", routes::router(pool.clone()))
        .layer(CorsLayer::permissive());

    let addr = format!("0.0.0.0:{}", cfg.port);
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
