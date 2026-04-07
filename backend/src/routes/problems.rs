use axum::{extract::{Path, Query, State}, Json};
use serde::Deserialize;
use std::collections::HashSet;
use tokio::sync::broadcast;
use uuid::Uuid;
use crate::{
    leetcode::{self, LeetcodeClient},
    models::{Model, Problem, RaceResultWithModel},
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

    // Find active models with no result for this problem
    let benchmarked_ids: HashSet<String> = results.iter()
        .filter(|r| !r.is_human)
        .map(|r| r.model_id.clone())
        .collect();

    let all_active = sqlx::query_as!(
        Model,
        "SELECT * FROM models WHERE is_active = true AND is_human = false"
    ).fetch_all(&state.pool).await.unwrap_or_default();

    let missing: Vec<Model> = all_active.into_iter()
        .filter(|m| !benchmarked_ids.contains(&m.id))
        .collect();

    if !missing.is_empty() {
        // Prevent duplicate spawns: only trigger if no benchmark is already in flight
        // for this problem.
        let already_running = {
            let mut in_flight = state.benchmarks_in_flight.lock().await;
            !in_flight.insert(id.clone())
        };

        if !already_running {
            let problem_opt = sqlx::query_as!(
                crate::models::Problem,
                "SELECT * FROM problems WHERE id = $1",
                id
            ).fetch_optional(&state.pool).await.unwrap_or(None);

            if let Some(problem) = problem_opt {
                let runner = state.runner.clone();
                let pool = state.pool.clone();
                let in_flight = state.benchmarks_in_flight.clone();
                let problem_id = id.clone();
                let race_id = Uuid::new_v4().to_string();
                let (tx, _) = broadcast::channel(64);
                tokio::spawn(async move {
                    tracing::info!(
                        "on-demand: benchmarking {} missing models for '{}'",
                        missing.len(),
                        problem.title
                    );
                    let bench_results = runner.race(&race_id, &problem, missing, tx).await;
                    for result in &bench_results {
                        sqlx::query!(
                            r#"INSERT INTO results (id, problem_id, model_id, solved, time_ms, attempts, run_at)
                           VALUES ($1, $2, $3, $4, $5, $6, $7)
                           ON CONFLICT (problem_id, model_id) DO UPDATE SET
                               solved = EXCLUDED.solved,
                               time_ms = EXCLUDED.time_ms,
                               attempts = EXCLUDED.attempts,
                               run_at = EXCLUDED.run_at"#,
                            result.id, result.problem_id, result.model_id,
                            result.solved, result.time_ms, result.attempts, result.run_at
                        ).execute(&pool).await.ok();
                    }
                    // Remove from in-flight set so future requests can re-trigger
                    // if new models are added later.
                    in_flight.lock().await.remove(&problem_id);
                });
            } else {
                // Problem not found; remove from in-flight set
                state.benchmarks_in_flight.lock().await.remove(&id);
            }
        }
    }

    Json(results)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    #[test]
    fn test_missing_model_detection_finds_unbenchmarked() {
        let benchmarked: HashSet<String> = ["model-a", "model-b"]
            .iter().map(|s| s.to_string()).collect();
        let all_active = vec!["model-a".to_string(), "model-b".to_string(), "model-c".to_string()];
        let missing: Vec<String> = all_active.into_iter()
            .filter(|id| !benchmarked.contains(id))
            .collect();
        assert_eq!(missing, vec!["model-c"]);
    }

    #[test]
    fn test_missing_model_detection_empty_when_all_benchmarked() {
        let benchmarked: HashSet<String> = ["model-a"].iter().map(|s| s.to_string()).collect();
        let all_active = vec!["model-a".to_string()];
        let missing: Vec<String> = all_active.into_iter()
            .filter(|id| !benchmarked.contains(id))
            .collect();
        assert!(missing.is_empty());
    }
}
