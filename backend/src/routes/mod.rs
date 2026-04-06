pub mod models;
pub mod problems;
pub mod races;

use axum::{routing::{get, post}, Router};
use sqlx::SqlitePool;
use std::sync::Arc;
use crate::{config::Config, runner::Runner};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<Config>,
    pub runner: Arc<Runner>,
}

pub fn router(pool: SqlitePool, config: Arc<Config>) -> Router {
    let runner = Arc::new(Runner::new(config.clone()));
    let state = AppState { pool, config, runner };

    Router::new()
        .route("/problems/random", get(problems::random))
        .route("/problems/search", get(problems::search))
        .route("/problems/:id/results", get(problems::results))
        .route("/races", post(races::create))
        .route("/races/:id/stream", get(races::stream))
        .route("/models", get(models::list))
        .with_state(state)
}
