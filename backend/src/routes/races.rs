use axum::{
    extract::{Path, State},
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
) -> Json<CreateRaceResponse> {
    let race_id = Uuid::new_v4().to_string();

    let problem = sqlx::query_as!(
        crate::models::Problem,
        r#"SELECT id as "id!", lc_id as "lc_id!", title as "title!", difficulty as "difficulty!",
           description as "description!", starter_code as "starter_code!",
           test_cases as "test_cases!", source as "source!", cached_at as "cached_at!"
           FROM problems WHERE id = ?"#,
        body.problem_id
    ).fetch_one(&state.pool).await.expect("problem not found");

    let models = sqlx::query_as!(
        crate::models::Model,
        r#"SELECT id as "id!", provider as "provider!", name as "name!", display_name as "display_name!",
           api_key_env as "api_key_env!", is_active as "is_active!", is_new as "is_new!",
           is_human as "is_human!", human_times, added_at as "added_at!"
           FROM models WHERE is_active = 1"#
    ).fetch_all(&state.pool).await.unwrap_or_default();

    let (tx, _) = broadcast::channel::<RaceEvent>(64);
    let tx_clone = tx.clone();
    let runner = state.runner.clone();
    let pool = state.pool.clone();
    let race_id_clone = race_id.clone();

    let now = Utc::now().to_rfc3339();
    sqlx::query!(
        "INSERT INTO races (id, problem_id, started_at) VALUES (?, ?, ?)",
        race_id, body.problem_id, now
    ).execute(&state.pool).await.ok();

    tokio::spawn(async move {
        let results = runner.race(&race_id_clone, &problem, models, tx_clone).await;
        for result in &results {
            sqlx::query!(
                "INSERT OR REPLACE INTO results (id, problem_id, model_id, solved, time_ms, attempts, run_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
                result.id, result.problem_id, result.model_id, result.solved,
                result.time_ms, result.attempts, result.run_at
            ).execute(&pool).await.ok();
        }
        let finished = Utc::now().to_rfc3339();
        sqlx::query!("UPDATE races SET finished_at = ? WHERE id = ?", finished, race_id_clone)
            .execute(&pool).await.ok();
    });

    Json(CreateRaceResponse { race_id })
}

pub async fn stream(
    State(_state): State<AppState>,
    Path(_race_id): Path<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = futures::stream::empty::<Result<Event, Infallible>>();
    Sse::new(stream)
}
