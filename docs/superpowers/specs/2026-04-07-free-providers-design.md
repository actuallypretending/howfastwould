# Free Providers & Competitive Benchmarking Design

## Overview

Add free-tier and near-free AI providers to the benchmark leaderboard. All models compete on the same leaderboard. Results are pre-benchmarked and stored; no live API calls per user visit.

---

## Providers & Models

| Provider | `provider` key | Models | New env vars |
|----------|---------------|--------|--------------|
| Groq | `groq` | `llama-3.3-70b-versatile`, `mixtral-8x7b-32768` | `GROQ_API_KEY` |
| GitHub Models | `github` | `gpt-4o-mini`, `Meta-Llama-3-70B-Instruct` | `GITHUB_TOKEN` |
| Cloudflare Workers AI | `cloudflare` | `@cf/meta/llama-3.1-8b-instruct` | `CF_API_TOKEN`, `CF_ACCOUNT_ID` |
| Google AI Studio | `google` (existing) | `gemini-2.0-flash` | `GEMINI_API_KEY` |
| DeepSeek | `deepseek` (existing) | `deepseek-chat`, `deepseek-reasoner` | `DEEPSEEK_API_KEY` |
| Qwen | `qwen` (existing) | `qwen2.5-72b-instruct` | `DASHSCOPE_API_KEY` |

`groq` and `github` use OpenAI-compatible format. `cloudflare` has a custom URL structure and response shape. `google`, `deepseek`, and `qwen` provider code already exists in `runner.rs` — only DB seed rows and env vars needed.

---

## Architecture

### On-demand benchmarking

`GET /problems/:id/results` gains a post-query check: if any active, non-human model has no result for the problem, a single `tokio::spawn` task benchmarks all missing models concurrently (reusing the existing parallel runner pattern). The HTTP response returns immediately with existing results. The frontend's existing polling loop surfaces new results as they complete.

### Nightly re-benchmark

`main.rs` spawns a background Tokio task on startup using `tokio::time::interval(24h)`. Each tick queries for the 20 stalest problem/model result pairs older than 7 days and re-benchmarks them. This keeps results fresh without hammering free-tier rate limits.

---

## Code Changes

### `backend/src/runner.rs`

Add to `build_api_request`:

```rust
"groq" => Ok((
    "https://api.groq.com/openai/v1/chat/completions".into(),
    json!({ "model": model_name, "messages": [{"role":"user","content": prompt}], "max_tokens": 2048 })
)),
"github" => Ok((
    "https://models.inference.ai.azure.com/chat/completions".into(),
    json!({ "model": model_name, "messages": [{"role":"user","content": prompt}], "max_tokens": 2048 })
)),
"cloudflare" => {
    let account_id = std::env::var("CF_ACCOUNT_ID").unwrap_or_default();
    Ok((
        format!("https://api.cloudflare.com/client/v4/accounts/{}/ai/run/{}", account_id, model_name),
        json!({ "messages": [{"role":"user","content": prompt}], "max_tokens": 2048 })
    ))
},
```

Add to `extract_code`:
```rust
"cloudflare" => resp["result"]["response"].as_str().unwrap_or(""),
```

`groq` and `github` use the default `choices[0].message.content` path — no new `extract_code` branch needed.

### `backend/src/config.rs`

Add fields:
```rust
pub groq_api_key: Option<String>,
pub github_token: Option<String>,
pub cf_api_token: Option<String>,
pub cf_account_id: Option<String>,
pub gemini_api_key: Option<String>,
pub deepseek_api_key: Option<String>,
pub dashscope_api_key: Option<String>,
```

### `backend/src/routes/problems.rs`

After fetching results for a problem, check for missing models and spawn background benchmark:

```rust
let active_models = db::get_active_models(&pool).await?;
let benchmarked_ids: HashSet<_> = results.iter().map(|r| &r.model_id).collect();
let missing: Vec<_> = active_models.into_iter()
    .filter(|m| !m.is_human && !benchmarked_ids.contains(&m.id))
    .collect();

if !missing.is_empty() {
    let runner = runner.clone();
    let problem = problem.clone();
    let pool = pool.clone();
    tokio::spawn(async move {
        let results = runner.race(&race_id, &problem, missing, tx).await;
        db::save_results(&pool, &results).await.ok();
    });
}
```

### `backend/src/main.rs`

Spawn nightly re-benchmark task after server starts:

```rust
tokio::spawn(async move {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(24 * 3600));
    loop {
        interval.tick().await;
        let stale = db::get_stale_results(&pool, 20, 7).await.unwrap_or_default();
        for (problem, models) in stale {
            runner.race(&Uuid::new_v4().to_string(), &problem, models, tx.clone()).await;
        }
    }
});
```

### `backend/src/db.rs`

Add `get_stale_results(pool, limit, days) -> Vec<(Problem, Vec<Model>)>`:

```sql
SELECT DISTINCT r.problem_id, r.model_id
FROM race_results r
WHERE r.run_at < datetime('now', '-7 days')
ORDER BY r.run_at ASC
LIMIT 20
```

Group by problem, join to models, return problem+model pairs.

### DB seed (migration or seed script)

Insert new model rows:

```sql
INSERT OR IGNORE INTO models (id, provider, name, display_name, api_key_env, is_active, is_new, is_human, added_at) VALUES
  ('groq-llama-33-70b',    'groq',       'llama-3.3-70b-versatile',         'Llama 3.3 70B',         'GROQ_API_KEY',      1, 1, 0, datetime('now')),
  ('groq-mixtral-8x7b',    'groq',       'mixtral-8x7b-32768',              'Mixtral 8x7B',          'GROQ_API_KEY',      1, 0, 0, datetime('now')),
  ('github-gpt4o-mini',    'github',     'gpt-4o-mini',                     'GPT-4o mini',           'GITHUB_TOKEN',      1, 0, 0, datetime('now')),
  ('github-llama-3-70b',   'github',     'Meta-Llama-3-70B-Instruct',       'Llama 3 70B',           'GITHUB_TOKEN',      1, 0, 0, datetime('now')),
  ('cf-llama-3-8b',        'cloudflare', '@cf/meta/llama-3.1-8b-instruct',  'Llama 3.1 8B (CF)',     'CF_API_TOKEN',      1, 0, 0, datetime('now')),
  ('google-gemini-flash',  'google',     'gemini-2.0-flash',                'Gemini 2.0 Flash',      'GEMINI_API_KEY',    1, 1, 0, datetime('now')),
  ('deepseek-v3',          'deepseek',   'deepseek-chat',                   'DeepSeek V3',           'DEEPSEEK_API_KEY',  1, 1, 0, datetime('now')),
  ('deepseek-r1',          'deepseek',   'deepseek-reasoner',               'DeepSeek R1',           'DEEPSEEK_API_KEY',  1, 1, 0, datetime('now')),
  ('qwen-25-72b',          'qwen',       'qwen2.5-72b-instruct',            'Qwen 2.5 72B',          'DASHSCOPE_API_KEY', 1, 1, 0, datetime('now'));
```

### `backend/.env.example`

Document all new keys:
```
GROQ_API_KEY=
GITHUB_TOKEN=
CF_API_TOKEN=
CF_ACCOUNT_ID=
GEMINI_API_KEY=
DEEPSEEK_API_KEY=
DASHSCOPE_API_KEY=
```

---

## What is NOT changing

- Frontend — no changes; leaderboard already renders all model results
- `auth_header` logic — `groq` and `github` use `Authorization: Bearer`, already the default
- `piston.rs` / code verification — unchanged
- Race results schema — unchanged
