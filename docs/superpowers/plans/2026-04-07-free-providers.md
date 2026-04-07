# Free Providers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Groq, GitHub Models, and Cloudflare Workers AI providers to the benchmark leaderboard alongside existing free-tier models (Gemini, DeepSeek, Qwen already seeded), with an on-demand trigger that benchmarks missing models when a user views a problem.

**Architecture:** New provider branches in `runner.rs` handle API format differences. `sync.rs` seed is changed to always upsert (removing the "skip if models exist" guard) so new models appear on existing deployments. `routes/problems.rs` spawns a background Tokio task when active models have no result for the requested problem.

**Tech Stack:** Rust, axum, sqlx (PostgreSQL), tokio, reqwest

---

## File Structure

| File | Change |
|------|--------|
| `backend/src/runner.rs` | Add `groq`, `github`, `cloudflare` to `build_api_request`; add `cloudflare` to `extract_code` |
| `backend/src/config.rs` | Add `groq_api_key`, `github_token`, `cf_api_token`, `cf_account_id` fields |
| `backend/src/sync.rs` | Remove early-return count guard; change seed INSERT to `ON CONFLICT (name) DO NOTHING`; add 5 new model rows |
| `backend/src/routes/problems.rs` | Add on-demand trigger: find missing models, spawn background benchmark |
| `backend/.env.example` | Document 4 new env vars |

---

### Task 1: Add groq, github, cloudflare provider support to runner.rs

**Files:**
- Modify: `backend/src/runner.rs`

**Context:** `build_api_request` is a pure match function returning `(url, body)`. Groq and GitHub Models are OpenAI-compatible (same body format, different base URL). Cloudflare uses a different URL structure (`/accounts/{CF_ACCOUNT_ID}/ai/run/{model}`) and a different response shape (`result.response` instead of `choices[0].message.content`). Auth headers are handled separately by `auth_header_name`/`auth_header_value` and work correctly for all three (all use `Authorization: Bearer <key>`).

- [ ] **Step 1: Write failing tests**

Add at the bottom of `backend/src/runner.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_api_request_groq() {
        let (url, body) = build_api_request("groq", "llama-3.3-70b-versatile", "", "test").unwrap();
        assert_eq!(url, "https://api.groq.com/openai/v1/chat/completions");
        assert_eq!(body["model"], "llama-3.3-70b-versatile");
        assert!(body["messages"].is_array());
    }

    #[test]
    fn test_build_api_request_github() {
        let (url, body) = build_api_request("github", "gpt-4o-mini", "", "test").unwrap();
        assert_eq!(url, "https://models.inference.ai.azure.com/chat/completions");
        assert_eq!(body["model"], "gpt-4o-mini");
    }

    #[test]
    fn test_build_api_request_cloudflare() {
        std::env::set_var("CF_ACCOUNT_ID", "abc123");
        let (url, _) = build_api_request("cloudflare", "@cf/meta/llama-3.1-8b-instruct", "", "test").unwrap();
        assert!(url.contains("abc123"), "URL should contain account ID");
        assert!(url.contains("%40cf") || url.contains("@cf"), "URL should contain model name");
    }

    #[test]
    fn test_build_api_request_unknown_provider_errors() {
        let result = build_api_request("notreal", "model", "", "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_code_cloudflare() {
        let resp = serde_json::json!({
            "result": { "response": "def solution():\n    return 42" },
            "success": true
        });
        let code = extract_code(&resp, "cloudflare").unwrap();
        assert_eq!(code, "def solution():\n    return 42");
    }

    #[test]
    fn test_extract_code_cloudflare_with_code_fence() {
        let resp = serde_json::json!({
            "result": { "response": "```python\ndef solution():\n    return 42\n```" }
        });
        let code = extract_code(&resp, "cloudflare").unwrap();
        assert_eq!(code, "def solution():\n    return 42");
    }

    #[test]
    fn test_extract_code_cloudflare_missing_field() {
        let resp = serde_json::json!({ "result": {} });
        let code = extract_code(&resp, "cloudflare").unwrap();
        assert_eq!(code, "");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /path/to/howfastwould
cargo test --manifest-path backend/Cargo.toml 2>&1 | tail -20
```

Expected: compile error or test failures mentioning "groq", "github", "cloudflare" not matched.

- [ ] **Step 3: Add groq and github to `build_api_request`**

In `backend/src/runner.rs`, find the `build_api_request` function. The first match arm is:
```rust
"openai" | "xai" | "fireworks" | "deepseek" | "mistral" => {
```

Change it to include `groq` and `github` in the same arm (they use the same OpenAI-compatible format):

```rust
"openai" | "xai" | "fireworks" | "deepseek" | "mistral" | "groq" | "github" => {
    let base = match provider {
        "openai" => "https://api.openai.com/v1",
        "xai" => "https://api.x.ai/v1",
        "fireworks" => "https://api.fireworks.ai/inference/v1",
        "deepseek" => "https://api.deepseek.com/v1",
        "mistral" => "https://api.mistral.ai/v1",
        "groq" => "https://api.groq.com/openai/v1",
        "github" => "https://models.inference.ai.azure.com",
        _ => unreachable!(),
    };
    Ok((
        format!("{}/chat/completions", base),
        json!({ "model": model_name, "messages": [{"role":"user","content": prompt}], "max_tokens": 2048 })
    ))
}
```

- [ ] **Step 4: Add cloudflare to `build_api_request`**

After the `"google" => ...` arm (line ~206), add:

```rust
"cloudflare" => {
    let account_id = std::env::var("CF_ACCOUNT_ID").unwrap_or_default();
    Ok((
        format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/ai/run/{}",
            account_id, model_name
        ),
        json!({ "messages": [{"role":"user","content": prompt}], "max_tokens": 2048 })
    ))
},
```

- [ ] **Step 5: Add cloudflare to `extract_code`**

In `extract_code`, add a `"cloudflare"` arm before the default `_` arm:

```rust
fn extract_code(resp: &Value, provider: &str) -> Result<String> {
    let text = match provider {
        "anthropic" => resp["content"][0]["text"].as_str().unwrap_or(""),
        "google" => resp["candidates"][0]["content"]["parts"][0]["text"].as_str().unwrap_or(""),
        "qwen" => resp["output"]["text"].as_str().unwrap_or(""),
        "hunyuan" => resp["Choices"][0]["Message"]["Content"].as_str().unwrap_or(""),
        "cloudflare" => resp["result"]["response"].as_str().unwrap_or(""),
        _ => resp["choices"][0]["message"]["content"].as_str().unwrap_or(""),
    };
    // ... rest of function unchanged
```

- [ ] **Step 6: Run tests to verify they pass**

```bash
cargo test --manifest-path backend/Cargo.toml 2>&1 | tail -20
```

Expected: all new tests pass. Note: `test_build_api_request_cloudflare` sets `CF_ACCOUNT_ID` env var — if tests run in parallel this is fine since each test sets it before asserting.

- [ ] **Step 7: Commit**

```bash
git add backend/src/runner.rs
git commit -m "feat: add groq, github, cloudflare provider support"
```

---

### Task 2: Add config fields for new providers

**Files:**
- Modify: `backend/src/config.rs`

**Context:** `config.rs` holds all API keys loaded from env vars on startup. The `runner.rs` reads API keys at race time via `std::env::var(&model.api_key_env)`, so these config fields aren't strictly required for the runner — but they're needed for the `sync_models` auto-discovery function in `sync.rs` (which checks `if !config.groq_api_key.is_empty()`). Add them now so `sync.rs` can be extended in the future. `CF_ACCOUNT_ID` is used directly in `build_api_request` via `std::env::var`, so it doesn't need a config field — but add it for completeness and testability.

- [ ] **Step 1: Write failing test**

Add to `backend/src/config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_new_fields_default_empty() {
        // Ensure new fields don't panic when env vars are absent
        std::env::remove_var("GROQ_API_KEY");
        std::env::remove_var("GITHUB_TOKEN");
        std::env::remove_var("CF_API_TOKEN");
        std::env::remove_var("CF_ACCOUNT_ID");
        let cfg = Config::from_env().unwrap();
        assert_eq!(cfg.groq_api_key, "");
        assert_eq!(cfg.github_token, "");
        assert_eq!(cfg.cf_api_token, "");
        assert_eq!(cfg.cf_account_id, "");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test --manifest-path backend/Cargo.toml config::tests 2>&1
```

Expected: compile error — fields don't exist yet.

- [ ] **Step 3: Add fields to Config struct**

In `backend/src/config.rs`, add to the `Config` struct after `mistral_api_key`:

```rust
pub groq_api_key: String,
pub github_token: String,
pub cf_api_token: String,
pub cf_account_id: String,
```

- [ ] **Step 4: Add fields to `from_env`**

In `from_env()`, add after `mistral_api_key: std::env::var("MISTRAL_API_KEY").unwrap_or_default(),`:

```rust
groq_api_key: std::env::var("GROQ_API_KEY").unwrap_or_default(),
github_token: std::env::var("GITHUB_TOKEN").unwrap_or_default(),
cf_api_token: std::env::var("CF_API_TOKEN").unwrap_or_default(),
cf_account_id: std::env::var("CF_ACCOUNT_ID").unwrap_or_default(),
```

- [ ] **Step 5: Run test to verify it passes**

```bash
cargo test --manifest-path backend/Cargo.toml config::tests 2>&1
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add backend/src/config.rs
git commit -m "feat: add groq, github, cloudflare fields to Config"
```

---

### Task 3: Fix seed and add new model rows

**Files:**
- Modify: `backend/src/sync.rs`

**Context:** `seed_initial_models` currently short-circuits with `if count > 0 { return Ok(()); }` and uses a plain INSERT with no conflict handling. This means new model rows never get added to existing deployments. Fix: remove the count guard and add `ON CONFLICT (name) DO NOTHING` to the INSERT so it's safe to run on every startup. Then add the 5 new model rows (groq ×2, github ×2, cloudflare ×1). Note: gemini-2.0-flash, deepseek-chat, deepseek-reasoner, and qwen models are already in the seed list.

- [ ] **Step 1: Remove the count guard**

In `backend/src/sync.rs`, find `seed_initial_models`. Remove these lines:

```rust
let count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM models")
    .fetch_one(pool).await?.unwrap_or(0);

if count > 0 { return Ok(()); }
```

- [ ] **Step 2: Add `ON CONFLICT (name) DO NOTHING` to the seed INSERT**

Find the `sqlx::query!` call inside the `for` loop in `seed_initial_models`. Change:

```rust
sqlx::query!(
    "INSERT INTO models (id, provider, name, display_name, api_key_env, is_active, is_new, is_human, human_times, added_at) VALUES ($1, $2, $3, $4, $5, true, false, $6, $7, $8)",
    id, provider, name, display, key_env, is_human, human_times, now
).execute(pool).await?;
```

To:

```rust
sqlx::query!(
    "INSERT INTO models (id, provider, name, display_name, api_key_env, is_active, is_new, is_human, human_times, added_at) VALUES ($1, $2, $3, $4, $5, true, false, $6, $7, $8) ON CONFLICT (name) DO NOTHING",
    id, provider, name, display, key_env, is_human, human_times, now
).execute(pool).await?;
```

- [ ] **Step 3: Add new model rows to the seed list**

Find the `models: &[(&str, &str, &str, &str, bool)]` slice in `seed_initial_models`. Add these 5 rows after the existing entries and before the human rows:

```rust
("groq",       "llama-3.3-70b-versatile",           "Llama 3.3 70B",        "GROQ_API_KEY",  false),
("groq",       "mixtral-8x7b-32768",                 "Mixtral 8x7B",         "GROQ_API_KEY",  false),
("github",     "gpt-4o-mini",                        "GPT-4o mini",          "GITHUB_TOKEN",  false),
("github",     "Meta-Llama-3-70B-Instruct",          "Llama 3 70B",          "GITHUB_TOKEN",  false),
("cloudflare", "@cf/meta/llama-3.1-8b-instruct",     "Llama 3.1 8B",         "CF_API_TOKEN",  false),
```

- [ ] **Step 4: Verify compilation**

```bash
cargo check --manifest-path backend/Cargo.toml 2>&1
```

Expected: no errors. (sqlx compile-time query checks run against `DATABASE_URL` — if not set locally, use `SQLX_OFFLINE=true cargo check ...`)

- [ ] **Step 5: Commit**

```bash
git add backend/src/sync.rs
git commit -m "feat: seed groq, github, cloudflare models; always upsert on startup"
```

---

### Task 4: On-demand benchmark trigger in problems.rs

**Files:**
- Modify: `backend/src/routes/problems.rs`

**Context:** The `results` handler currently fetches and returns race results for a problem. We extend it to: (1) check which active non-human models have no result for this problem, (2) if any are missing, spawn a `tokio::spawn` background task that calls `runner.race()` and saves results. The HTTP response returns immediately with existing results — the frontend's polling loop surfaces new results as they complete. `AppState` already has `runner: Arc<Runner>` and `pool: PgPool`.

- [ ] **Step 1: Write a test for the missing-model detection logic**

The detection logic (finding model IDs with no results) is pure set math. Write a unit test for it:

```rust
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    #[test]
    fn test_missing_model_ids() {
        let benchmarked: HashSet<String> = ["model-a", "model-b"]
            .iter().map(|s| s.to_string()).collect();
        let all_active = vec!["model-a", "model-b", "model-c", "model-d"];
        let missing: Vec<&str> = all_active.into_iter()
            .filter(|id| !benchmarked.contains(*id))
            .collect();
        assert_eq!(missing, vec!["model-c", "model-d"]);
    }

    #[test]
    fn test_no_missing_models_when_all_benchmarked() {
        let benchmarked: HashSet<String> = ["model-a"].iter().map(|s| s.to_string()).collect();
        let all_active = vec!["model-a"];
        let missing: Vec<&str> = all_active.into_iter()
            .filter(|id| !benchmarked.contains(*id))
            .collect();
        assert!(missing.is_empty());
    }
}
```

Add this at the bottom of `backend/src/routes/problems.rs`.

- [ ] **Step 2: Run tests to verify they pass (pure logic, no DB needed)**

```bash
cargo test --manifest-path backend/Cargo.toml routes::problems 2>&1
```

Expected: 2 tests pass.

- [ ] **Step 3: Add imports to problems.rs**

At the top of `backend/src/routes/problems.rs`, add to the existing `use` block:

```rust
use std::collections::HashSet;
use tokio::sync::broadcast;
use uuid::Uuid;
use crate::models::Model;
```

- [ ] **Step 4: Replace the `results` handler with the on-demand trigger version**

Replace the entire `results` function in `backend/src/routes/problems.rs`:

```rust
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

    // On-demand trigger: find active models with no result for this problem
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
        let problem_result = sqlx::query_as!(
            crate::models::Problem,
            "SELECT * FROM problems WHERE id = $1",
            id
        ).fetch_optional(&state.pool).await.unwrap_or(None);

        if let Some(problem) = problem_result {
            let runner = state.runner.clone();
            let pool = state.pool.clone();
            let race_id = Uuid::new_v4().to_string();
            let (tx, _) = broadcast::channel(64);
            tokio::spawn(async move {
                tracing::info!(
                    "on-demand benchmark: {} missing models for problem {}",
                    missing.len(), problem.id
                );
                let results = runner.race(&race_id, &problem, missing, tx).await;
                for result in &results {
                    sqlx::query!(
                        r#"INSERT INTO results (id, problem_id, model_id, solved, time_ms, attempts, run_at)
                           VALUES ($1, $2, $3, $4, $5, $6, $7)
                           ON CONFLICT (id) DO NOTHING"#,
                        result.id, result.problem_id, result.model_id,
                        result.solved, result.time_ms, result.attempts, result.run_at
                    ).execute(&pool).await.ok();
                }
            });
        }
    }

    Json(results)
}
```

- [ ] **Step 5: Verify compilation**

```bash
cargo check --manifest-path backend/Cargo.toml 2>&1
```

Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add backend/src/routes/problems.rs
git commit -m "feat: on-demand benchmark trigger when models are missing results"
```

---

### Task 5: Document new env vars

**Files:**
- Modify: `backend/.env.example`

- [ ] **Step 1: Add new env vars to .env.example**

Open `backend/.env.example` and add after the existing key entries:

```
# Free-tier providers
GROQ_API_KEY=
GITHUB_TOKEN=
CF_API_TOKEN=
CF_ACCOUNT_ID=
```

- [ ] **Step 2: Verify no other changes needed**

```bash
cargo check --manifest-path backend/Cargo.toml 2>&1
```

Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add backend/.env.example
git commit -m "docs: add free provider env vars to .env.example"
```

---

## Post-implementation

After all tasks are complete, set env vars in Railway for any providers you have keys for:

```
GROQ_API_KEY=<from console.groq.com>
GITHUB_TOKEN=<GitHub personal access token with Models permission>
CF_API_TOKEN=<from Cloudflare dashboard>
CF_ACCOUNT_ID=<from Cloudflare dashboard>
```

On next deploy, `seed_initial_models` will insert the 5 new model rows. On first user visit to any problem, the on-demand trigger fires and benchmarks all missing models in the background.
