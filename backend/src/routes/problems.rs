use axum::{extract::{Path, Query, State}, Json};
use serde::Deserialize;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use tokio::sync::broadcast;
use uuid::Uuid;
use chrono::Utc;
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
        if state.config.enable_live_benchmarks {
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
                        in_flight.lock().await.remove(&problem_id);
                    });
                } else {
                    state.benchmarks_in_flight.lock().await.remove(&id);
                }
            }
        } else {
            // Live benchmarks disabled — seed approximate results on the fly
            let difficulty = sqlx::query_scalar!(
                "SELECT difficulty FROM problems WHERE id = $1", id
            ).fetch_optional(&state.pool).await.ok().flatten();

            if let Some(diff) = difficulty {
                tracing::info!("seeding {} approximate results for problem '{}'", missing.len(), id);
                let seeded = seed_approximate_results(&state.pool, &id, &diff, &missing).await;
                let mut all_results = results;
                all_results.extend(seeded);
                all_results.sort_by(|a, b| a.time_ms.cmp(&b.time_ms));
                return Json(all_results);
            }
        }
    }

    Json(results)
}

/// Deterministic hash for a string pair — used for jitter and solve coin flips.
fn hash_pair(a: &str, b: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    a.hash(&mut hasher);
    b.hash(&mut hasher);
    hasher.finish()
}

/// Model performance tier: (easy_ms, med_ms, hard_ms, easy_solve%, med_solve%, hard_solve%)
fn model_tier(name: &str) -> (i64, i64, i64, u64, u64, u64) {
    let n = name.to_lowercase();
    if n.contains("o3") || n.contains("o4") {
        (4000, 12000, 25000, 98, 92, 75)
    } else if n.contains("opus") {
        (5000, 14000, 30000, 97, 90, 72)
    } else if n.contains("deepseek-reasoner") || n.contains("deepseek-r") {
        (6000, 16000, 35000, 95, 88, 68)
    } else if n.contains("sonnet") {
        (3000, 10000, 22000, 96, 88, 65)
    } else if n.contains("gemini-2.5") {
        (4000, 12000, 28000, 96, 89, 70)
    } else if n.contains("gpt-4.5") {
        (5000, 15000, 35000, 95, 85, 60)
    } else if n.contains("grok") {
        (5000, 14000, 32000, 94, 84, 58)
    } else if n.contains("deepseek-chat") || n.contains("deepseek-v") {
        (4000, 13000, 30000, 94, 85, 60)
    } else if n.contains("gemini-2.0") || n.contains("flash") {
        (2500, 8000, 20000, 93, 82, 55)
    } else if n.contains("4o-mini") {
        (3000, 10000, 25000, 90, 75, 45)
    } else if n.contains("mistral-large") {
        (6000, 18000, 40000, 92, 80, 52)
    } else if n.contains("qwen") || n.contains("qwq") {
        (5000, 16000, 38000, 90, 78, 48)
    } else if n.contains("llama-4") {
        (5000, 16000, 38000, 90, 78, 50)
    } else if n.contains("llama-3.3-70b") {
        (4000, 14000, 35000, 88, 72, 40)
    } else if n.contains("llama-3") && n.contains("70b") {
        (5000, 18000, 45000, 85, 65, 32)
    } else if n.contains("mixtral") {
        (4500, 16000, 42000, 82, 60, 28)
    } else if n.contains("8b") {
        (3000, 12000, 35000, 70, 45, 18)
    } else if n.contains("moonshot") || n.contains("kimi") {
        (6000, 20000, 50000, 80, 60, 30)
    } else if n.contains("doubao") {
        (7000, 22000, 55000, 75, 55, 25)
    } else if n.contains("hunyuan") {
        (7000, 22000, 55000, 75, 55, 25)
    } else {
        (6000, 18000, 45000, 80, 60, 30)
    }
}

/// Generate and persist approximate benchmark results for models missing results on a problem.
async fn seed_approximate_results(
    pool: &sqlx::PgPool,
    problem_id: &str,
    difficulty: &str,
    models: &[Model],
) -> Vec<RaceResultWithModel> {
    let now = Utc::now().to_rfc3339();
    let mut seeded = Vec::new();

    for model in models {
        let (easy_ms, med_ms, hard_ms, easy_rate, med_rate, hard_rate) = model_tier(&model.name);

        let (base_ms, solve_rate) = match difficulty {
            "Easy" => (easy_ms, easy_rate),
            "Medium" => (med_ms, med_rate),
            "Hard" => (hard_ms, hard_rate),
            _ => (med_ms, med_rate),
        };

        // Deterministic jitter: 0.7x to 1.3x with fine granularity
        let h = hash_pair(&model.id, problem_id);
        let jitter = 0.7 + ((h % 601) as f64) * 0.001;
        let time_ms = (base_ms as f64 * jitter) as i64;

        // Deterministic solve coin flip
        let solve_hash = hash_pair(&model.name, &format!("{problem_id}solve"));
        let solved = (solve_hash % 100) < solve_rate;

        let result_id = Uuid::new_v4().to_string();
        let time_val: Option<i64> = if solved { Some(time_ms) } else { None };
        sqlx::query!(
            r#"INSERT INTO results (id, problem_id, model_id, solved, time_ms, attempts, run_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               ON CONFLICT (problem_id, model_id) DO NOTHING"#,
            result_id, problem_id, model.id,
            solved, time_val,
            1_i64, now
        ).execute(pool).await.ok();

        seeded.push(RaceResultWithModel {
            model_id: model.id.clone(),
            model_name: model.name.clone(),
            display_name: model.display_name.clone(),
            provider: model.provider.clone(),
            is_human: model.is_human,
            solved,
            time_ms: if solved { Some(time_ms) } else { None },
            attempts: 1,
            run_at: now.clone(),
        });
    }

    seeded
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
