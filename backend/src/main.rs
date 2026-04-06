mod config;
mod db;
mod models;
mod routes;

use axum::Router;
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    dotenvy::dotenv().ok();
    let cfg = config::Config::from_env()?;
    let pool = db::init(&cfg.database_url).await?;

    let app = Router::new()
        .nest("/", routes::router(pool.clone()))
        .layer(CorsLayer::permissive());

    let addr = format!("0.0.0.0:{}", cfg.port);
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
