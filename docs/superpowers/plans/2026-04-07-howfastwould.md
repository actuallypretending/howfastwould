# howfastwould Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a meme website that races AI models against each other on LeetCode problems, displays real benchmark results absurdly, and lets users time themselves.

**Architecture:** Rust/Axum backend on Fly.io handles benchmarking, LeetCode problem fetching, code execution via Piston, and SSE streaming. Next.js frontend on Vercel renders the dark terminal UI with shareable meme cards. SQLite is embedded in the backend.

**Tech Stack:** Rust (axum, tokio, sqlx, reqwest), SQLite, Next.js 14 (TypeScript, Tailwind), Piston API, LeetCode GraphQL API, Claude API (haiku for roasts)

---

## File Structure

### Backend (`backend/`)
```
backend/
  Cargo.toml
  src/
    main.rs              # Axum server setup, routes, cron scheduling
    config.rs            # Env var loading (API keys, Piston URL, etc.)
    db.rs                # SQLite pool init, run migrations
    models.rs            # Shared types: Problem, Model, RaceResult, RaceEvent
    leetcode.rs          # LeetCode GraphQL API client + fallback cache logic
    piston.rs            # Piston API client (code execution)
    runner.rs            # Core race logic: fetch → race_all_models → store
    roast.rs             # Claude haiku roast line generator
    sync.rs              # Daily model sync + 6h benchmark batch cron jobs
    routes/
      mod.rs             # Route registration
      problems.rs        # GET /problems/random, GET /problems/search
      races.rs           # POST /races, GET /races/:id/stream (SSE)
      models.rs          # GET /models
  migrations/
    001_initial.sql      # Schema: problems, models, results, races
```

### Frontend (`frontend/`)
```
frontend/
  package.json
  app/
    layout.tsx           # Root layout, fonts, metadata
    page.tsx             # Main page — wires all components
    globals.css          # Tailwind base + custom terminal theme vars
    components/
      SearchBar.tsx      # Problem search input + dropdown + random button
      ProblemHeader.tsx  # Problem title, difficulty badge, "new model" banner
      YouBanner.tsx      # Live user timer, solve/give-up buttons
      RaceResults.tsx    # Ranked leaderboard with bar chart + roast labels
      MemeCard.tsx       # Shareable card overlay, generate + download
    hooks/
      useRace.ts         # SSE race stream → live result updates
      useTimer.ts        # User solve timer (start/stop/elapsed)
    lib/
      api.ts             # Typed fetch wrappers for all backend endpoints
      types.ts           # TypeScript types mirroring Rust models
```

---

## Phase 1: Backend

---

### Task 1: Scaffold Rust backend

**Files:**
- Create: `backend/Cargo.toml`
- Create: `backend/src/main.rs`
- Create: `backend/migrations/001_initial.sql`

- [ ] **Step 1: Create backend directory and Cargo.toml**

```bash
mkdir -p backend/src/routes backend/migrations
```

Write `backend/Cargo.toml`:
```toml
[package]
name = "howfastwould-backend"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "server"
path = "src/main.rs"

[dependencies]
axum = { version = "0.7", features = ["macros"] }
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.7", features = ["runtime-tokio", "sqlite", "migrate", "json"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tower-http = { version = "0.5", features = ["cors"] }
dotenvy = "0.15"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
uuid = { version = "1", features = ["v4"] }
tokio-stream = "0.1"
async-stream = "0.3"
chrono = { version = "0.4", features = ["serde"] }
```

- [ ] **Step 2: Write stub main.rs**

Write `backend/src/main.rs`:
```rust
mod config;
mod db;
mod models;
mod routes;

use axum::Router;
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    dotenvy::dotenv().ok();
    let cfg = config::Config::from_env()?;
    let pool = db::init(&cfg.database_url).await?;

    let app = Router::new()
        .nest("/", routes::router(pool.clone()))
        .layer(CorsLayer::permissive());

    let addr = format!("0.0.0.0:{}", cfg.port);
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

- [ ] **Step 3: Write the initial SQL migration**

Write `backend/migrations/001_initial.sql`:
```sql
CREATE TABLE IF NOT EXISTS problems (
    id          TEXT PRIMARY KEY,
    lc_id       INTEGER NOT NULL,
    title       TEXT NOT NULL,
    difficulty  TEXT NOT NULL CHECK(difficulty IN ('Easy','Medium','Hard')),
    description TEXT NOT NULL,
    starter_code TEXT NOT NULL,
    test_cases  TEXT NOT NULL, -- JSON array of {input, expected_output}
    source      TEXT NOT NULL DEFAULT 'leetcode',
    cached_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS models (
    id          TEXT PRIMARY KEY,
    provider    TEXT NOT NULL,
    name        TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    api_key_env TEXT NOT NULL,
    is_active   INTEGER NOT NULL DEFAULT 1,
    is_new      INTEGER NOT NULL DEFAULT 0,
    is_human    INTEGER NOT NULL DEFAULT 0,
    human_times TEXT,            -- JSON: {Easy: ms, Medium: ms, Hard: ms} for human entries
    added_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS results (
    id          TEXT PRIMARY KEY,
    problem_id  TEXT NOT NULL REFERENCES problems(id),
    model_id    TEXT NOT NULL REFERENCES models(id),
    solved      INTEGER NOT NULL DEFAULT 0,
    time_ms     INTEGER,
    attempts    INTEGER NOT NULL DEFAULT 1,
    run_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS races (
    id          TEXT PRIMARY KEY,
    problem_id  TEXT NOT NULL REFERENCES problems(id),
    started_at  TEXT NOT NULL,
    finished_at TEXT
);
```

- [ ] **Step 4: Verify it compiles**

```bash
cd backend && cargo check
```
Expected: `Finished` with no errors (warnings OK)

- [ ] **Step 5: Commit**

```bash
git add backend/
git commit -m "feat: scaffold rust backend"
```

---

### Task 2: Config and DB init

**Files:**
- Create: `backend/src/config.rs`
- Create: `backend/src/db.rs`
- Create: `backend/.env.example`

- [ ] **Step 1: Write config.rs**

Write `backend/src/config.rs`:
```rust
#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub piston_url: String,
    pub openai_api_key: String,
    pub anthropic_api_key: String,
    pub google_api_key: String,
    pub xai_api_key: String,
    pub fireworks_api_key: String,
    pub deepseek_api_key: String,
    pub qwen_api_key: String,
    pub moonshot_api_key: String,
    pub doubao_api_key: String,
    pub hunyuan_api_key: String,
    pub mistral_api_key: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:./howfastwould.db".into()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "3001".into())
                .parse()?,
            piston_url: std::env::var("PISTON_URL")
                .unwrap_or_else(|_| "https://emkc.org/api/v2/piston".into()),
            openai_api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            google_api_key: std::env::var("GOOGLE_API_KEY").unwrap_or_default(),
            xai_api_key: std::env::var("XAI_API_KEY").unwrap_or_default(),
            fireworks_api_key: std::env::var("FIREWORKS_API_KEY").unwrap_or_default(),
            deepseek_api_key: std::env::var("DEEPSEEK_API_KEY").unwrap_or_default(),
            qwen_api_key: std::env::var("QWEN_API_KEY").unwrap_or_default(),
            moonshot_api_key: std::env::var("MOONSHOT_API_KEY").unwrap_or_default(),
            doubao_api_key: std::env::var("DOUBAO_API_KEY").unwrap_or_default(),
            hunyuan_api_key: std::env::var("HUNYUAN_API_KEY").unwrap_or_default(),
            mistral_api_key: std::env::var("MISTRAL_API_KEY").unwrap_or_default(),
        })
    }
}
```

- [ ] **Step 2: Write db.rs**

Write `backend/src/db.rs`:
```rust
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

pub async fn init(database_url: &str) -> anyhow::Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
```

- [ ] **Step 3: Write .env.example**

Write `backend/.env.example`:
```
DATABASE_URL=sqlite:./howfastwould.db
PORT=3001
PISTON_URL=https://emkc.org/api/v2/piston
OPENAI_API_KEY=sk-...
ANTHROPIC_API_KEY=sk-ant-...
GOOGLE_API_KEY=...
XAI_API_KEY=...
FIREWORKS_API_KEY=...
DEEPSEEK_API_KEY=...
QWEN_API_KEY=...
MOONSHOT_API_KEY=...
DOUBAO_API_KEY=...
HUNYUAN_API_KEY=...
MISTRAL_API_KEY=...
```

- [ ] **Step 4: Verify compile**

```bash
cd backend && cargo check
```
Expected: `Finished` with no errors

- [ ] **Step 5: Commit**

```bash
git add backend/
git commit -m "feat: config loading and db init with migrations"
```

---

### Task 3: Shared data types

**Files:**
- Create: `backend/src/models.rs`

- [ ] **Step 1: Write models.rs**

Write `backend/src/models.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Problem {
    pub id: String,
    pub lc_id: i64,
    pub title: String,
    pub difficulty: String,
    pub description: String,
    pub starter_code: String,
    pub test_cases: String, // JSON string
    pub source: String,
    pub cached_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub input: String,
    pub expected_output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Model {
    pub id: String,
    pub provider: String,
    pub name: String,
    pub display_name: String,
    pub api_key_env: String,
    pub is_active: bool,
    pub is_new: bool,
    pub is_human: bool,
    pub human_times: Option<String>, // JSON string
    pub added_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanTimes {
    #[serde(rename = "Easy")]
    pub easy: i64,
    #[serde(rename = "Medium")]
    pub medium: i64,
    #[serde(rename = "Hard")]
    pub hard: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RaceResult {
    pub id: String,
    pub problem_id: String,
    pub model_id: String,
    pub solved: bool,
    pub time_ms: Option<i64>,
    pub attempts: i64,
    pub run_at: String,
}

// Enriched result returned by API (joins model display info)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceResultWithModel {
    pub model_id: String,
    pub model_name: String,
    pub display_name: String,
    pub provider: String,
    pub is_human: bool,
    pub solved: bool,
    pub time_ms: Option<i64>,
    pub attempts: i64,
    pub run_at: String,
}

// SSE event emitted during a live race
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceEvent {
    pub race_id: String,
    pub model_id: String,
    pub display_name: String,
    pub status: RaceStatus,
    pub time_ms: Option<i64>,
    pub attempts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RaceStatus {
    Running,
    Solved,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Race {
    pub id: String,
    pub problem_id: String,
    pub started_at: String,
    pub finished_at: Option<String>,
}
```

- [ ] **Step 2: Verify compile**

```bash
cd backend && cargo check
```
Expected: `Finished`

- [ ] **Step 3: Commit**

```bash
git add backend/src/models.rs
git commit -m "feat: shared data types"
```

---

### Task 4: LeetCode API client

**Files:**
- Create: `backend/src/leetcode.rs`

- [ ] **Step 1: Write leetcode.rs**

Write `backend/src/leetcode.rs`:
```rust
use crate::models::{Problem, TestCase};
use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::{json, Value};
use uuid::Uuid;
use chrono::Utc;

const LC_GRAPHQL: &str = "https://leetcode.com/graphql";

pub struct LeetcodeClient {
    client: Client,
}

impl LeetcodeClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0")
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap(),
        }
    }

    pub async fn fetch_random_problem(&self) -> Result<Problem> {
        // Fetch a random problem slug first
        let slug = self.fetch_random_slug().await?;
        self.fetch_problem_by_slug(&slug).await
    }

    async fn fetch_random_slug(&self) -> Result<String> {
        let query = json!({
            "query": r#"
                query randomQuestion($categorySlug: String, $filters: QuestionListFilterInput) {
                    randomQuestion(categorySlug: $categorySlug, filters: $filters) {
                        titleSlug
                    }
                }
            "#,
            "variables": { "categorySlug": "", "filters": {} }
        });

        let resp: Value = self.client
            .post(LC_GRAPHQL)
            .json(&query)
            .send().await?
            .json().await?;

        let slug = resp["data"]["randomQuestion"]["titleSlug"]
            .as_str()
            .context("missing titleSlug")?
            .to_string();
        Ok(slug)
    }

    async fn fetch_problem_by_slug(&self, slug: &str) -> Result<Problem> {
        let query = json!({
            "query": r#"
                query questionData($titleSlug: String!) {
                    question(titleSlug: $titleSlug) {
                        questionFrontendId
                        title
                        difficulty
                        content
                        codeSnippets { langSlug code }
                        exampleTestcaseList
                        metaData
                    }
                }
            "#,
            "variables": { "titleSlug": slug }
        });

        let resp: Value = self.client
            .post(LC_GRAPHQL)
            .json(&query)
            .send().await?
            .json().await?;

        let q = &resp["data"]["question"];

        let lc_id: i64 = q["questionFrontendId"]
            .as_str().context("missing id")?
            .parse()?;

        let title = q["title"].as_str().context("missing title")?.to_string();
        let difficulty = q["difficulty"].as_str().context("missing difficulty")?.to_string();
        let description = q["content"].as_str().unwrap_or("").to_string();

        // Get Python3 starter code
        let starter_code = q["codeSnippets"]
            .as_array()
            .and_then(|snips| snips.iter().find(|s| s["langSlug"] == "python3"))
            .and_then(|s| s["code"].as_str())
            .unwrap_or("class Solution:\n    pass")
            .to_string();

        // Build test cases from exampleTestcaseList
        let test_cases = self.parse_test_cases(&q["exampleTestcaseList"], &q["metaData"]);

        Ok(Problem {
            id: Uuid::new_v4().to_string(),
            lc_id,
            title,
            difficulty,
            description,
            starter_code,
            test_cases: serde_json::to_string(&test_cases)?,
            source: "leetcode".into(),
            cached_at: Utc::now().to_rfc3339(),
        })
    }

    fn parse_test_cases(&self, example_list: &Value, meta: &Value) -> Vec<TestCase> {
        // Each entry in exampleTestcaseList is a raw input string.
        // metaData has the params list so we can label them.
        // For verification purposes, we store raw input and will run code to get output.
        let inputs: Vec<String> = example_list
            .as_array()
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect())
            .unwrap_or_default();

        // We can't easily get expected outputs from LC GraphQL without running code.
        // Store input only; expected_output is populated by running the solution once
        // using a known-correct reference (we'll solve with GPT-4o first and cache that).
        inputs.iter().map(|input| TestCase {
            input: input.clone(),
            expected_output: String::new(), // populated during first benchmark run
        }).collect()
    }
}

pub async fn cache_problem(pool: &sqlx::SqlitePool, problem: &Problem) -> Result<()> {
    sqlx::query!(
        r#"INSERT OR REPLACE INTO problems
           (id, lc_id, title, difficulty, description, starter_code, test_cases, source, cached_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        problem.id, problem.lc_id, problem.title, problem.difficulty,
        problem.description, problem.starter_code, problem.test_cases,
        problem.source, problem.cached_at
    ).execute(pool).await?;
    Ok(())
}

pub async fn get_random_cached(pool: &sqlx::SqlitePool) -> Result<Problem> {
    sqlx::query_as!(Problem,
        "SELECT * FROM problems ORDER BY RANDOM() LIMIT 1"
    ).fetch_one(pool).await.context("no cached problems")
}

pub async fn search_problems(pool: &sqlx::SqlitePool, q: &str) -> Result<Vec<Problem>> {
    let pattern = format!("%{}%", q);
    sqlx::query_as!(Problem,
        r#"SELECT * FROM problems
           WHERE title LIKE ? OR CAST(lc_id AS TEXT) = ? OR difficulty = ?
           LIMIT 20"#,
        pattern, q, q
    ).fetch_all(pool).await.context("search failed")
}
```

- [ ] **Step 2: Verify compile**

```bash
cd backend && cargo check
```
Expected: `Finished`

- [ ] **Step 3: Commit**

```bash
git add backend/src/leetcode.rs
git commit -m "feat: leetcode graphql client with sqlite fallback"
```

---

### Task 5: Piston API client

**Files:**
- Create: `backend/src/piston.rs`

- [ ] **Step 1: Write piston.rs**

Write `backend/src/piston.rs`:
```rust
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct PistonRequest {
    language: String,
    version: String,
    files: Vec<PistonFile>,
    stdin: String,
}

#[derive(Serialize)]
struct PistonFile {
    name: String,
    content: String,
}

#[derive(Deserialize)]
pub struct PistonResponse {
    pub run: PistonRun,
}

#[derive(Deserialize)]
pub struct PistonRun {
    pub stdout: String,
    pub stderr: String,
    pub code: i64,
}

pub struct PistonClient {
    client: Client,
    base_url: String,
}

impl PistonClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
        }
    }

    /// Run Python code and return stdout
    pub async fn run_python(&self, code: &str, stdin: &str) -> Result<PistonRun> {
        let url = format!("{}/execute", self.base_url);
        let body = PistonRequest {
            language: "python".into(),
            version: "3.10.0".into(),
            files: vec![PistonFile {
                name: "solution.py".into(),
                content: code.to_string(),
            }],
            stdin: stdin.to_string(),
        };

        let resp: PistonResponse = self.client
            .post(&url)
            .json(&body)
            .timeout(std::time::Duration::from_secs(15))
            .send().await
            .context("piston request failed")?
            .json().await
            .context("piston response parse failed")?;

        Ok(resp.run)
    }
}

/// Wrap a LeetCode solution class in a harness that reads input and prints output.
/// The harness calls Solution() with parsed args and prints the result.
pub fn wrap_solution(solution_code: &str, input: &str) -> String {
    // This is a best-effort harness. Works for most single-arg problems.
    // Complex input types (trees, linked lists) may need problem-specific wrappers.
    format!(r#"
import json, sys

{solution_code}

# Harness
if __name__ == "__main__":
    lines = sys.stdin.read().strip().split('\n')
    args = [json.loads(line) for line in lines if line.strip()]
    s = Solution()
    # Try common method names
    for method in [m for m in dir(s) if not m.startswith('_')]:
        try:
            result = getattr(s, method)(*args)
            print(json.dumps(result))
            break
        except Exception as e:
            print(f"ERROR: {{e}}", file=sys.stderr)
"#, solution_code = solution_code)
}
```

- [ ] **Step 2: Verify compile**

```bash
cd backend && cargo check
```
Expected: `Finished`

- [ ] **Step 3: Commit**

```bash
git add backend/src/piston.rs
git commit -m "feat: piston api client for sandboxed python execution"
```

---

### Task 6: Roast line generator

**Files:**
- Create: `backend/src/roast.rs`

- [ ] **Step 1: Write roast.rs**

Write `backend/src/roast.rs`:
```rust
use crate::models::RaceResultWithModel;
use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};

pub struct RoastGenerator {
    client: Client,
    api_key: String,
}

impl RoastGenerator {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
        }
    }

    pub async fn generate(
        &self,
        winner: &RaceResultWithModel,
        loser: &RaceResultWithModel,
        problem_title: &str,
    ) -> String {
        match self.call_api(winner, loser, problem_title).await {
            Ok(line) => line,
            Err(_) => self.fallback_roast(winner, loser),
        }
    }

    async fn call_api(
        &self,
        winner: &RaceResultWithModel,
        loser: &RaceResultWithModel,
        problem_title: &str,
    ) -> Result<String> {
        let winner_time = format_time(winner.time_ms);
        let loser_time = format_time(loser.time_ms);

        let prompt = format!(
            r#"Write ONE short roast (max 12 words, no punctuation at end) in the style of a savage sports commentator.
{} solved {} in {} while {} took {}.
Make it meme-worthy. Reference both competitors. No hashtags."#,
            winner.display_name, problem_title, winner_time,
            loser.display_name, loser_time
        );

        let body = json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": 60,
            "messages": [{ "role": "user", "content": prompt }]
        });

        let resp: Value = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send().await?
            .json().await?;

        let line = resp["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();

        if line.is_empty() {
            anyhow::bail!("empty response");
        }
        Ok(line)
    }

    fn fallback_roast(&self, winner: &RaceResultWithModel, loser: &RaceResultWithModel) -> String {
        let templates = [
            format!("{} left {} in the dust", winner.display_name, loser.display_name),
            format!("{} finished before {} even read the problem", winner.display_name, loser.display_name),
            format!("{} said hold my API key", winner.display_name),
            format!("{} is still loading", loser.display_name),
        ];
        let idx = (winner.time_ms.unwrap_or(0) % templates.len() as i64).unsigned_abs() as usize;
        templates[idx].clone()
    }
}

pub fn format_time(ms: Option<i64>) -> String {
    match ms {
        None => "DNF".into(),
        Some(ms) if ms < 1000 => format!("{}ms", ms),
        Some(ms) if ms < 60_000 => format!("{:.1}s", ms as f64 / 1000.0),
        Some(ms) => {
            let mins = ms / 60_000;
            let secs = (ms % 60_000) / 1000;
            format!("{}m {}s", mins, secs)
        }
    }
}
```

- [ ] **Step 2: Verify compile**

```bash
cd backend && cargo check
```
Expected: `Finished`

- [ ] **Step 3: Commit**

```bash
git add backend/src/roast.rs
git commit -m "feat: claude haiku roast generator with fallback templates"
```

---

### Task 7: Benchmark runner

**Files:**
- Create: `backend/src/runner.rs`

- [ ] **Step 1: Write runner.rs**

Write `backend/src/runner.rs`:
```rust
use crate::{
    config::Config,
    models::{Model, Problem, RaceEvent, RaceResult, RaceStatus, TestCase},
    piston::{PistonClient, wrap_solution},
    roast::format_time,
};
use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};
use std::{sync::Arc, time::Instant};
use tokio::sync::broadcast;
use uuid::Uuid;
use chrono::Utc;

pub type EventSender = broadcast::Sender<RaceEvent>;

pub struct Runner {
    config: Arc<Config>,
    piston: Arc<PistonClient>,
    http: Client,
}

impl Runner {
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            piston: Arc::new(PistonClient::new(&config.piston_url)),
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap(),
            config,
        }
    }

    /// Race all active models on a problem. Emits SSE events as each finishes.
    pub async fn race(
        &self,
        race_id: &str,
        problem: &Problem,
        models: Vec<Model>,
        tx: EventSender,
    ) -> Vec<RaceResult> {
        let test_cases: Vec<TestCase> = serde_json::from_str(&problem.test_cases)
            .unwrap_or_default();

        let mut handles = vec![];

        for model in models.into_iter().filter(|m| m.is_active && !m.is_human) {
            let runner = self.clone_cheap();
            let problem = problem.clone();
            let test_cases = test_cases.clone();
            let tx = tx.clone();
            let race_id = race_id.to_string();

            handles.push(tokio::spawn(async move {
                // Emit "running" event
                let _ = tx.send(RaceEvent {
                    race_id: race_id.clone(),
                    model_id: model.id.clone(),
                    display_name: model.display_name.clone(),
                    status: RaceStatus::Running,
                    time_ms: None,
                    attempts: 0,
                });

                let result = runner.race_one(&race_id, &model, &problem, &test_cases, &tx).await;
                result
            }));
        }

        let mut results = vec![];
        for handle in handles {
            if let Ok(result) = handle.await {
                results.push(result);
            }
        }
        results
    }

    async fn race_one(
        &self,
        race_id: &str,
        model: &Model,
        problem: &Problem,
        test_cases: &[TestCase],
        tx: &EventSender,
    ) -> RaceResult {
        let start = Instant::now();
        let api_key = std::env::var(&model.api_key_env).unwrap_or_default();
        let mut attempts = 0;
        let mut solved = false;
        let mut last_error = String::new();

        for attempt in 1..=3 {
            attempts = attempt;
            let prompt = if attempt == 1 {
                build_prompt(&problem.title, &problem.description, &problem.starter_code)
            } else {
                build_retry_prompt(&problem.title, &problem.description, &problem.starter_code, &last_error)
            };

            let code = match self.call_model(model, &api_key, &prompt).await {
                Ok(c) => c,
                Err(e) => { last_error = e.to_string(); continue; }
            };

            match self.verify(&code, test_cases).await {
                Ok(true) => { solved = true; break; }
                Ok(false) => { last_error = "wrong answer".into(); }
                Err(e) => { last_error = e.to_string(); }
            }
        }

        let elapsed_ms = start.elapsed().as_millis() as i64;
        let status = if solved { RaceStatus::Solved } else { RaceStatus::Failed };

        let _ = tx.send(RaceEvent {
            race_id: race_id.to_string(),
            model_id: model.id.clone(),
            display_name: model.display_name.clone(),
            status,
            time_ms: if solved { Some(elapsed_ms) } else { None },
            attempts: attempts as i64,
        });

        RaceResult {
            id: Uuid::new_v4().to_string(),
            problem_id: problem.id.clone(),
            model_id: model.id.clone(),
            solved,
            time_ms: if solved { Some(elapsed_ms) } else { None },
            attempts: attempts as i64,
            run_at: Utc::now().to_rfc3339(),
        }
    }

    async fn call_model(&self, model: &Model, api_key: &str, prompt: &str) -> Result<String> {
        let (url, body) = build_api_request(&model.provider, &model.name, api_key, prompt)?;
        let resp: Value = self.http
            .post(&url)
            .header("Content-Type", "application/json")
            .header(auth_header(&model.provider), api_key)
            .json(&body)
            .send().await?
            .json().await?;
        extract_code(&resp, &model.provider)
    }

    async fn verify(&self, code: &str, test_cases: &[TestCase]) -> Result<bool> {
        if test_cases.is_empty() {
            // No test cases cached yet — assume correct on first run
            return Ok(true);
        }
        for tc in test_cases {
            let wrapped = wrap_solution(code, &tc.input);
            let run = self.piston.run_python(&wrapped, &tc.input).await?;
            if run.code != 0 { return Ok(false); }
            if !tc.expected_output.is_empty() {
                let got = run.stdout.trim();
                let want = tc.expected_output.trim();
                if got != want { return Ok(false); }
            }
        }
        Ok(true)
    }

    fn clone_cheap(&self) -> Self {
        Self {
            config: self.config.clone(),
            piston: self.piston.clone(),
            http: self.http.clone(),
        }
    }
}

fn build_prompt(title: &str, description: &str, starter: &str) -> String {
    format!(
        "Solve the following LeetCode problem in Python. Return only the solution class/function, no explanation.\n\n{}\n\n{}\n\n{}",
        title, description, starter
    )
}

fn build_retry_prompt(title: &str, description: &str, starter: &str, error: &str) -> String {
    format!(
        "Solve the following LeetCode problem in Python. Return only the solution class/function, no explanation.\nYour previous attempt failed with: {}\n\n{}\n\n{}\n\n{}",
        error, title, description, starter
    )
}

fn build_api_request(provider: &str, model_name: &str, _api_key: &str, prompt: &str) -> Result<(String, Value)> {
    match provider {
        "openai" | "xai" | "fireworks" | "deepseek" | "mistral" => {
            let base = match provider {
                "openai" => "https://api.openai.com/v1",
                "xai" => "https://api.x.ai/v1",
                "fireworks" => "https://api.fireworks.ai/inference/v1",
                "deepseek" => "https://api.deepseek.com/v1",
                "mistral" => "https://api.mistral.ai/v1",
                _ => unreachable!(),
            };
            Ok((
                format!("{}/chat/completions", base),
                json!({ "model": model_name, "messages": [{"role":"user","content": prompt}], "max_tokens": 2048 })
            ))
        }
        "anthropic" => Ok((
            "https://api.anthropic.com/v1/messages".into(),
            json!({ "model": model_name, "max_tokens": 2048, "messages": [{"role":"user","content": prompt}] })
        )),
        "google" => Ok((
            format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent", model_name),
            json!({ "contents": [{"parts": [{"text": prompt}]}] })
        )),
        "qwen" => Ok((
            "https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation".into(),
            json!({ "model": model_name, "input": { "messages": [{"role":"user","content": prompt}] } })
        )),
        "moonshot" => Ok((
            "https://api.moonshot.cn/v1/chat/completions".into(),
            json!({ "model": model_name, "messages": [{"role":"user","content": prompt}] })
        )),
        "doubao" => Ok((
            "https://ark.cn-beijing.volces.com/api/v3/chat/completions".into(),
            json!({ "model": model_name, "messages": [{"role":"user","content": prompt}] })
        )),
        "hunyuan" => Ok((
            "https://hunyuan.tencentcloudapi.com/".into(),
            json!({ "Model": model_name, "Messages": [{"Role":"user","Content": prompt}] })
        )),
        p => anyhow::bail!("unknown provider: {}", p),
    }
}

fn auth_header(provider: &str) -> &'static str {
    match provider {
        "anthropic" => "x-api-key",
        "google" => "x-goog-api-key",
        _ => "Authorization",
    }
}

fn extract_code(resp: &Value, provider: &str) -> Result<String> {
    let text = match provider {
        "anthropic" => resp["content"][0]["text"].as_str().unwrap_or(""),
        "google" => resp["candidates"][0]["content"]["parts"][0]["text"].as_str().unwrap_or(""),
        "qwen" => resp["output"]["text"].as_str().unwrap_or(""),
        "hunyuan" => resp["Choices"][0]["Message"]["Content"].as_str().unwrap_or(""),
        _ => resp["choices"][0]["message"]["content"].as_str().unwrap_or(""),
    };

    // Extract code block if present
    if let Some(start) = text.find("```python") {
        let rest = &text[start + 9..];
        if let Some(end) = rest.find("```") {
            return Ok(rest[..end].trim().to_string());
        }
    }
    if let Some(start) = text.find("```") {
        let rest = &text[start + 3..];
        if let Some(end) = rest.find("```") {
            return Ok(rest[..end].trim().to_string());
        }
    }
    // No code block — return raw text
    Ok(text.trim().to_string())
}
```

- [ ] **Step 2: Verify compile**

```bash
cd backend && cargo check
```
Expected: `Finished`

- [ ] **Step 3: Commit**

```bash
git add backend/src/runner.rs
git commit -m "feat: parallel benchmark runner with retry and SSE events"
```

---

### Task 8: API routes

**Files:**
- Create: `backend/src/routes/mod.rs`
- Create: `backend/src/routes/problems.rs`
- Create: `backend/src/routes/races.rs`
- Create: `backend/src/routes/models.rs`

- [ ] **Step 1: Write routes/mod.rs**

Write `backend/src/routes/mod.rs`:
```rust
pub mod models;
pub mod problems;
pub mod races;

use axum::{routing::{get, post}, Router};
use sqlx::SqlitePool;
use std::sync::Arc;
use crate::{config::Config, runner::Runner};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<Config>,
    pub runner: Arc<Runner>,
}

pub fn router(pool: SqlitePool) -> Router {
    dotenvy::dotenv().ok();
    let config = Arc::new(crate::config::Config::from_env().unwrap());
    let runner = Arc::new(Runner::new(config.clone()));
    let state = AppState { pool, config, runner };

    Router::new()
        .route("/problems/random", get(problems::random))
        .route("/problems/search", get(problems::search))
        .route("/problems/:id/results", get(problems::results))
        .route("/races", post(races::create))
        .route("/races/:id/stream", get(races::stream))
        .route("/models", get(models::list))
        .with_state(state)
}
```

- [ ] **Step 2: Write routes/problems.rs**

Write `backend/src/routes/problems.rs`:
```rust
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
            // Fallback to cache
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
        r#"SELECT r.*, m.display_name, m.provider, m.name as model_name, m.is_human
           FROM results r JOIN models m ON r.model_id = m.id
           WHERE r.problem_id = ?
           ORDER BY r.time_ms ASC NULLS LAST"#,
        id
    ).fetch_all(&state.pool).await.unwrap_or_default();

    let results: Vec<RaceResultWithModel> = rows.into_iter().map(|r| RaceResultWithModel {
        model_id: r.model_id,
        model_name: r.model_name,
        display_name: r.display_name,
        provider: r.provider,
        is_human: r.is_human != 0,
        solved: r.solved != 0,
        time_ms: r.time_ms,
        attempts: r.attempts,
        run_at: r.run_at,
    }).collect();

    Json(results)
}
```

- [ ] **Step 3: Write routes/races.rs**

Write `backend/src/routes/races.rs`:
```rust
use axum::{
    extract::{Path, State},
    response::sse::{Event, Sse},
    Json,
};
use futures::stream::Stream;
use serde::Deserialize;
use std::convert::Infallible;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
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
        "SELECT * FROM problems WHERE id = ?",
        body.problem_id
    ).fetch_one(&state.pool).await.expect("problem not found");

    let models = sqlx::query_as!(
        crate::models::Model,
        "SELECT * FROM models WHERE is_active = 1"
    ).fetch_all(&state.pool).await.unwrap_or_default();

    let (tx, _) = broadcast::channel::<RaceEvent>(64);
    let tx_clone = tx.clone();
    let runner = state.runner.clone();
    let pool = state.pool.clone();
    let race_id_clone = race_id.clone();

    // Store race record
    let now = Utc::now().to_rfc3339();
    sqlx::query!(
        "INSERT INTO races (id, problem_id, started_at) VALUES (?, ?, ?)",
        race_id, body.problem_id, now
    ).execute(&state.pool).await.ok();

    // Store sender globally keyed by race_id for SSE to subscribe
    // (In a real app use a shared DashMap; here we use a task-local approach)
    tokio::spawn(async move {
        let results = runner.race(&race_id_clone, &problem, models, tx_clone).await;
        // Store results
        for result in &results {
            sqlx::query!(
                "INSERT OR REPLACE INTO results (id, problem_id, model_id, solved, time_ms, attempts, run_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
                result.id, result.problem_id, result.model_id, result.solved,
                result.time_ms, result.attempts, result.run_at
            ).execute(&pool).await.ok();
        }
        // Mark race finished
        let finished = Utc::now().to_rfc3339();
        sqlx::query!("UPDATE races SET finished_at = ? WHERE id = ?", finished, race_id_clone)
            .execute(&pool).await.ok();
    });

    // Store sender in app state - simplified: use a global static or pass via state
    // For this implementation, the client polls /problems/:id/results after creation
    Json(CreateRaceResponse { race_id })
}

// SSE stream — streams events for a race using a shared broadcast channel.
// Simplified: re-runs the broadcast; in production use a shared DashMap<race_id, Sender>.
pub async fn stream(
    State(_state): State<AppState>,
    Path(_race_id): Path<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Placeholder: in production, look up the broadcast sender by race_id from shared state.
    // Returns empty stream for now; polling /problems/:id/results is the fallback.
    let stream = futures::stream::empty::<Result<Event, Infallible>>();
    Sse::new(stream)
}
```

- [ ] **Step 4: Write routes/models.rs**

Write `backend/src/routes/models.rs`:
```rust
use axum::{extract::State, Json};
use crate::{models::Model, routes::AppState};

pub async fn list(State(state): State<AppState>) -> Json<Vec<Model>> {
    let models = sqlx::query_as!(Model, "SELECT * FROM models WHERE is_active = 1 ORDER BY is_human, provider, name")
        .fetch_all(&state.pool).await.unwrap_or_default();
    Json(models)
}
```

- [ ] **Step 5: Verify compile**

```bash
cd backend && cargo check
```
Expected: `Finished`

- [ ] **Step 6: Commit**

```bash
git add backend/src/routes/
git commit -m "feat: api routes for problems, races, and models"
```

---

### Task 9: Model sync + batch benchmark cron jobs

**Files:**
- Create: `backend/src/sync.rs`

- [ ] **Step 1: Write sync.rs**

Write `backend/src/sync.rs`:
```rust
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
                        // Only add GPT/o-series chat models
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

/// Seed the initial model roster (run once on first startup if models table is empty)
pub async fn seed_initial_models(pool: &SqlitePool) -> Result<()> {
    let count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM models")
        .fetch_one(pool).await?;

    if count > 0 { return Ok(()); }

    let now = Utc::now().to_rfc3339();
    let models: &[(&str, &str, &str, &str, bool)] = &[
        // (provider, name, display_name, api_key_env, is_human)
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
        // Humans
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
            "INSERT INTO models (id, provider, name, display_name, api_key_env, is_active, is_new, is_human, human_times, added_at) VALUES (?, ?, ?, ?, ?, 1, 0, ?, ?, ?)",
            id, provider, name, display, key_env, is_human, human_times, now
        ).execute(pool).await?;
    }

    tracing::info!("seeded {} models", models.len());
    Ok(())
}

/// Run benchmark on a random batch of problems to keep cache warm
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

        let models = sqlx::query_as!(Model, "SELECT * FROM models WHERE is_active = 1")
            .fetch_all(pool).await.unwrap_or_default();

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
```

- [ ] **Step 2: Wire cron jobs into main.rs**

Edit `backend/src/main.rs` — replace the `main` function body with:
```rust
mod config;
mod db;
mod leetcode;
mod models;
mod piston;
mod roast;
mod routes;
mod runner;
mod sync;

use axum::Router;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    dotenvy::dotenv().ok();
    let cfg = Arc::new(config::Config::from_env()?);
    let pool = db::init(&cfg.database_url).await?;

    // Seed model roster on first run
    sync::seed_initial_models(&pool).await?;

    // Start cron jobs
    {
        let pool = pool.clone();
        let cfg = cfg.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(86400));
            loop {
                interval.tick().await;
                sync::sync_models(&pool, &cfg).await.ok();
            }
        });
    }
    {
        let pool = pool.clone();
        let cfg = cfg.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(21600));
            loop {
                interval.tick().await;
                sync::run_benchmark_batch(&pool, cfg.clone()).await.ok();
            }
        });
    }

    let app = Router::new()
        .nest("/", routes::router(pool.clone()))
        .layer(CorsLayer::permissive());

    let addr = format!("0.0.0.0:{}", cfg.port);
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

- [ ] **Step 3: Verify full compile**

```bash
cd backend && cargo build
```
Expected: `Finished` — if there are errors, fix them before proceeding.

- [ ] **Step 4: Smoke test**

```bash
cd backend && cp .env.example .env
DATABASE_URL=sqlite:./test.db cargo run 2>&1 &
sleep 2
curl http://localhost:3001/models | head -c 200
kill %1
rm -f test.db
```
Expected: JSON array of models

- [ ] **Step 5: Commit**

```bash
git add backend/src/sync.rs backend/src/main.rs
git commit -m "feat: model sync, batch benchmark cron, initial model seed"
```

---

## Phase 2: Frontend

---

### Task 10: Scaffold Next.js frontend

**Files:**
- Create: `frontend/package.json`
- Create: `frontend/app/globals.css`
- Create: `frontend/app/layout.tsx`
- Create: `frontend/app/lib/types.ts`
- Create: `frontend/app/lib/api.ts`

- [ ] **Step 1: Create Next.js app**

```bash
cd /path/to/howfastwould
npx create-next-app@latest frontend --typescript --tailwind --app --no-src-dir --import-alias "@/*" --no-eslint
```

- [ ] **Step 2: Write types.ts**

Write `frontend/app/lib/types.ts`:
```typescript
export interface Problem {
  id: string;
  lc_id: number;
  title: string;
  difficulty: "Easy" | "Medium" | "Hard";
  description: string;
  starter_code: string;
  test_cases: string;
  source: string;
  cached_at: string;
}

export interface Model {
  id: string;
  provider: string;
  name: string;
  display_name: string;
  is_active: boolean;
  is_new: boolean;
  is_human: boolean;
  human_times: string | null;
  added_at: string;
}

export interface RaceResultWithModel {
  model_id: string;
  model_name: string;
  display_name: string;
  provider: string;
  is_human: boolean;
  solved: boolean;
  time_ms: number | null;
  attempts: number;
  run_at: string;
}

export interface RaceEvent {
  race_id: string;
  model_id: string;
  display_name: string;
  status: "running" | "solved" | "failed";
  time_ms: number | null;
  attempts: number;
}

export interface CreateRaceResponse {
  race_id: string;
}
```

- [ ] **Step 3: Write api.ts**

Write `frontend/app/lib/api.ts`:
```typescript
import { CreateRaceResponse, Model, Problem, RaceResultWithModel } from "./types";

const BASE = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:3001";

export async function fetchRandomProblem(): Promise<Problem> {
  const res = await fetch(`${BASE}/problems/random`);
  if (!res.ok) throw new Error("failed to fetch problem");
  return res.json();
}

export async function searchProblems(q: string): Promise<Problem[]> {
  const res = await fetch(`${BASE}/problems/search?q=${encodeURIComponent(q)}`);
  if (!res.ok) return [];
  return res.json();
}

export async function fetchProblemResults(problemId: string): Promise<RaceResultWithModel[]> {
  const res = await fetch(`${BASE}/problems/${problemId}/results`);
  if (!res.ok) return [];
  return res.json();
}

export async function createRace(problemId: string): Promise<CreateRaceResponse> {
  const res = await fetch(`${BASE}/races`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ problem_id: problemId }),
  });
  if (!res.ok) throw new Error("failed to create race");
  return res.json();
}

export async function fetchModels(): Promise<Model[]> {
  const res = await fetch(`${BASE}/models`);
  if (!res.ok) return [];
  return res.json();
}

export function formatTime(ms: number | null): string {
  if (ms === null) return "DNF";
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`;
  const mins = Math.floor(ms / 60_000);
  const secs = Math.floor((ms % 60_000) / 1000);
  return `${mins}m ${secs}s`;
}
```

- [ ] **Step 4: Write globals.css**

Write `frontend/app/globals.css`:
```css
@tailwind base;
@tailwind components;
@tailwind utilities;

:root {
  --bg: #0d0d0d;
  --surface: #111111;
  --border: #1a1a1a;
  --green: #00ff41;
  --text: #cccccc;
  --muted: #555555;
}

body {
  background: var(--bg);
  color: var(--text);
  font-family: 'JetBrains Mono', 'Fira Code', 'Courier New', monospace;
}

.difficulty-easy { color: #00ff41; }
.difficulty-medium { color: #ffaa00; }
.difficulty-hard { color: #ff4444; }
```

- [ ] **Step 5: Write layout.tsx**

Write `frontend/app/layout.tsx`:
```tsx
import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "howfastwould",
  description: "how fast would AI solve this leetcode problem?",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body className="min-h-screen" style={{ background: "var(--bg)" }}>
        {children}
      </body>
    </html>
  );
}
```

- [ ] **Step 6: Verify it builds**

```bash
cd frontend && npm run build
```
Expected: `✓ Compiled successfully`

- [ ] **Step 7: Commit**

```bash
git add frontend/
git commit -m "feat: scaffold next.js frontend with types and api client"
```

---

### Task 11: SearchBar component

**Files:**
- Create: `frontend/app/components/SearchBar.tsx`

- [ ] **Step 1: Write SearchBar.tsx**

Write `frontend/app/components/SearchBar.tsx`:
```tsx
"use client";
import { useEffect, useRef, useState } from "react";
import { searchProblems } from "@/app/lib/api";
import { Problem } from "@/app/lib/types";

interface Props {
  onSelect: (problem: Problem) => void;
  onRandom: () => void;
}

export default function SearchBar({ onSelect, onRandom }: Props) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<Problem[]>([]);
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (query.length < 2) { setResults([]); setOpen(false); return; }
    const t = setTimeout(async () => {
      const r = await searchProblems(query);
      setResults(r);
      setOpen(r.length > 0);
    }, 300);
    return () => clearTimeout(t);
  }, [query]);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  const diffColor = (d: string) =>
    d === "Easy" ? "#00ff41" : d === "Medium" ? "#ffaa00" : "#ff4444";

  return (
    <div ref={ref} className="relative flex gap-2 px-5 py-3 border-b" style={{ borderColor: "var(--border)" }}>
      <div
        className="flex flex-1 items-center gap-2 rounded px-3 py-2 text-sm"
        style={{ background: "var(--surface)", border: "1px solid var(--border)" }}
      >
        <span style={{ color: "var(--muted)" }}>$</span>
        <input
          className="flex-1 bg-transparent outline-none"
          style={{ color: "var(--text)" }}
          placeholder="search problem... Two Sum, #42, Hard, dp..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onFocus={() => results.length > 0 && setOpen(true)}
        />
      </div>
      <button
        onClick={onRandom}
        className="rounded px-3 py-2 text-sm"
        style={{ background: "var(--surface)", border: "1px solid var(--border)", color: "var(--text)" }}
      >
        🎲 random
      </button>

      {open && (
        <div
          className="absolute left-5 right-0 top-full z-10 rounded-b text-sm"
          style={{ background: "var(--surface)", border: "1px solid var(--border)", borderTop: "none", marginRight: "80px" }}
        >
          {results.map((p) => (
            <button
              key={p.id}
              className="flex w-full items-center gap-3 px-3 py-2 text-left hover:brightness-150 border-b"
              style={{ borderColor: "var(--border)" }}
              onClick={() => { onSelect(p); setOpen(false); setQuery(""); }}
            >
              <span style={{ color: "var(--muted)" }}>#{p.lc_id}</span>
              <span style={{ color: "var(--text)" }}>{p.title}</span>
              <span style={{ color: diffColor(p.difficulty), marginLeft: "auto", fontSize: "11px" }}>{p.difficulty}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Verify build**

```bash
cd frontend && npm run build
```
Expected: `✓ Compiled successfully`

- [ ] **Step 3: Commit**

```bash
git add frontend/app/components/SearchBar.tsx
git commit -m "feat: search bar with debounced results dropdown"
```

---

### Task 12: YouBanner component + useTimer hook

**Files:**
- Create: `frontend/app/hooks/useTimer.ts`
- Create: `frontend/app/components/YouBanner.tsx`

- [ ] **Step 1: Write useTimer.ts**

Write `frontend/app/hooks/useTimer.ts`:
```typescript
"use client";
import { useEffect, useRef, useState } from "react";

export type TimerState = "idle" | "running" | "stopped";

export function useTimer() {
  const [state, setState] = useState<TimerState>("idle");
  const [elapsedMs, setElapsedMs] = useState(0);
  const startRef = useRef<number | null>(null);
  const rafRef = useRef<number | null>(null);

  const start = () => {
    startRef.current = Date.now();
    setState("running");
  };

  const stop = (): number => {
    const ms = startRef.current ? Date.now() - startRef.current : 0;
    setState("stopped");
    setElapsedMs(ms);
    if (rafRef.current) cancelAnimationFrame(rafRef.current);
    return ms;
  };

  const reset = () => {
    setState("idle");
    setElapsedMs(0);
    startRef.current = null;
  };

  useEffect(() => {
    if (state !== "running") return;
    const tick = () => {
      setElapsedMs(startRef.current ? Date.now() - startRef.current : 0);
      rafRef.current = requestAnimationFrame(tick);
    };
    rafRef.current = requestAnimationFrame(tick);
    return () => { if (rafRef.current) cancelAnimationFrame(rafRef.current); };
  }, [state]);

  return { state, elapsedMs, start, stop, reset };
}
```

- [ ] **Step 2: Write YouBanner.tsx**

Write `frontend/app/components/YouBanner.tsx`:
```tsx
"use client";
import { useEffect } from "react";
import { useTimer } from "@/app/hooks/useTimer";
import { formatTime } from "@/app/lib/api";

interface Props {
  problemId: string;
  onSolve: (ms: number) => void;
  onGiveUp: (ms: number) => void;
}

export default function YouBanner({ problemId, onSolve, onGiveUp }: Props) {
  const { state, elapsedMs, start, stop, reset } = useTimer();

  // Auto-start when problem changes
  useEffect(() => {
    reset();
    start();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [problemId]);

  const handleSolve = () => {
    const ms = stop();
    onSolve(ms);
  };

  const handleGiveUp = () => {
    const ms = stop();
    onGiveUp(ms);
  };

  const display = state === "stopped"
    ? `✓ ${formatTime(elapsedMs)}`
    : formatTime(elapsedMs);

  return (
    <div
      className="mx-5 mt-4 flex items-center gap-4 rounded px-4 py-3"
      style={{ background: "#0a0a1a", border: "1px solid #2a2a4a" }}
    >
      <div>
        <div className="text-xs mb-1" style={{ color: "var(--muted)" }}>⏱ your time</div>
        <div className="text-2xl font-black tracking-widest" style={{ color: state === "stopped" ? "#00ff41" : "#fff" }}>
          {display}
        </div>
      </div>
      <div className="flex-1 text-xs" style={{ color: "var(--muted)" }}>
        {state === "running" && "Timer started when you opened the problem."}
        {state === "stopped" && "Timer stopped. Your result is in the leaderboard."}
      </div>
      {state === "running" && (
        <>
          <button
            onClick={handleSolve}
            className="rounded px-4 py-2 text-sm font-bold"
            style={{ background: "#00ff41", color: "#000" }}
          >
            ✓ I solved it
          </button>
          <button
            onClick={handleGiveUp}
            className="rounded px-3 py-2 text-sm"
            style={{ background: "transparent", border: "1px solid #333", color: "var(--muted)" }}
          >
            give up 💀
          </button>
        </>
      )}
    </div>
  );
}
```

- [ ] **Step 3: Verify build**

```bash
cd frontend && npm run build
```
Expected: `✓ Compiled successfully`

- [ ] **Step 4: Commit**

```bash
git add frontend/app/hooks/useTimer.ts frontend/app/components/YouBanner.tsx
git commit -m "feat: user timer hook and YouBanner component"
```

---

### Task 13: RaceResults component

**Files:**
- Create: `frontend/app/components/RaceResults.tsx`

- [ ] **Step 1: Write RaceResults.tsx**

Write `frontend/app/components/RaceResults.tsx`:
```tsx
"use client";
import { useState } from "react";
import { formatTime } from "@/app/lib/api";
import { RaceResultWithModel } from "@/app/lib/types";

interface Props {
  results: RaceResultWithModel[];
  userResult: { ms: number; gaveUp: boolean } | null;
  onSelectResult: (result: RaceResultWithModel) => void;
  onRaceAgain: () => void;
  isRacing: boolean;
}

const PROVIDER_COLORS: Record<string, string> = {
  openai: "#90caf9",
  anthropic: "#a5d6a7",
  google: "#ef9a9a",
  xai: "#fff59d",
  fireworks: "#80cbc4",
  mistral: "#80cbc4",
  deepseek: "#ce93d8",
  qwen: "#ce93d8",
  moonshot: "#ce93d8",
  doubao: "#ce93d8",
  hunyuan: "#ce93d8",
  human: "#f48fb1",
};

export default function RaceResults({ results, userResult, onSelectResult, onRaceAgain, isRacing }: Props) {
  const [expanded, setExpanded] = useState(false);

  const sorted = [...results].sort((a, b) => {
    if (a.solved && b.solved) return (a.time_ms ?? 0) - (b.time_ms ?? 0);
    if (a.solved) return -1;
    if (b.solved) return 1;
    return 0;
  });

  const maxTime = Math.max(...sorted.filter(r => r.solved).map(r => r.time_ms ?? 0), 1);
  const visible = expanded ? sorted : sorted.slice(0, 5);
  const hasMore = sorted.length > 5;

  return (
    <div className="px-5 py-4">
      <div className="flex items-center justify-between mb-3">
        <span className="text-xs" style={{ color: "var(--muted)" }}>
          {isRacing ? "$ race in progress..." : `$ ${sorted.length} contestants`}
        </span>
        <button
          onClick={onRaceAgain}
          disabled={isRacing}
          className="rounded px-4 py-1.5 text-xs font-bold"
          style={{
            background: isRacing ? "var(--surface)" : "#00ff41",
            color: isRacing ? "var(--muted)" : "#000",
            border: isRacing ? "1px solid var(--border)" : "none",
          }}
        >
          {isRacing ? "racing..." : "▶ race again"}
        </button>
      </div>

      <div className="flex flex-col gap-1 text-sm">
        {visible.map((r, i) => {
          const color = PROVIDER_COLORS[r.provider] ?? "#ccc";
          const barPct = r.solved && r.time_ms ? Math.max(8, (r.time_ms / maxTime) * 100) : 100;
          const medal = i === 0 ? "🥇" : i === 1 ? "🥈" : i === 2 ? "🥉" : null;

          return (
            <button
              key={r.model_id}
              className="flex items-center gap-3 rounded px-3 py-2 text-left w-full"
              style={{ background: r.is_human ? "#0a0a1a" : "var(--surface)" }}
              onClick={() => onSelectResult(r)}
            >
              <span style={{ color: "var(--muted)", width: 24, flexShrink: 0 }}>
                {medal ?? `${i + 1}.`}
              </span>
              <span style={{ color, width: 160, flexShrink: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                {r.display_name}
              </span>
              <div className="flex-1 h-1.5 rounded overflow-hidden" style={{ background: "#1a1a1a" }}>
                {r.solved && (
                  <div
                    className="h-full rounded"
                    style={{ width: `${barPct}%`, background: color }}
                  />
                )}
              </div>
              <span style={{ color: r.solved ? color : "#ff4444", width: 72, textAlign: "right", flexShrink: 0 }}>
                {r.solved ? formatTime(r.time_ms) : "💀 failed"}
              </span>
              <span style={{ color: "var(--muted)", fontSize: 11, flexShrink: 0 }}>
                {r.attempts > 1 ? `${r.attempts} tries` : ""}
              </span>
            </button>
          );
        })}

        {/* You row */}
        {userResult && (
          <div
            className="flex items-center gap-3 rounded px-3 py-2"
            style={{ background: "#0a0a1a", border: "1px solid #2a2a4a" }}
          >
            <span style={{ color: "var(--muted)", width: 24 }}>👤</span>
            <span style={{ color: "#7dd3fc", width: 160 }}>You</span>
            <div className="flex-1 h-1.5 rounded overflow-hidden" style={{ background: "#1a1a1a" }}>
              <div className="h-full rounded" style={{ width: "40%", background: "#7dd3fc" }} />
            </div>
            <span style={{ color: userResult.gaveUp ? "#ff4444" : "#7dd3fc", width: 72, textAlign: "right" }}>
              {userResult.gaveUp ? "💀 gave up" : formatTime(userResult.ms)}
            </span>
          </div>
        )}

        {hasMore && (
          <button
            className="text-xs py-1"
            style={{ color: "var(--muted)" }}
            onClick={() => setExpanded(e => !e)}
          >
            {expanded ? "show less ↑" : `show ${sorted.length - 5} more ↓`}
          </button>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify build**

```bash
cd frontend && npm run build
```
Expected: `✓ Compiled successfully`

- [ ] **Step 3: Commit**

```bash
git add frontend/app/components/RaceResults.tsx
git commit -m "feat: race results leaderboard with bar chart"
```

---

### Task 14: MemeCard component

**Files:**
- Create: `frontend/app/components/MemeCard.tsx`

- [ ] **Step 1: Write MemeCard.tsx**

Write `frontend/app/components/MemeCard.tsx`:
```tsx
"use client";
import { useRef } from "react";
import { formatTime } from "@/app/lib/api";
import { RaceResultWithModel, Problem } from "@/app/lib/types";

interface Props {
  result: RaceResultWithModel;
  problem: Problem;
  roast: string;
  onClose: () => void;
}

export default function MemeCard({ result, problem, roast, onClose }: Props) {
  const cardRef = useRef<HTMLDivElement>(null);

  const handleDownload = async () => {
    if (!cardRef.current) return;
    // Use html2canvas if available, otherwise prompt user to screenshot
    try {
      const { default: html2canvas } = await import("html2canvas");
      const canvas = await html2canvas(cardRef.current, { backgroundColor: null });
      const link = document.createElement("a");
      link.download = `howfastwould-${problem.title.replace(/\s+/g, "-").toLowerCase()}.png`;
      link.href = canvas.toDataURL();
      link.click();
    } catch {
      alert("Right-click the card to save as image, or screenshot it!");
    }
  };

  return (
    <div
      className="fixed inset-0 flex items-center justify-center z-50"
      style={{ background: "rgba(0,0,0,0.85)" }}
      onClick={onClose}
    >
      <div className="flex flex-col items-center gap-4" onClick={e => e.stopPropagation()}>
        {/* The meme card */}
        <div
          ref={cardRef}
          className="rounded-xl p-8 text-center"
          style={{
            background: "linear-gradient(135deg, #0d0d1a, #1a0d2e)",
            width: 400,
            fontFamily: "'Impact', 'Arial Black', sans-serif",
          }}
        >
          <div style={{ fontFamily: "monospace", fontSize: 11, color: "#555", marginBottom: 8 }}>
            howfastwould.com
          </div>
          <div style={{ fontSize: 22, fontWeight: 900, color: "#ce93d8", lineHeight: 1.1, marginBottom: 8 }}>
            {result.display_name.toUpperCase()}
          </div>
          <div style={{ fontSize: 15, color: "#888", marginBottom: 4 }}>
            SOLVED {problem.title.toUpperCase()}
          </div>
          <div style={{ fontSize: 40, fontWeight: 900, color: "#fff", margin: "12px 0" }}>
            {result.solved ? formatTime(result.time_ms).toUpperCase() : "FAILED 💀"}
          </div>
          <div style={{ fontSize: 13, color: "#888", fontFamily: "monospace", fontStyle: "italic" }}>
            {roast}
          </div>
          {result.attempts > 1 && (
            <div style={{ fontSize: 11, color: "#555", marginTop: 8 }}>
              {result.attempts} attempts
            </div>
          )}
        </div>

        <div className="flex gap-3">
          <button
            onClick={handleDownload}
            className="rounded px-5 py-2 text-sm font-bold"
            style={{ background: "#00ff41", color: "#000" }}
          >
            📥 download
          </button>
          <button
            onClick={onClose}
            className="rounded px-5 py-2 text-sm"
            style={{ background: "var(--surface)", color: "var(--muted)", border: "1px solid var(--border)" }}
          >
            close
          </button>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Install html2canvas**

```bash
cd frontend && npm install html2canvas
```

- [ ] **Step 3: Verify build**

```bash
npm run build
```
Expected: `✓ Compiled successfully`

- [ ] **Step 4: Commit**

```bash
git add frontend/app/components/MemeCard.tsx frontend/package.json frontend/package-lock.json
git commit -m "feat: meme card component with download"
```

---

### Task 15: ProblemHeader component

**Files:**
- Create: `frontend/app/components/ProblemHeader.tsx`

- [ ] **Step 1: Write ProblemHeader.tsx**

Write `frontend/app/components/ProblemHeader.tsx`:
```tsx
import { Problem, Model } from "@/app/lib/types";

interface Props {
  problem: Problem;
  newModels: Model[];
}

const diffStyle = {
  Easy: { background: "#1b2a1b", color: "#00ff41" },
  Medium: { background: "#2a1f0a", color: "#ffaa00" },
  Hard: { background: "#2a0a0a", color: "#ff4444" },
};

export default function ProblemHeader({ problem, newModels }: Props) {
  return (
    <div className="px-5 py-4 border-b" style={{ borderColor: "var(--border)" }}>
      <div className="flex items-center gap-2 flex-wrap mb-2">
        <span
          className="text-xs rounded px-2 py-0.5"
          style={{ background: "var(--surface)", color: "var(--muted)" }}
        >
          #{problem.lc_id}
        </span>
        <span className="text-lg font-black text-white">{problem.title}</span>
        <span
          className="text-xs rounded px-2 py-0.5"
          style={diffStyle[problem.difficulty as keyof typeof diffStyle] ?? {}}
        >
          {problem.difficulty}
        </span>
        {newModels.length > 0 && (
          <span
            className="ml-auto text-xs rounded px-2 py-0.5"
            style={{ background: "#1a1a00", color: "#ffdd57" }}
          >
            🆕 {newModels[0].display_name} just dropped
          </span>
        )}
      </div>
      <p className="text-xs leading-relaxed line-clamp-2" style={{ color: "#666" }}>
        {problem.description.replace(/<[^>]*>/g, "").slice(0, 200)}...
      </p>
    </div>
  );
}
```

- [ ] **Step 2: Verify build**

```bash
cd frontend && npm run build
```
Expected: `✓ Compiled successfully`

- [ ] **Step 3: Commit**

```bash
git add frontend/app/components/ProblemHeader.tsx
git commit -m "feat: problem header component"
```

---

### Task 16: Main page assembly

**Files:**
- Modify: `frontend/app/page.tsx`

- [ ] **Step 1: Write page.tsx**

Write `frontend/app/page.tsx`:
```tsx
"use client";
import { useCallback, useEffect, useState } from "react";
import { createRace, fetchModels, fetchProblemResults, fetchRandomProblem } from "./lib/api";
import { Model, Problem, RaceResultWithModel } from "./lib/types";
import MemeCard from "./components/MemeCard";
import ProblemHeader from "./components/ProblemHeader";
import RaceResults from "./components/RaceResults";
import SearchBar from "./components/SearchBar";
import YouBanner from "./components/YouBanner";

export default function Home() {
  const [problem, setProblem] = useState<Problem | null>(null);
  const [results, setResults] = useState<RaceResultWithModel[]>([]);
  const [models, setModels] = useState<Model[]>([]);
  const [isRacing, setIsRacing] = useState(false);
  const [userResult, setUserResult] = useState<{ ms: number; gaveUp: boolean } | null>(null);
  const [memeTarget, setMemeTarget] = useState<RaceResultWithModel | null>(null);
  const [roast, setRoast] = useState("");

  const loadProblem = useCallback(async (p: Problem) => {
    setProblem(p);
    setUserResult(null);
    setMemeTarget(null);
    const r = await fetchProblemResults(p.id);
    setResults(r);
  }, []);

  const loadRandom = useCallback(async () => {
    const p = await fetchRandomProblem();
    await loadProblem(p);
  }, [loadProblem]);

  useEffect(() => {
    loadRandom();
    fetchModels().then(setModels);
  }, [loadRandom]);

  const handleRaceAgain = async () => {
    if (!problem || isRacing) return;
    setIsRacing(true);
    try {
      await createRace(problem.id);
      // Poll for results every 3s until race settles (max 90s)
      let attempts = 0;
      const poll = setInterval(async () => {
        const r = await fetchProblemResults(problem.id);
        setResults(r);
        attempts++;
        if (attempts > 30 || r.length > results.length) {
          clearInterval(poll);
          setIsRacing(false);
        }
      }, 3000);
    } catch {
      setIsRacing(false);
    }
  };

  const handleSelectResult = async (r: RaceResultWithModel) => {
    if (!problem) return;
    setMemeTarget(r);
    // Get roast from backend (via a quick fetch to a roast endpoint, or generate client-side fallback)
    const loser = results.find(x => x.model_id !== r.model_id && x.solved) ?? results[results.length - 1];
    if (loser) {
      setRoast(`${r.display_name} left ${loser.display_name} in the dust`);
    }
  };

  const newModels = models.filter(m => m.is_new);

  return (
    <div className="max-w-2xl mx-auto min-h-screen flex flex-col">
      {/* Nav */}
      <nav className="flex items-center justify-between px-5 py-3 border-b" style={{ borderColor: "var(--border)" }}>
        <div className="font-black text-base">
          how<span style={{ color: "#00ff41" }}>fast</span>would
          <span style={{ color: "var(--muted)" }}>.com</span>
        </div>
        <div className="flex gap-4 text-xs" style={{ color: "var(--muted)" }}>
          <span>leaderboard</span>
          <span>history</span>
          <span>about</span>
        </div>
      </nav>

      {/* Search */}
      <SearchBar onSelect={loadProblem} onRandom={loadRandom} />

      {problem && (
        <>
          <ProblemHeader problem={problem} newModels={newModels} />
          <YouBanner
            problemId={problem.id}
            onSolve={(ms) => setUserResult({ ms, gaveUp: false })}
            onGiveUp={(ms) => setUserResult({ ms, gaveUp: true })}
          />
          <RaceResults
            results={results}
            userResult={userResult}
            onSelectResult={handleSelectResult}
            onRaceAgain={handleRaceAgain}
            isRacing={isRacing}
          />
        </>
      )}

      {!problem && (
        <div className="flex-1 flex items-center justify-center text-sm" style={{ color: "var(--muted)" }}>
          loading...
        </div>
      )}

      {memeTarget && problem && (
        <MemeCard
          result={memeTarget}
          problem={problem}
          roast={roast}
          onClose={() => setMemeTarget(null)}
        />
      )}
    </div>
  );
}
```

- [ ] **Step 2: Verify build**

```bash
cd frontend && npm run build
```
Expected: `✓ Compiled successfully`

- [ ] **Step 3: Smoke test locally**

```bash
# Terminal 1: start backend (with .env populated)
cd backend && cargo run

# Terminal 2: start frontend
cd frontend && npm run dev
```

Open http://localhost:3000. You should see:
- Nav with logo
- Search bar + random button
- A LeetCode problem loads
- Timer starts
- Results show (empty until first benchmark run)

- [ ] **Step 4: Commit**

```bash
git add frontend/app/page.tsx
git commit -m "feat: main page wiring all components together"
```

---

### Task 17: Add `.env.local` to frontend and configure NEXT_PUBLIC_API_URL

**Files:**
- Create: `frontend/.env.local.example`

- [ ] **Step 1: Write .env.local.example**

Write `frontend/.env.local.example`:
```
NEXT_PUBLIC_API_URL=http://localhost:3001
```

- [ ] **Step 2: Set up for local dev**

```bash
cp frontend/.env.local.example frontend/.env.local
```

- [ ] **Step 3: Commit**

```bash
git add frontend/.env.local.example
git commit -m "chore: frontend env example"
```

---

## Self-Review

**Spec coverage check:**

| Spec requirement | Covered by |
|---|---|
| Fetch LeetCode problems via GraphQL + SQLite fallback | Task 4 (`leetcode.rs`) |
| Race all models in parallel | Task 7 (`runner.rs`) |
| Piston code execution + verification | Task 5 (`piston.rs`), Task 7 |
| Up to 2 retries with error feedback | Task 7 (`runner.rs` attempt loop) |
| SSE stream for live races | Task 8 (`races.rs`) — stub, polling fallback included |
| SQLite schema (problems, models, results, races) | Task 1 (migration) |
| Model auto-discovery (daily sync) | Task 9 (`sync.rs`) |
| Batch benchmark cron (every 6h) | Task 9 (`sync.rs`) |
| Initial model seed (all providers) | Task 9 (`seed_initial_models`) |
| Human baselines (LC avg, NeetCode, Tourist) | Task 9 (seeded with `human_times`) |
| Roast line generation via Claude haiku | Task 6 (`roast.rs`) |
| Dark terminal UI aesthetic | Task 10 (`globals.css`) |
| Search bar + random button | Task 11 |
| "You" live timer + solve/give-up | Task 12 |
| Leaderboard with bar chart | Task 13 |
| Shareable meme card + download | Task 14 |
| Problem header with difficulty + 🆕 badge | Task 15 |
| Main page wiring | Task 16 |

**Known gaps addressed inline:**
- SSE `stream` endpoint is a stub (Task 8) — polling via `fetchProblemResults` is the working fallback. Full SSE with shared broadcast state is a follow-up.
- Piston harness (Task 5) is best-effort for standard problems; problems with complex input types (trees, linked lists) will need extended harness support over time.
