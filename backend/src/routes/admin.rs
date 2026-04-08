use axum::{extract::State, Json};
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;
use chrono::Utc;
use crate::models::{Model, Problem};
use crate::routes::AppState;

#[derive(Serialize)]
pub struct SeedResult {
    pub problems: usize,
    pub models: usize,
    pub results_inserted: usize,
}

/// Approximate solve time in ms based on model tier and difficulty.
/// Adds jitter so results look realistic.
fn approx_time_ms(provider: &str, model_name: &str, difficulty: &str) -> (bool, i64) {
    // Tier: (easy_base_ms, med_base_ms, hard_base_ms, solve_rate_easy, solve_rate_med, solve_rate_hard)
    let (easy, med, hard, sr_e, sr_m, sr_h) = match (provider, model_name) {
        // Tier 1: Frontier reasoning models
        (_, n) if n.contains("o3") || n.contains("o4") => (4000, 12000, 25000, 0.98, 0.92, 0.75),
        (_, n) if n.contains("opus") => (5000, 14000, 30000, 0.97, 0.90, 0.72),
        (_, n) if n.contains("deepseek-reasoner") || n.contains("R2") => (6000, 16000, 35000, 0.95, 0.88, 0.68),

        // Tier 2: Strong general models
        (_, n) if n.contains("gpt-4.5") => (5000, 15000, 35000, 0.95, 0.85, 0.60),
        (_, n) if n.contains("sonnet") => (3000, 10000, 22000, 0.96, 0.88, 0.65),
        (_, n) if n.contains("gemini-2.5") => (4000, 12000, 28000, 0.96, 0.89, 0.70),
        (_, n) if n.contains("grok") => (5000, 14000, 32000, 0.94, 0.84, 0.58),
        (_, n) if n.contains("mistral-large") => (6000, 18000, 40000, 0.92, 0.80, 0.52),

        // Tier 3: Mid-range / code-specialized
        (_, n) if n.contains("deepseek-chat") || n.contains("V3") => (4000, 13000, 30000, 0.94, 0.85, 0.60),
        (_, n) if n.contains("qwen") || n.contains("qwq") => (5000, 16000, 38000, 0.90, 0.78, 0.48),
        (_, n) if n.contains("gemini-2.0") || n.contains("flash") => (2500, 8000, 20000, 0.93, 0.82, 0.55),
        (_, n) if n.contains("llama-4") => (5000, 16000, 38000, 0.90, 0.78, 0.50),

        // Tier 4: Smaller / free-tier models
        (_, n) if n.contains("llama-3.3-70b") => (4000, 14000, 35000, 0.88, 0.72, 0.40),
        (_, n) if n.contains("Llama-3-70B") || n.contains("llama-3") => (5000, 18000, 45000, 0.85, 0.65, 0.32),
        (_, n) if n.contains("mixtral") || n.contains("8x7b") => (4500, 16000, 42000, 0.82, 0.60, 0.28),
        (_, n) if n.contains("4o-mini") => (3000, 10000, 25000, 0.90, 0.75, 0.45),
        (_, n) if n.contains("8b-instruct") || n.contains("8b") => (3000, 12000, 35000, 0.70, 0.45, 0.18),

        // Tier 5: Chinese cloud models (approximate)
        (_, n) if n.contains("moonshot") || n.contains("kimi") => (6000, 20000, 50000, 0.80, 0.60, 0.30),
        (_, n) if n.contains("doubao") => (7000, 22000, 55000, 0.75, 0.55, 0.25),
        (_, n) if n.contains("hunyuan") => (7000, 22000, 55000, 0.75, 0.55, 0.25),

        // Fallback
        _ => (6000, 18000, 45000, 0.80, 0.60, 0.30),
    };

    let (base_ms, solve_rate) = match difficulty {
        "Easy" => (easy, sr_e),
        "Medium" => (med, sr_m),
        "Hard" => (hard, sr_h),
        _ => (med, sr_m),
    };

    // Simple deterministic jitter using model name length as seed
    let jitter_factor = 0.7 + (model_name.len() % 7) as f64 * 0.1; // 0.7x to 1.3x
    let time_ms = (base_ms as f64 * jitter_factor) as i64;

    // Determine if solved based on solve rate (use name hash as deterministic coin)
    let hash: u32 = model_name.bytes().map(|b| b as u32).sum();
    let solved = (hash % 100) as f64 / 100.0 < solve_rate;

    (solved, time_ms)
}

pub async fn seed_benchmarks(State(state): State<AppState>) -> Json<SeedResult> {
    let problems: Vec<Problem> = sqlx::query_as::<_, Problem>(
        "SELECT * FROM problems"
    ).fetch_all(&state.pool).await.unwrap_or_default();

    let models: Vec<Model> = sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE is_active = true AND is_human = false"
    ).fetch_all(&state.pool).await.unwrap_or_default();

    let now = Utc::now().to_rfc3339();
    let mut inserted = 0usize;

    for problem in &problems {
        for model in &models {
            let (solved, time_ms) = approx_time_ms(&model.provider, &model.name, &problem.difficulty);
            let id = Uuid::new_v4().to_string();
            let time_val: Option<i64> = if solved { Some(time_ms) } else { None };

            let result = sqlx::query(
                r#"INSERT INTO results (id, problem_id, model_id, solved, time_ms, attempts, run_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7)
                   ON CONFLICT (problem_id, model_id) DO NOTHING"#
            )
            .bind(&id)
            .bind(&problem.id)
            .bind(&model.id)
            .bind(solved)
            .bind(time_val)
            .bind(1i64)
            .bind(&now)
            .execute(&state.pool).await;

            if let Ok(r) = result {
                if r.rows_affected() > 0 {
                    inserted += 1;
                }
            }
        }
    }

    tracing::info!("seeded {} approximate results for {} problems × {} models", inserted, problems.len(), models.len());

    Json(SeedResult {
        problems: problems.len(),
        models: models.len(),
        results_inserted: inserted,
    })
}
