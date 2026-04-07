use crate::{config::Config, leetcode::LeetcodeClient, models::Model, runner::Runner};
use anyhow::Result;
use reqwest::Client;
use serde_json::Value;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;
use chrono::Utc;

pub async fn sync_models(pool: &PgPool, config: &Config) -> Result<()> {
    let client = Client::new();
    let now = Utc::now().to_rfc3339();

    // OpenAI
    if !config.openai_api_key.is_empty() {
        if let Ok(resp) = client.get("https://api.openai.com/v1/models")
            .bearer_auth(&config.openai_api_key)
            .send().await
        {
            if let Ok(body) = resp.json::<Value>().await {
                if let Some(models) = body["data"].as_array() {
                    for m in models {
                        let name = m["id"].as_str().unwrap_or_default();
                        if name.starts_with("gpt-") || name.starts_with("o1") || name.starts_with("o3") || name.starts_with("o4") {
                            upsert_model(pool, "openai", name, name, "OPENAI_API_KEY", &now).await.ok();
                        }
                    }
                }
            }
        }
    }

    // Anthropic
    if !config.anthropic_api_key.is_empty() {
        if let Ok(resp) = client.get("https://api.anthropic.com/v1/models")
            .header("x-api-key", &config.anthropic_api_key)
            .header("anthropic-version", "2023-06-01")
            .send().await
        {
            if let Ok(body) = resp.json::<Value>().await {
                if let Some(models) = body["data"].as_array() {
                    for m in models {
                        let name = m["id"].as_str().unwrap_or_default();
                        upsert_model(pool, "anthropic", name, name, "ANTHROPIC_API_KEY", &now).await.ok();
                    }
                }
            }
        }
    }

    // DeepSeek
    if !config.deepseek_api_key.is_empty() {
        if let Ok(resp) = client.get("https://api.deepseek.com/v1/models")
            .bearer_auth(&config.deepseek_api_key)
            .send().await
        {
            if let Ok(body) = resp.json::<Value>().await {
                if let Some(models) = body["data"].as_array() {
                    for m in models {
                        let name = m["id"].as_str().unwrap_or_default();
                        upsert_model(pool, "deepseek", name, &format!("🐉 {}", name), "DEEPSEEK_API_KEY", &now).await.ok();
                    }
                }
            }
        }
    }

    tracing::info!("model sync complete");
    Ok(())
}

async fn upsert_model(
    pool: &PgPool,
    provider: &str,
    name: &str,
    display_name: &str,
    api_key_env: &str,
    now: &str,
) -> Result<()> {
    let id = Uuid::new_v4().to_string();
    let result = sqlx::query!(
        "INSERT INTO models (id, provider, name, display_name, api_key_env, is_active, is_new, is_human, added_at) VALUES ($1, $2, $3, $4, $5, true, true, false, $6) ON CONFLICT (name) DO NOTHING",
        id, provider, name, display_name, api_key_env, now
    ).execute(pool).await?;
    if result.rows_affected() > 0 {
        tracing::info!("new model discovered: {} ({})", name, provider);
    }
    Ok(())
}

pub async fn seed_initial_models(pool: &PgPool) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let models: &[(&str, &str, &str, &str, bool)] = &[
        ("openai", "gpt-4.5", "GPT-4.5", "OPENAI_API_KEY", false),
        ("openai", "o3", "o3", "OPENAI_API_KEY", false),
        ("openai", "o4-mini", "o4-mini", "OPENAI_API_KEY", false),
        ("anthropic", "claude-opus-4-6", "Claude Opus 4.6", "ANTHROPIC_API_KEY", false),
        ("anthropic", "claude-sonnet-4-6", "Claude Sonnet 4.6", "ANTHROPIC_API_KEY", false),
        ("google", "gemini-2.5-pro", "Gemini 2.5 Pro", "GOOGLE_API_KEY", false),
        ("google", "gemini-2.0-flash", "Gemini 2.0 Flash", "GOOGLE_API_KEY", false),
        ("xai", "grok-3", "Grok 3", "XAI_API_KEY", false),
        ("fireworks", "accounts/meta/models/llama-4", "Llama 4", "FIREWORKS_API_KEY", false),
        ("mistral", "mistral-large-latest", "Mistral Large 2", "MISTRAL_API_KEY", false),
        ("deepseek", "deepseek-chat", "🐉 DeepSeek V3", "DEEPSEEK_API_KEY", false),
        ("deepseek", "deepseek-reasoner", "🐉 DeepSeek R2", "DEEPSEEK_API_KEY", false),
        ("qwen", "qwen2.5-coder-32b-instruct", "🐉 Qwen 2.5 Coder", "QWEN_API_KEY", false),
        ("qwen", "qwq-32b", "🐉 QwQ-32B", "QWEN_API_KEY", false),
        ("moonshot", "moonshot-v1-8k", "🐉 Kimi k1.5", "MOONSHOT_API_KEY", false),
        ("doubao", "doubao-pro-32k", "🐉 Doubao", "DOUBAO_API_KEY", false),
        ("hunyuan", "hunyuan-standard", "🐉 Hunyuan", "HUNYUAN_API_KEY", false),
        ("groq", "llama-3.3-70b-versatile", "Llama 3.3 70B", "GROQ_API_KEY", false),
        ("groq", "mixtral-8x7b-32768", "Mixtral 8x7B", "GROQ_API_KEY", false),
        ("github", "gpt-4o-mini", "GPT-4o mini", "GITHUB_TOKEN", false),
        ("github", "Meta-Llama-3-70B-Instruct", "Llama 3 70B", "GITHUB_TOKEN", false),
        ("cloudflare", "@cf/meta/llama-3.1-8b-instruct", "Llama 3.1 8B", "CF_API_TOKEN", false),
        ("human", "lc-avg", "👤 LeetCode Avg", "", true),
        ("human", "neetcode", "👤 NeetCode", "", true),
        ("human", "tourist", "👤 Tourist", "", true),
    ];

    for (provider, name, display, key_env, is_human) in models {
        let id = Uuid::new_v4().to_string();
        let human_times = if *is_human {
            match *name {
                "lc-avg" => Some(r#"{"Easy":900000,"Medium":2700000,"Hard":7200000}"#),
                "neetcode" => Some(r#"{"Easy":120000,"Medium":600000,"Hard":1800000}"#),
                "tourist" => Some(r#"{"Easy":60000,"Medium":180000,"Hard":600000}"#),
                _ => None,
            }
        } else { None };

        sqlx::query!(
            "INSERT INTO models (id, provider, name, display_name, api_key_env, is_active, is_new, is_human, human_times, added_at) VALUES ($1, $2, $3, $4, $5, true, false, $6, $7, $8) ON CONFLICT (name) DO NOTHING",
            id, provider, name, display, key_env, is_human, human_times, now
        ).execute(pool).await?;
    }

    tracing::info!("seeded {} models", models.len());
    Ok(())
}

pub async fn run_benchmark_batch(pool: &PgPool, config: Arc<Config>) -> Result<()> {
    let lc = LeetcodeClient::new();
    let runner = Runner::new(config);
    let (tx, _) = broadcast::channel(64);

    for _ in 0..3 {
        let problem = match lc.fetch_random_problem().await {
            Ok(p) => { crate::leetcode::cache_problem(pool, &p).await.ok(); p }
            Err(_) => match crate::leetcode::get_random_cached(pool).await {
                Ok(p) => p,
                Err(_) => continue,
            }
        };

        let models = sqlx::query_as!(Model,
            "SELECT * FROM models WHERE is_active = true"
        ).fetch_all(pool).await.unwrap_or_default();

        let race_id = Uuid::new_v4().to_string();
        let results = runner.race(&race_id, &problem, models, tx.clone()).await;

        for result in &results {
            sqlx::query!(
                r#"INSERT INTO results (id, problem_id, model_id, solved, time_ms, attempts, run_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7)
                   ON CONFLICT (id) DO UPDATE SET
                   problem_id=$2, model_id=$3, solved=$4, time_ms=$5, attempts=$6, run_at=$7"#,
                result.id, result.problem_id, result.model_id, result.solved,
                result.time_ms, result.attempts, result.run_at
            ).execute(pool).await.ok();
        }

        tracing::info!("batch benchmark done for: {}", problem.title);
    }
    Ok(())
}
