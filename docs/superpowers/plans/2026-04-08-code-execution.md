# Code Execution & Verification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add real code execution for users (run/submit against Judge0) and execution visibility for AI results (expandable details showing generated code, per-test-case results, stderr).

**Architecture:** Two new tables (`submissions`, `execution_details`), three new API endpoints (`POST /run`, `POST /submit`, `GET /results/:id/details`), an in-memory rate limiter, runner changes to capture execution detail, and frontend updates to RaceEditor (run/submit buttons + test results panel) and RaceResults (expandable detail rows).

**Tech Stack:** Rust/Axum, SQLx/Postgres, Judge0 CE, Next.js 16/React 19, TypeScript, Tailwind 4

---

## File Structure

**Backend — create:**
- `backend/migrations/003_execution.sql` — new `submissions` and `execution_details` tables
- `backend/src/routes/execution.rs` — `POST /run`, `POST /submit`, `GET /results/:id/details` handlers
- `backend/src/rate_limit.rs` — in-memory IP rate limiter

**Backend — modify:**
- `backend/Cargo.toml` — add `dashmap` and `sha2` dependencies
- `backend/src/models.rs` — add `TestCaseResult`, `ExecutionDetail`, `Submission`, request/response structs
- `backend/src/runner.rs` — add `verify_with_detail()` method, update `race_one()` to capture execution details
- `backend/src/routes/mod.rs` — register new routes, add rate limiter to `AppState`
- `backend/src/routes/races.rs` — update race handler to store execution details
- `backend/src/main.rs` — no changes needed (rate limiter lives in AppState)

**Frontend — create:**
- `frontend/app/components/TestResultsPanel.tsx` — per-test-case pass/fail display
- `frontend/app/components/ResultDetail.tsx` — expanded AI result detail (code + test results + stderr)

**Frontend — modify:**
- `frontend/app/lib/types.ts` — add `TestCaseResult`, `RunResult`, `SubmitResult`, `ExecutionDetails`, `Submission` types
- `frontend/app/lib/api.ts` — add `runCode()`, `submitCode()`, `fetchResultDetails()`, `fetchSubmissions()`
- `frontend/app/components/RaceEditor.tsx` — add Run/Submit buttons, test results panel, attempt counter, rate limit UI
- `frontend/app/components/RaceResults.tsx` — add expand/collapse chevron, fetch+show execution details
- `frontend/app/page.tsx` — wire up submit flow to persist user results alongside AI results

---

### Task 1: Database Migration

**Files:**
- Create: `backend/migrations/003_execution.sql`

- [ ] **Step 1: Write the migration file**

```sql
-- backend/migrations/003_execution.sql

CREATE TABLE submissions (
    id           TEXT PRIMARY KEY,
    problem_id   TEXT NOT NULL REFERENCES problems(id),
    ip_hash      TEXT NOT NULL,
    solved       BOOLEAN NOT NULL DEFAULT false,
    time_ms      BIGINT,
    attempts     BIGINT NOT NULL DEFAULT 1,
    code         TEXT NOT NULL,
    submitted_at TEXT NOT NULL
);

CREATE INDEX idx_submissions_problem_id ON submissions(problem_id);

CREATE TABLE execution_details (
    id           TEXT PRIMARY KEY,
    result_id    TEXT NOT NULL REFERENCES results(id) ON DELETE CASCADE,
    code         TEXT NOT NULL,
    test_results TEXT NOT NULL,
    stderr       TEXT NOT NULL DEFAULT ''
);

CREATE INDEX idx_execution_details_result_id ON execution_details(result_id);
```

- [ ] **Step 2: Verify migration compiles**

Run from `backend/`:
```bash
cargo sqlx migrate run
```
Expected: migration 003 applied successfully.

- [ ] **Step 3: Regenerate sqlx offline data**

```bash
cargo sqlx prepare
```

- [ ] **Step 4: Commit**

```bash
git add backend/migrations/003_execution.sql backend/.sqlx/
git commit -m "feat: add submissions and execution_details tables"
```

---

### Task 2: Backend Models & Types

**Files:**
- Modify: `backend/src/models.rs`

- [ ] **Step 1: Add new structs to models.rs**

Add these structs at the end of `backend/src/models.rs`, before any `#[cfg(test)]` block:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseResult {
    pub input: String,
    pub expected: String,
    pub got: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExecutionDetail {
    pub id: String,
    pub result_id: String,
    pub code: String,
    pub test_results: String, // JSON: Vec<TestCaseResult>
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Submission {
    pub id: String,
    pub problem_id: String,
    pub ip_hash: String,
    pub solved: bool,
    pub time_ms: Option<i64>,
    pub attempts: i64,
    pub code: String,
    pub submitted_at: String,
}

// -- Request/response types for execution endpoints --

#[derive(Debug, Deserialize)]
pub struct RunCodeRequest {
    pub code: String,
    pub problem_id: String,
}

#[derive(Debug, Serialize)]
pub struct RunCodeResponse {
    pub passed: bool,
    pub results: Vec<TestCaseResult>,
    pub stderr: String,
}

#[derive(Debug, Deserialize)]
pub struct SubmitCodeRequest {
    pub code: String,
    pub problem_id: String,
    pub time_ms: i64,
    pub attempts: i64,
}

#[derive(Debug, Serialize)]
pub struct SubmitCodeResponse {
    pub passed: bool,
    pub results: Vec<TestCaseResult>,
    pub submission_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExecutionDetailResponse {
    pub code: String,
    pub test_results: Vec<TestCaseResult>,
    pub stderr: String,
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo check
```
Expected: compiles with no errors.

- [ ] **Step 3: Commit**

```bash
git add backend/src/models.rs
git commit -m "feat: add execution and submission model types"
```

---

### Task 3: Rate Limiter

**Files:**
- Create: `backend/src/rate_limit.rs`
- Modify: `backend/Cargo.toml`

- [ ] **Step 1: Add dashmap dependency**

Add to `[dependencies]` in `backend/Cargo.toml`:

```toml
dashmap = "6"
```

- [ ] **Step 2: Write rate_limit.rs with test**

Create `backend/src/rate_limit.rs`:

```rust
use dashmap::DashMap;
use std::net::IpAddr;
use std::time::Instant;

/// In-memory rate limiter: max_requests per window_secs, keyed by IP.
pub struct RateLimiter {
    map: DashMap<IpAddr, (u32, Instant)>,
    max_requests: u32,
    window_secs: u64,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            map: DashMap::new(),
            max_requests,
            window_secs,
        }
    }

    /// Returns Ok(()) if allowed, Err(seconds_until_reset) if rate-limited.
    pub fn check(&self, ip: IpAddr) -> Result<(), u64> {
        let now = Instant::now();
        let mut entry = self.map.entry(ip).or_insert((0, now));
        let (count, window_start) = entry.value_mut();

        // Reset window if expired
        if now.duration_since(*window_start).as_secs() >= self.window_secs {
            *count = 0;
            *window_start = now;
        }

        if *count >= self.max_requests {
            let elapsed = now.duration_since(*window_start).as_secs();
            let retry_after = self.window_secs.saturating_sub(elapsed);
            return Err(retry_after);
        }

        *count += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_allows_under_limit() {
        let limiter = RateLimiter::new(3, 60);
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        assert!(limiter.check(ip).is_ok());
        assert!(limiter.check(ip).is_ok());
        assert!(limiter.check(ip).is_ok());
    }

    #[test]
    fn test_blocks_over_limit() {
        let limiter = RateLimiter::new(2, 60);
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        assert!(limiter.check(ip).is_ok());
        assert!(limiter.check(ip).is_ok());
        assert!(limiter.check(ip).is_err());
    }

    #[test]
    fn test_different_ips_independent() {
        let limiter = RateLimiter::new(1, 60);
        let ip1 = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));
        let ip2 = IpAddr::V4(Ipv4Addr::new(5, 6, 7, 8));
        assert!(limiter.check(ip1).is_ok());
        assert!(limiter.check(ip2).is_ok());
        assert!(limiter.check(ip1).is_err());
        assert!(limiter.check(ip2).is_err());
    }
}
```

- [ ] **Step 3: Register the module in main.rs**

Add `mod rate_limit;` to the top of `backend/src/main.rs`, after the existing module declarations:

```rust
mod rate_limit;
```

- [ ] **Step 4: Run tests**

```bash
cargo test rate_limit
```
Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add backend/Cargo.toml backend/src/rate_limit.rs backend/src/main.rs
git commit -m "feat: add in-memory IP rate limiter"
```

---

### Task 4: Runner — verify_with_detail()

**Files:**
- Modify: `backend/src/runner.rs`

- [ ] **Step 1: Add verify_with_detail method**

Add this method to the `impl Runner` block in `backend/src/runner.rs`, right after the existing `verify()` method (around line 170):

```rust
    /// Like verify(), but returns per-test-case detail instead of just pass/fail.
    pub async fn verify_with_detail(
        &self,
        code: &str,
        test_cases: &[TestCase],
    ) -> Result<(bool, Vec<crate::models::TestCaseResult>, String)> {
        let mut all_passed = true;
        let mut case_results = Vec::new();
        let mut last_stderr = String::new();

        if test_cases.is_empty() {
            return Ok((true, case_results, last_stderr));
        }

        for tc in test_cases {
            let wrapped = wrap_solution(code, &tc.input);
            match self.piston.run_python(&wrapped, &tc.input).await {
                Ok(run) => {
                    let got = run.stdout.trim().to_string();
                    let expected = tc.expected_output.trim().to_string();
                    let passed = run.code == 0
                        && (expected.is_empty() || got == expected);

                    if !passed {
                        all_passed = false;
                    }
                    if !run.stderr.is_empty() {
                        last_stderr = run.stderr.clone();
                    }
                    case_results.push(crate::models::TestCaseResult {
                        input: tc.input.clone(),
                        expected,
                        got,
                        passed,
                    });
                }
                Err(e) => {
                    all_passed = false;
                    last_stderr = e.to_string();
                    case_results.push(crate::models::TestCaseResult {
                        input: tc.input.clone(),
                        expected: tc.expected_output.trim().to_string(),
                        got: String::new(),
                        passed: false,
                    });
                }
            }
        }

        Ok((all_passed, case_results, last_stderr))
    }
```

- [ ] **Step 2: Make PistonClient and wrap_solution public for use by execution routes**

Verify that `PistonClient` is already `pub struct` and `wrap_solution` is `pub fn` in `backend/src/piston.rs`. They already are — no changes needed.

Verify that `Runner`'s `piston` field and `config` field can be accessed. They are private, but `verify_with_detail()` is a method on `Runner` so it has access. The execution routes will call `runner.verify_with_detail()` directly. No changes needed.

- [ ] **Step 3: Verify it compiles**

```bash
cargo check
```
Expected: compiles with no errors.

- [ ] **Step 4: Commit**

```bash
git add backend/src/runner.rs
git commit -m "feat: add verify_with_detail() to runner"
```

---

### Task 5: Execution Route Handlers

**Files:**
- Create: `backend/src/routes/execution.rs`
- Modify: `backend/src/routes/mod.rs`
- Modify: `backend/Cargo.toml`

- [ ] **Step 1: Add sha2 dependency**

Add to `[dependencies]` in `backend/Cargo.toml`:

```toml
sha2 = "0.10"
```

- [ ] **Step 2: Create execution.rs with all three handlers**

Create `backend/src/routes/execution.rs`:

```rust
use axum::{
    extract::{ConnectInfo, Path, State},
    http::StatusCode,
    Json,
};
use sha2::{Sha256, Digest};
use std::net::SocketAddr;
use uuid::Uuid;
use chrono::Utc;
use crate::{
    models::{
        ExecutionDetailResponse, Problem, RunCodeRequest, RunCodeResponse,
        SubmitCodeRequest, SubmitCodeResponse, TestCase, TestCaseResult,
    },
    piston::wrap_solution,
    routes::AppState,
};

/// POST /run — execute user code against test cases, return results (no persistence).
pub async fn run_code(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(body): Json<RunCodeRequest>,
) -> Result<Json<RunCodeResponse>, StatusCode> {
    // Rate limit check
    if let Err(retry_after) = state.rate_limiter.check(addr.ip()) {
        tracing::warn!("rate limited {} (retry in {}s)", addr.ip(), retry_after);
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    let problem = sqlx::query_as!(
        Problem,
        "SELECT * FROM problems WHERE id = $1",
        body.problem_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let test_cases: Vec<TestCase> =
        serde_json::from_str(&problem.test_cases).unwrap_or_default();

    let (passed, results, stderr) = state
        .runner
        .verify_with_detail(&body.code, &test_cases)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RunCodeResponse {
        passed,
        results,
        stderr,
    }))
}

/// POST /submit — execute user code and persist if all tests pass.
pub async fn submit_code(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(body): Json<SubmitCodeRequest>,
) -> Result<Json<SubmitCodeResponse>, StatusCode> {
    // Rate limit check
    if let Err(retry_after) = state.rate_limiter.check(addr.ip()) {
        tracing::warn!("rate limited {} (retry in {}s)", addr.ip(), retry_after);
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    // Validate time_ms: > 0 and < 1 hour
    if body.time_ms <= 0 || body.time_ms > 3_600_000 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let problem = sqlx::query_as!(
        Problem,
        "SELECT * FROM problems WHERE id = $1",
        body.problem_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let test_cases: Vec<TestCase> =
        serde_json::from_str(&problem.test_cases).unwrap_or_default();

    let (passed, results, _stderr) = state
        .runner
        .verify_with_detail(&body.code, &test_cases)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut submission_id = None;

    if passed {
        let id = Uuid::new_v4().to_string();
        let ip_hash = format!("{:x}", Sha256::digest(addr.ip().to_string().as_bytes()));
        let now = Utc::now().to_rfc3339();

        sqlx::query!(
            r#"INSERT INTO submissions (id, problem_id, ip_hash, solved, time_ms, attempts, code, submitted_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
            id,
            body.problem_id,
            ip_hash,
            true,
            body.time_ms,
            body.attempts,
            body.code,
            now
        )
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        submission_id = Some(id);
    }

    Ok(Json(SubmitCodeResponse {
        passed,
        results,
        submission_id,
    }))
}

/// GET /results/:result_id/details — fetch execution details for an AI benchmark result.
pub async fn result_details(
    State(state): State<AppState>,
    Path(result_id): Path<String>,
) -> Result<Json<ExecutionDetailResponse>, StatusCode> {
    let detail = sqlx::query_as!(
        crate::models::ExecutionDetail,
        "SELECT * FROM execution_details WHERE result_id = $1",
        result_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let test_results: Vec<TestCaseResult> =
        serde_json::from_str(&detail.test_results).unwrap_or_default();

    Ok(Json(ExecutionDetailResponse {
        code: detail.code,
        test_results,
        stderr: detail.stderr,
    }))
}
```

- [ ] **Step 3: Update routes/mod.rs — add rate limiter to AppState, register routes**

In `backend/src/routes/mod.rs`:

Add `pub mod execution;` to the module declarations at the top:

```rust
pub mod execution;
pub mod models;
pub mod problems;
pub mod races;
```

Add the rate limiter import and field to `AppState`:

```rust
use std::sync::Arc;
use crate::rate_limit::RateLimiter;
```

Add to the `AppState` struct:

```rust
pub rate_limiter: Arc<RateLimiter>,
```

In the `router()` function, create the rate limiter and add it to state:

```rust
let rate_limiter = Arc::new(RateLimiter::new(10, 60));
let state = AppState { pool, config, runner, benchmarks_in_flight, rate_limiter };
```

Add the new routes to the router chain:

```rust
.route("/run", post(execution::run_code))
.route("/submit", post(execution::submit_code))
.route("/results/:id/details", get(execution::result_details))
```

- [ ] **Step 4: Update main.rs for ConnectInfo**

The `ConnectInfo<SocketAddr>` extractor requires `.into_make_service_with_connect_info::<SocketAddr>()` on the serve call. In `backend/src/main.rs`, change the last line from:

```rust
axum::serve(listener, app).await?;
```

to:

```rust
axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>()).await?;
```

- [ ] **Step 5: Verify it compiles**

```bash
cargo check
```
Expected: compiles with no errors.

- [ ] **Step 6: Commit**

```bash
git add backend/Cargo.toml backend/src/routes/execution.rs backend/src/routes/mod.rs backend/src/main.rs
git commit -m "feat: add /run, /submit, /results/:id/details endpoints"
```

---

### Task 6: Runner — Store Execution Details During Benchmarks

**Files:**
- Modify: `backend/src/runner.rs`
- Modify: `backend/src/routes/races.rs`

- [ ] **Step 1: Update race_one to capture code and test detail**

In `backend/src/runner.rs`, modify `race_one()` to track the last generated code and use `verify_with_detail()`:

Replace the body of `race_one()` (lines 77-131) with:

```rust
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
        let mut last_code = String::new();
        let mut last_test_results: Vec<crate::models::TestCaseResult> = Vec::new();
        let mut last_stderr = String::new();

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

            last_code = code.clone();

            match self.verify_with_detail(&code, test_cases).await {
                Ok((passed, results, stderr)) => {
                    last_test_results = results;
                    last_stderr = stderr;
                    if passed {
                        solved = true;
                        break;
                    } else {
                        last_error = "wrong answer".into();
                    }
                }
                Err(e) => {
                    last_error = e.to_string();
                    last_stderr = e.to_string();
                }
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
            last_code,
            last_test_results: serde_json::to_string(&last_test_results).unwrap_or_default(),
            last_stderr,
        }
    }
```

- [ ] **Step 2: Add execution detail fields to RaceResult**

In `backend/src/models.rs`, add three fields to the `RaceResult` struct. These are NOT stored in the `results` table — they're transient fields used to pass data to the execution_details insert. Mark them with `#[sqlx(skip)]`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RaceResult {
    pub id: String,
    pub problem_id: String,
    pub model_id: String,
    pub solved: bool,
    pub time_ms: Option<i64>,
    pub attempts: i64,
    pub run_at: String,
    #[sqlx(skip)]
    #[serde(skip)]
    pub last_code: String,
    #[sqlx(skip)]
    #[serde(skip)]
    pub last_test_results: String,
    #[sqlx(skip)]
    #[serde(skip)]
    pub last_stderr: String,
}
```

- [ ] **Step 3: Store execution details in races.rs after saving results**

In `backend/src/routes/races.rs`, inside the `tokio::spawn` block that saves results (around line 72), add after the `results` insert:

```rust
            // Store execution details
            let detail_id = uuid::Uuid::new_v4().to_string();
            sqlx::query!(
                r#"INSERT INTO execution_details (id, result_id, code, test_results, stderr)
                   VALUES ($1, $2, $3, $4, $5)
                   ON CONFLICT (result_id) DO UPDATE SET
                       code = EXCLUDED.code,
                       test_results = EXCLUDED.test_results,
                       stderr = EXCLUDED.stderr"#,
                detail_id,
                result.id,
                result.last_code,
                result.last_test_results,
                result.last_stderr
            ).execute(&pool).await.ok();
```

Wait — `execution_details` doesn't have a UNIQUE constraint on `result_id`, so `ON CONFLICT` won't work. Use a simple INSERT instead (one execution detail per result, no upsert needed):

```rust
            let detail_id = uuid::Uuid::new_v4().to_string();
            if !result.last_code.is_empty() {
                sqlx::query!(
                    r#"INSERT INTO execution_details (id, result_id, code, test_results, stderr)
                       VALUES ($1, $2, $3, $4, $5)"#,
                    detail_id,
                    result.id,
                    result.last_code,
                    result.last_test_results,
                    result.last_stderr
                ).execute(&pool).await.ok();
            }
```

- [ ] **Step 4: Also store execution details in the on-demand benchmark path**

In `backend/src/routes/problems.rs`, inside the `tokio::spawn` block that runs on-demand benchmarks (around line 111-125), add the same execution detail insert after each result insert. The block currently looks like:

```rust
for result in &bench_results {
    sqlx::query!(/* ... insert into results ... */).execute(&pool).await.ok();
}
```

Add after each result insert:

```rust
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

    if !result.last_code.is_empty() {
        let detail_id = uuid::Uuid::new_v4().to_string();
        sqlx::query!(
            r#"INSERT INTO execution_details (id, result_id, code, test_results, stderr)
               VALUES ($1, $2, $3, $4, $5)"#,
            detail_id,
            result.id,
            result.last_code,
            result.last_test_results,
            result.last_stderr
        ).execute(&pool).await.ok();
    }
}
```

- [ ] **Step 5: Verify it compiles**

```bash
cargo check
```
Expected: compiles with no errors.

- [ ] **Step 6: Regenerate sqlx offline data**

```bash
cargo sqlx prepare
```

- [ ] **Step 7: Commit**

```bash
git add backend/src/runner.rs backend/src/models.rs backend/src/routes/races.rs backend/src/routes/problems.rs backend/.sqlx/
git commit -m "feat: capture and store execution details during benchmarks"
```

---

### Task 7: Frontend Types & API Client

**Files:**
- Modify: `frontend/app/lib/types.ts`
- Modify: `frontend/app/lib/api.ts`

- [ ] **Step 1: Add types**

Add to the end of `frontend/app/lib/types.ts`:

```typescript
export interface TestCaseResult {
  input: string;
  expected: string;
  got: string;
  passed: boolean;
}

export interface RunResult {
  passed: boolean;
  results: TestCaseResult[];
  stderr: string;
}

export interface SubmitResult {
  passed: boolean;
  results: TestCaseResult[];
  submission_id: string | null;
}

export interface ExecutionDetails {
  code: string;
  test_results: TestCaseResult[];
  stderr: string;
}

export interface Submission {
  id: string;
  problem_id: string;
  ip_hash: string;
  solved: boolean;
  time_ms: number | null;
  attempts: number;
  code: string;
  submitted_at: string;
}
```

- [ ] **Step 2: Add API functions**

Add to the end of `frontend/app/lib/api.ts`:

```typescript
import { ExecutionDetails, RunResult, SubmitResult, Submission } from "./types";
```

Wait — the file already imports from `"./types"`. Merge the new types into the existing import at line 1:

```typescript
import { CreateRaceResponse, ExecutionDetails, LeaderboardEntry, Model, Problem, RaceResultWithModel, RunResult, Submission, SubmitResult } from "./types";
```

Then add these functions at the end of the file:

```typescript
export async function runCode(code: string, problemId: string): Promise<RunResult> {
  const res = await fetch(`${BASE}/run`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ code, problem_id: problemId }),
  });
  if (res.status === 429) {
    const retryAfter = parseInt(res.headers.get("Retry-After") ?? "60", 10);
    throw new RateLimitError(retryAfter);
  }
  if (!res.ok) throw new Error("failed to run code");
  return res.json();
}

export async function submitCode(
  code: string,
  problemId: string,
  timeMs: number,
  attempts: number
): Promise<SubmitResult> {
  const res = await fetch(`${BASE}/submit`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ code, problem_id: problemId, time_ms: timeMs, attempts }),
  });
  if (res.status === 429) {
    const retryAfter = parseInt(res.headers.get("Retry-After") ?? "60", 10);
    throw new RateLimitError(retryAfter);
  }
  if (!res.ok) throw new Error("failed to submit code");
  return res.json();
}

export async function fetchResultDetails(resultId: string): Promise<ExecutionDetails> {
  const res = await fetch(`${BASE}/results/${resultId}/details`);
  if (!res.ok) throw new Error("no execution details");
  return res.json();
}

export async function fetchSubmissions(problemId: string): Promise<Submission[]> {
  const res = await fetch(`${BASE}/problems/${problemId}/submissions`);
  if (!res.ok) return [];
  return res.json();
}

export class RateLimitError extends Error {
  retryAfter: number;
  constructor(retryAfter: number) {
    super(`Rate limited. Try again in ${retryAfter}s`);
    this.retryAfter = retryAfter;
  }
}
```

- [ ] **Step 3: Verify it compiles**

```bash
cd frontend && npx tsc --noEmit
```
Expected: no type errors.

- [ ] **Step 4: Commit**

```bash
git add frontend/app/lib/types.ts frontend/app/lib/api.ts
git commit -m "feat: add execution types and API client functions"
```

---

### Task 8: TestResultsPanel Component

**Files:**
- Create: `frontend/app/components/TestResultsPanel.tsx`

- [ ] **Step 1: Create the component**

Create `frontend/app/components/TestResultsPanel.tsx`:

```tsx
"use client";
import { TestCaseResult } from "@/app/lib/types";

interface Props {
  results: TestCaseResult[];
  stderr: string;
}

export default function TestResultsPanel({ results, stderr }: Props) {
  if (results.length === 0) return null;

  const passCount = results.filter((r) => r.passed).length;
  const failCount = results.length - passCount;

  return (
    <div
      className="border-t"
      style={{ background: "var(--surface-2)", borderColor: "var(--border)" }}
    >
      {/* Summary */}
      <div
        className="flex items-center justify-between px-4 py-2 border-b"
        style={{ borderColor: "var(--border)" }}
      >
        <span className="text-xs font-semibold" style={{ color: "var(--text)" }}>
          Test Results
        </span>
        <span className="text-xs">
          <span style={{ color: "var(--green, #00d4aa)" }}>{passCount} passed</span>
          {failCount > 0 && (
            <>
              {" · "}
              <span style={{ color: "var(--red)" }}>{failCount} failed</span>
            </>
          )}
        </span>
      </div>

      {/* Per-case results */}
      <div className="px-4 py-2 flex flex-col gap-2 max-h-48 overflow-y-auto">
        {results.map((tc, i) => (
          <div
            key={i}
            className="flex items-start gap-2 text-xs"
            style={{ fontFamily: "'Courier New', monospace" }}
          >
            <span
              className="flex-shrink-0 mt-0.5"
              style={{ color: tc.passed ? "var(--green, #00d4aa)" : "var(--red)" }}
            >
              {tc.passed ? "✓" : "✗"}
            </span>
            <div className="flex-1 min-w-0">
              <div style={{ color: "var(--muted)" }}>
                <span>Input: </span>
                <span style={{ color: "var(--text)" }}>{tc.input}</span>
              </div>
              <div style={{ color: "var(--muted)" }}>
                <span>Expected: </span>
                <span style={{ color: "var(--green, #00d4aa)" }}>{tc.expected}</span>
              </div>
              {!tc.passed && (
                <div style={{ color: "var(--muted)" }}>
                  <span>Got: </span>
                  <span
                    style={{
                      color: "var(--red)",
                      textDecoration: "line-through",
                    }}
                  >
                    {tc.got || "(empty)"}
                  </span>
                </div>
              )}
            </div>
          </div>
        ))}
      </div>

      {/* Stderr */}
      {stderr && (
        <div
          className="mx-4 mb-2 px-3 py-2 rounded text-xs"
          style={{
            background: "rgba(239,71,67,0.08)",
            border: "1px solid rgba(239,71,67,0.2)",
            color: "var(--red)",
            fontFamily: "'Courier New', monospace",
            whiteSpace: "pre-wrap",
            maxHeight: "6rem",
            overflowY: "auto",
          }}
        >
          {stderr}
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add frontend/app/components/TestResultsPanel.tsx
git commit -m "feat: add TestResultsPanel component"
```

---

### Task 9: RaceEditor — Run & Submit Buttons

**Files:**
- Modify: `frontend/app/components/RaceEditor.tsx`

- [ ] **Step 1: Add imports and state**

At the top of `frontend/app/components/RaceEditor.tsx`, add to the imports:

```typescript
import { runCode, submitCode, RateLimitError } from "@/app/lib/api";
import { TestCaseResult } from "@/app/lib/types";
import TestResultsPanel from "./TestResultsPanel";
```

Inside the `RaceEditor` component function, add new state after the existing state declarations (around line 48-50):

```typescript
  const [testResults, setTestResults] = useState<TestCaseResult[]>([]);
  const [testStderr, setTestStderr] = useState("");
  const [runAttempts, setRunAttempts] = useState(0);
  const [isRunning, setIsRunning] = useState(false);
  const [rateLimitCountdown, setRateLimitCountdown] = useState(0);
```

- [ ] **Step 2: Add rate limit countdown effect**

Add this effect after the existing effects:

```typescript
  // Rate limit countdown timer
  useEffect(() => {
    if (rateLimitCountdown <= 0) return;
    const t = setInterval(() => {
      setRateLimitCountdown((c) => {
        if (c <= 1) { clearInterval(t); return 0; }
        return c - 1;
      });
    }, 1000);
    return () => clearInterval(t);
  }, [rateLimitCountdown]);
```

- [ ] **Step 3: Add Run and Submit handlers**

Add these handler functions after the existing `handleGiveUp`:

```typescript
  const handleRun = async () => {
    if (isRunning || rateLimitCountdown > 0) return;
    startRace(); // ensure timer is running
    setIsRunning(true);
    try {
      const result = await runCode(code, problem.id);
      setTestResults(result.results);
      setTestStderr(result.stderr);
      setRunAttempts((a) => a + 1);
    } catch (e) {
      if (e instanceof RateLimitError) {
        setRateLimitCountdown(e.retryAfter);
      }
    } finally {
      setIsRunning(false);
    }
  };

  const handleRunSubmit = async () => {
    if (isRunning || rateLimitCountdown > 0) return;
    setIsRunning(true);
    try {
      const ms = stop();
      setPhase("submitted");
      const result = await submitCode(code, problem.id, ms, runAttempts + 1);
      setTestResults(result.results);
      setTestStderr("");
      if (result.passed) {
        onSolve(ms);
      } else {
        onGiveUp(ms);
      }
    } catch (e) {
      if (e instanceof RateLimitError) {
        setRateLimitCountdown(e.retryAfter);
        // Un-submit on rate limit — let them try again
        setPhase("racing");
        start();
      }
    } finally {
      setIsRunning(false);
    }
  };
```

- [ ] **Step 4: Update the submit bar JSX**

Replace the existing submit bar div (the one containing the Submit button, around lines 262-287) with:

```tsx
      {/* Submit bar */}
      <div className="flex items-center gap-3 px-4 py-2.5 border-t" style={{ background: "var(--surface)", borderColor: "var(--border)" }}>
        <button
          onClick={handleRun}
          disabled={phase !== "racing" || isRunning || rateLimitCountdown > 0}
          className="rounded px-4 py-1.5 text-sm font-bold"
          style={{
            background: phase === "racing" && !isRunning ? "#3a3a3a" : "#2a2a2a",
            color: phase === "racing" && !isRunning ? "var(--text)" : "var(--muted)",
            cursor: phase === "racing" && !isRunning ? "pointer" : "not-allowed",
            border: "1px solid #4a4a4a",
          }}
        >
          {isRunning ? "Running…" : "▶ Run"}
        </button>
        <button
          onClick={handleRunSubmit}
          disabled={phase !== "racing" || isRunning || rateLimitCountdown > 0}
          className="rounded px-5 py-1.5 text-sm font-bold"
          style={{
            background: phase === "racing" && !isRunning ? "var(--orange)" : "#3a3a3a",
            color: phase === "racing" && !isRunning ? "#000" : "var(--muted)",
            cursor: phase === "racing" && !isRunning ? "pointer" : "not-allowed",
          }}
        >
          Submit
        </button>
        <span className="text-xs italic flex-1" style={{ color: "#555" }}>
          {rateLimitCountdown > 0
            ? `Try again in ${rateLimitCountdown}s`
            : phase === "submitted"
            ? "We told you so."
            : ""}
        </span>
        {phase === "racing" && (
          <button
            onClick={handleGiveUp}
            className="text-xs"
            style={{ color: "var(--muted)", background: "transparent", border: "none", cursor: "pointer" }}
          >
            give up
          </button>
        )}
      </div>

      {/* Test results panel */}
      {testResults.length > 0 && (
        <TestResultsPanel results={testResults} stderr={testStderr} />
      )}
```

- [ ] **Step 5: Reset test results when problem changes**

In the existing `useEffect` that resets on problem change (around line 55-62), add:

```typescript
    setTestResults([]);
    setTestStderr("");
    setRunAttempts(0);
    setRateLimitCountdown(0);
```

- [ ] **Step 6: Verify it compiles**

```bash
cd frontend && npx tsc --noEmit
```
Expected: no type errors.

- [ ] **Step 7: Commit**

```bash
git add frontend/app/components/RaceEditor.tsx
git commit -m "feat: add Run and Submit buttons with test results to RaceEditor"
```

---

### Task 10: ResultDetail Component

**Files:**
- Create: `frontend/app/components/ResultDetail.tsx`

- [ ] **Step 1: Create the component**

Create `frontend/app/components/ResultDetail.tsx`:

```tsx
"use client";
import { useEffect, useState } from "react";
import { fetchResultDetails } from "@/app/lib/api";
import { ExecutionDetails } from "@/app/lib/types";

interface Props {
  resultId: string;
}

export default function ResultDetail({ resultId }: Props) {
  const [details, setDetails] = useState<ExecutionDetails | null>(null);
  const [error, setError] = useState(false);

  useEffect(() => {
    let cancelled = false;
    fetchResultDetails(resultId)
      .then((d) => { if (!cancelled) setDetails(d); })
      .catch(() => { if (!cancelled) setError(true); });
    return () => { cancelled = true; };
  }, [resultId]);

  if (error) {
    return (
      <div className="px-5 py-3 text-xs" style={{ color: "var(--muted)" }}>
        No execution details available for this result.
      </div>
    );
  }

  if (!details) {
    return (
      <div className="px-5 py-3 text-xs" style={{ color: "var(--muted)" }}>
        Loading…
      </div>
    );
  }

  const passCount = details.test_results.filter((r) => r.passed).length;

  return (
    <div
      className="border-t"
      style={{ background: "#1a1a1a", borderColor: "var(--border)" }}
    >
      {/* Generated code */}
      <div className="px-5 py-3">
        <div
          className="text-xs font-semibold mb-2"
          style={{ color: "var(--muted)", letterSpacing: "0.1em", textTransform: "uppercase" }}
        >
          Generated Code
        </div>
        <pre
          className="text-xs rounded p-3 overflow-x-auto"
          style={{
            background: "#0d0d0d",
            border: "1px solid var(--border)",
            color: "var(--text)",
            fontFamily: "'Courier New', monospace",
            lineHeight: "1.6",
            maxHeight: "12rem",
            overflowY: "auto",
          }}
        >
          {details.code}
        </pre>
      </div>

      {/* Test cases */}
      <div className="px-5 pb-3">
        <div
          className="text-xs font-semibold mb-2"
          style={{ color: "var(--muted)", letterSpacing: "0.1em", textTransform: "uppercase" }}
        >
          Test Cases — {passCount}/{details.test_results.length} passed
        </div>
        <div className="flex flex-col gap-1.5">
          {details.test_results.map((tc, i) => (
            <div
              key={i}
              className="flex items-start gap-2 text-xs"
              style={{ fontFamily: "'Courier New', monospace" }}
            >
              <span
                className="flex-shrink-0"
                style={{ color: tc.passed ? "var(--green, #00d4aa)" : "var(--red)" }}
              >
                {tc.passed ? "✓" : "✗"}
              </span>
              <span style={{ color: "var(--muted)" }}>
                {tc.input} → {tc.passed ? (
                  <span style={{ color: "var(--green, #00d4aa)" }}>{tc.expected}</span>
                ) : (
                  <>
                    <span style={{ color: "var(--red)", textDecoration: "line-through" }}>
                      {tc.got || "(empty)"}
                    </span>
                    {" expected "}
                    <span style={{ color: "var(--green, #00d4aa)" }}>{tc.expected}</span>
                  </>
                )}
              </span>
            </div>
          ))}
        </div>
      </div>

      {/* Stderr */}
      {details.stderr && (
        <div
          className="mx-5 mb-3 px-3 py-2 rounded text-xs"
          style={{
            background: "rgba(239,71,67,0.08)",
            border: "1px solid rgba(239,71,67,0.2)",
            color: "var(--red)",
            fontFamily: "'Courier New', monospace",
            whiteSpace: "pre-wrap",
          }}
        >
          {details.stderr}
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add frontend/app/components/ResultDetail.tsx
git commit -m "feat: add ResultDetail component for expandable AI execution traces"
```

---

### Task 11: RaceResults — Expandable Detail Rows

**Files:**
- Modify: `frontend/app/components/RaceResults.tsx`

- [ ] **Step 1: Add imports and expand state**

At the top of `frontend/app/components/RaceResults.tsx`, add:

```typescript
import ResultDetail from "./ResultDetail";
```

The component already has `RaceResultWithModel` imported. We need to add a `result_id` field to `RaceResultWithModel` so we can fetch details. But the existing type doesn't have `id` — let me check. Looking at the results query in `problems.rs`, it selects `r.id` but the `RaceResultWithModel` struct doesn't include it.

We need to add `id` to `RaceResultWithModel`. In `backend/src/models.rs`:

```rust
pub struct RaceResultWithModel {
    pub id: String,        // ADD THIS FIELD
    pub model_id: String,
    // ... rest unchanged
}
```

And in `backend/src/routes/problems.rs`, in the `results()` function where `RaceResultWithModel` is constructed (around line 55), add:

```rust
    let results: Vec<RaceResultWithModel> = rows.into_iter().map(|r| RaceResultWithModel {
        id: r.id,  // ADD THIS LINE
        model_id: r.model_id,
        // ... rest unchanged
    }).collect();
```

And in `frontend/app/lib/types.ts`, add `id` to `RaceResultWithModel`:

```typescript
export interface RaceResultWithModel {
  id: string;  // ADD THIS FIELD
  model_id: string;
  // ... rest unchanged
}
```

- [ ] **Step 2: Update RaceResults to support expand/collapse**

Replace the existing `expanded` state (which currently toggles show more/less) with two separate states. Rename the existing one to avoid confusion:

Change the existing line:
```typescript
const [expanded, setExpanded] = useState(false);
```

To:
```typescript
const [showAll, setShowAll] = useState(false);
const [expandedId, setExpandedId] = useState<string | null>(null);
```

Update all references from `expanded` to `showAll`:
- `const visible = showAll ? withUser : withUser.slice(0, 5);`
- In the "show more" button: `onClick={() => setShowAll(e => !e)}`
- In the button text: `{showAll ? "show less ↑" : ...}`

- [ ] **Step 3: Add chevron and detail row to each AI result**

In the AI result rendering (the `<button>` element, around line 113-144), change it from a `<button>` to a `<div>` wrapper containing the button and the detail panel:

```tsx
          return (
            <div key={r.model_id}>
              <button
                className="w-full px-5 py-2.5 border-b text-left result-row"
                style={{ borderColor: "var(--border)", cursor: "pointer", transition: "background 0.15s ease" }}
                onClick={() => setExpandedId(expandedId === r.id ? null : r.id)}
              >
                <div className="flex items-center gap-2 mb-1.5">
                  <span className="text-xs w-4" style={{ color: "var(--muted)" }}>
                    {displayRank === 1 ? "🥇" : displayRank === 2 ? "🥈" : displayRank === 3 ? "🥉" : displayRank}
                  </span>
                  <span
                    className="text-sm font-semibold flex-1 truncate"
                    style={{ color: isWinner ? "var(--orange)" : "var(--text)" }}
                  >
                    {r.display_name}
                  </span>
                  <span
                    className="text-sm font-bold flex-shrink-0"
                    style={{ color: r.solved ? (isWinner ? "var(--orange)" : "var(--muted)") : "var(--red)" }}
                  >
                    {r.solved ? formatTime(r.time_ms) : "failed"}
                  </span>
                  <span
                    className="text-xs flex-shrink-0"
                    style={{
                      color: "var(--muted)",
                      transform: expandedId === r.id ? "rotate(90deg)" : "none",
                      transition: "transform 0.15s",
                    }}
                  >
                    ▶
                  </span>
                </div>
                <div className="ml-6 h-1.5 rounded overflow-hidden" style={{ background: "#2e2e2e" }}>
                  {r.solved && (
                    <div
                      className="h-full rounded"
                      style={{ width: `${barPct}%`, background: isWinner ? "var(--orange)" : "#5c5c5c" }}
                    />
                  )}
                </div>
              </button>
              {expandedId === r.id && <ResultDetail resultId={r.id} />}
            </div>
          );
```

Remove the old `onSelectResult` call since we're replacing click-to-meme with click-to-expand. Actually, keep it — the user might still want the meme feature. Let's add a small meme icon button separately. For now, remove the `onSelectResult(r)` from the onClick and use it only if we need it later. The primary click action is now expand/collapse.

- [ ] **Step 4: Verify it compiles**

```bash
cd frontend && npx tsc --noEmit
```
Expected: no type errors.

- [ ] **Step 5: Commit**

```bash
git add frontend/app/components/RaceResults.tsx frontend/app/lib/types.ts backend/src/models.rs backend/src/routes/problems.rs
git commit -m "feat: add expandable execution detail rows to RaceResults"
```

---

### Task 12: Wire Up Submit Flow in page.tsx

**Files:**
- Modify: `frontend/app/page.tsx`

- [ ] **Step 1: No structural changes needed**

The existing `page.tsx` already:
- Passes `onSolve` and `onGiveUp` callbacks to `RaceEditor`
- Tracks `userResult` state
- Passes `userResult` to `RaceResults` for display

The `RaceEditor` changes from Task 9 already call `onSolve(ms)` on successful submit and `onGiveUp(ms)` on failed submit. The user result will appear in the results table via the existing `RaceResults` user row rendering.

The only change: after a successful submit, we should refresh the results list so the user's submission appears in context. Add a results refresh after `onSolve`:

In `page.tsx`, update the `onSolve` callback passed to `RaceEditor`:

```tsx
onSolve={async (ms) => {
  setUserResult({ ms, gaveUp: false });
  // Refresh results to include latest
  const r = await fetchProblemResults(problem.id);
  setResults(r);
}}
```

- [ ] **Step 2: Verify it compiles**

```bash
cd frontend && npx tsc --noEmit
```

- [ ] **Step 3: Commit**

```bash
git add frontend/app/page.tsx
git commit -m "feat: refresh results after successful code submission"
```

---

### Task 13: End-to-End Smoke Test

**Files:** None — manual testing

- [ ] **Step 1: Start backend**

```bash
cd backend && cargo run
```
Verify: migration 003 runs, server starts on port 3001.

- [ ] **Step 2: Start frontend**

```bash
cd frontend && npm run dev
```

- [ ] **Step 3: Test the Run flow**

1. Load a problem
2. Type Python code in the editor
3. Click "▶ Run"
4. Verify test results panel appears with per-case pass/fail
5. Modify code, click Run again — results update

- [ ] **Step 4: Test the Submit flow**

1. Write correct solution code
2. Click "Submit"
3. Verify timer stops, editor locks
4. Verify "You" row appears in results table

- [ ] **Step 5: Test execution details**

1. Click the chevron on any AI result row
2. Verify the generated code and test case results appear
3. Click again to collapse

- [ ] **Step 6: Test rate limiting**

1. Click Run rapidly 11+ times
2. Verify the buttons disable and countdown appears after 10 requests

- [ ] **Step 7: Commit any fixes**

```bash
git add -u
git commit -m "fix: smoke test fixes for code execution feature"
```

---

Plan complete and saved to `docs/superpowers/plans/2026-04-08-code-execution.md`. Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
