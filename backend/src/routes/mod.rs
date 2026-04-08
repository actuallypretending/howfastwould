pub mod admin;
pub mod models;
pub mod problems;
pub mod races;

use axum::{routing::{get, post}, Router};
use sqlx::PgPool;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{config::Config, runner::Runner};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
    pub runner: Arc<Runner>,
    /// Tracks problem IDs that currently have an on-demand benchmark in flight,
    /// preventing duplicate spawns from concurrent requests.
    pub benchmarks_in_flight: Arc<Mutex<HashSet<String>>>,
}

pub fn router(pool: PgPool, config: Arc<Config>) -> Router {
    let runner = Arc::new(Runner::new(config.clone()));
    let benchmarks_in_flight = Arc::new(Mutex::new(HashSet::new()));
    let state = AppState { pool, config, runner, benchmarks_in_flight };

    Router::new()
        .route("/problems/random", get(problems::random))
        .route("/problems/search", get(problems::search))
        .route("/problems/:id/results", get(problems::results))
        .route("/races", post(races::create))
        .route("/races/:id/stream", get(races::stream))
        .route("/models", get(models::list))
        .route("/leaderboard", get(models::leaderboard))
        .route("/admin/seed-benchmarks", post(admin::seed_benchmarks))
        .with_state(state)
}
