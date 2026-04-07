use axum::{extract::{Path, Query, State}, Json};
use serde::Deserialize;
use crate::{
    leetcode::{self, LeetcodeClient},
    models::{Problem, RaceResultWithModel},
    routes::AppState,
};

pub async fn random(State(state): State<AppState>) -> Json<Problem> {
    let client = LeetcodeClient::new();
    match client.fetch_random_problem().await {
        Ok(problem) => {
            let _ = leetcode::cache_problem(&state.pool, &problem).await;
            Json(problem)
        }
        Err(_) => {
            let problem = leetcode::get_random_cached(&state.pool).await
                .expect("no cached problems available");
            Json(problem)
        }
    }
}

#[derive(Deserialize)]
pub struct SearchQuery { pub q: String }

pub async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Json<Vec<Problem>> {
    let results = leetcode::search_problems(&state.pool, &params.q).await
        .unwrap_or_default();
    Json(results)
}

pub async fn results(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<Vec<RaceResultWithModel>> {
    let rows = sqlx::query!(
        r#"SELECT r.id, r.problem_id, r.model_id, r.solved, r.time_ms, r.attempts, r.run_at,
           m.display_name, m.provider, m.name as model_name, m.is_human
           FROM results r JOIN models m ON r.model_id = m.id
           WHERE r.problem_id = $1
           ORDER BY r.time_ms ASC NULLS LAST"#,
        id
    ).fetch_all(&state.pool).await.unwrap_or_default();

    let results: Vec<RaceResultWithModel> = rows.into_iter().map(|r| RaceResultWithModel {
        model_id: r.model_id,
        model_name: r.model_name,
        display_name: r.display_name,
        provider: r.provider,
        is_human: r.is_human,
        solved: r.solved,
        time_ms: r.time_ms,
        attempts: r.attempts,
        run_at: r.run_at,
    }).collect();

    Json(results)
}
