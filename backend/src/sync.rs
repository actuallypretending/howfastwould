use crate::{config::Config, leetcode::LeetcodeClient, models::Model, runner::Runner};
use anyhow::Result;
use reqwest::Client;
use serde_json::Value;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;
use chrono::Utc;

pub async fn sync_models(pool: &SqlitePool, config: &Config) -> Result<()> {
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
                        upsert_model(pool, "deepseek", name, &format!("\u{1F409} {}", name), "DEEPSEEK_API_KEY", &now).await.ok();
                    }
                }
            }
        }
    }

    tracing::info!("model sync complete");
    Ok(())
}

async fn upsert_model(
    pool: &SqlitePool,
    provider: &str,
    name: &str,
    display_name: &str,
    api_key_env: &str,
    now: &str,
) -> Result<()> {
    let existing = sqlx::query!("SELECT id FROM models WHERE name = ?", name)
        .fetch_optional(pool).await?;

    if existing.is_none() {
        let id = Uuid::new_v4().to_string();
        sqlx::query!(
            "INSERT INTO models (id, provider, name, display_name, api_key_env, is_active, is_new, is_human, added_at) VALUES (?, ?, ?, ?, ?, 1, 1, 0, ?)",
            id, provider, name, display_name, api_key_env, now
        ).execute(pool).await?;
        tracing::info!("new model discovered: {} ({})", name, provider);
    }
    Ok(())
}

pub async fn seed_initial_models(pool: &SqlitePool) -> Result<()> {
    let count: i32 = sqlx::query_scalar!("SELECT COUNT(*) FROM models")
        .fetch_one(pool).await?;

    if count > 0 { return Ok(()); }

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
        ("deepseek", "deepseek-chat", "\u{1F409} DeepSeek V3", "DEEPSEEK_API_KEY", false),
        ("deepseek", "deepseek-reasoner", "\u{1F409} DeepSeek R2", "DEEPSEEK_API_KEY", false),
        ("qwen", "qwen2.5-coder-32b-instruct", "\u{1F409} Qwen 2.5 Coder", "QWEN_API_KEY", false),
        ("qwen", "qwq-32b", "\u{1F409} QwQ-32B", "QWEN_API_KEY", false),
        ("moonshot", "moonshot-v1-8k", "\u{1F409} Kimi k1.5", "MOONSHOT_API_KEY", false),
        ("doubao", "doubao-pro-32k", "\u{1F409} Doubao", "DOUBAO_API_KEY", false),
        ("hunyuan", "hunyuan-standard", "\u{1F409} Hunyuan", "HUNYUAN_API_KEY", false),
        ("human", "lc-avg", "\u{1F464} LeetCode Avg", "", true),
        ("human", "neetcode", "\u{1F464} NeetCode", "", true),
        ("human", "tourist", "\u{1F464} Tourist", "", true),
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
            "INSERT INTO models (id, provider, name, display_name, api_key_env, is_active, is_new, is_human, human_times, added_at) VALUES (?, ?, ?, ?, ?, 1, 0, ?, ?, ?)",
            id, provider, name, display, key_env, is_human, human_times, now
        ).execute(pool).await?;
    }

    tracing::info!("seeded {} models", models.len());
    Ok(())
}

pub async fn run_benchmark_batch(pool: &SqlitePool, config: Arc<Config>) -> Result<()> {
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
            r#"SELECT id as "id!", provider as "provider!", name as "name!", display_name as "display_name!",
               api_key_env as "api_key_env!", is_active as "is_active!", is_new as "is_new!",
               is_human as "is_human!", human_times, added_at as "added_at!"
               FROM models WHERE is_active = 1"#
        ).fetch_all(pool).await.unwrap_or_default();

        let race_id = Uuid::new_v4().to_string();
        let results = runner.race(&race_id, &problem, models, tx.clone()).await;

        for result in &results {
            sqlx::query!(
                "INSERT OR REPLACE INTO results (id, problem_id, model_id, solved, time_ms, attempts, run_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
                result.id, result.problem_id, result.model_id, result.solved,
                result.time_ms, result.attempts, result.run_at
            ).execute(pool).await.ok();
        }

        tracing::info!("batch benchmark done for: {}", problem.title);
    }
    Ok(())
}
