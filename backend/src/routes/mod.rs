pub mod execution;
pub mod models;
pub mod problems;
pub mod races;

use axum::{routing::{get, post}, Router};
use sqlx::PgPool;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{config::Config, rate_limit::RateLimiter, runner::Runner};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
    pub runner: Arc<Runner>,
    /// Tracks problem IDs that currently have an on-demand benchmark in flight,
    /// preventing duplicate spawns from concurrent requests.
    pub benchmarks_in_flight: Arc<Mutex<HashSet<String>>>,
    pub rate_limiter: Arc<RateLimiter>,
}

pub fn router(pool: PgPool, config: Arc<Config>) -> Router {
    let runner = Arc::new(Runner::new(config.clone()));
    let benchmarks_in_flight = Arc::new(Mutex::new(HashSet::new()));
    let rate_limiter = Arc::new(RateLimiter::new(10, 60));
    let state = AppState { pool, config, runner, benchmarks_in_flight, rate_limiter };

    Router::new()
        .route("/problems/random", get(problems::random))
        .route("/problems/search", get(problems::search))
        .route("/problems/:id/results", get(problems::results))
        .route("/races", post(races::create))
        .route("/races/:id/stream", get(races::stream))
        .route("/models", get(models::list))
        .route("/leaderboard", get(models::leaderboard))
        .route("/run", post(execution::run_code))
        .route("/submit", post(execution::submit_code))
        .route("/results/:id/details", get(execution::result_details))
        .with_state(state)
}
