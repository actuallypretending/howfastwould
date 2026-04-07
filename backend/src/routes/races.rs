use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::sse::{Event, Sse},
    Json,
};
use futures::stream::Stream;
use serde::Deserialize;
use std::convert::Infallible;
use tokio::sync::broadcast;
use uuid::Uuid;
use chrono::Utc;
use crate::{models::RaceEvent, routes::AppState};

#[derive(Deserialize)]
pub struct CreateRaceBody {
    pub problem_id: String,
}

#[derive(serde::Serialize)]
pub struct CreateRaceResponse {
    pub race_id: String,
}

pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<CreateRaceBody>,
) -> Result<Json<CreateRaceResponse>, StatusCode> {
    let race_id = Uuid::new_v4().to_string();

    let problem = sqlx::query_as!(
        crate::models::Problem,
        "SELECT * FROM problems WHERE id = $1",
        body.problem_id
    ).fetch_optional(&state.pool).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let models = sqlx::query_as!(
        crate::models::Model,
        "SELECT * FROM models WHERE is_active = true"
    ).fetch_all(&state.pool).await.unwrap_or_default();

    let (tx, _) = broadcast::channel::<RaceEvent>(64);
    let tx_clone = tx.clone();
    let runner = state.runner.clone();
    let pool = state.pool.clone();
    let race_id_clone = race_id.clone();

    let now = Utc::now().to_rfc3339();
    sqlx::query!(
        "INSERT INTO races (id, problem_id, started_at) VALUES ($1, $2, $3)",
        race_id, body.problem_id, now
    ).execute(&state.pool).await.ok();

    tokio::spawn(async move {
        let results = runner.race(&race_id_clone, &problem, models, tx_clone).await;
        for result in &results {
            sqlx::query!(
                r#"INSERT INTO results (id, problem_id, model_id, solved, time_ms, attempts, run_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7)
                   ON CONFLICT (id) DO UPDATE SET
                   problem_id=$2, model_id=$3, solved=$4, time_ms=$5, attempts=$6, run_at=$7"#,
                result.id, result.problem_id, result.model_id, result.solved,
                result.time_ms, result.attempts, result.run_at
            ).execute(&pool).await.ok();
        }
        let finished = Utc::now().to_rfc3339();
        sqlx::query!(
            "UPDATE races SET finished_at = $1 WHERE id = $2",
            finished, race_id_clone
        ).execute(&pool).await.ok();
    });

    Ok(Json(CreateRaceResponse { race_id }))
}

pub async fn stream(
    State(_state): State<AppState>,
    Path(_race_id): Path<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = futures::stream::empty::<Result<Event, Infallible>>();
    Sse::new(stream)
}
