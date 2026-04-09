# Timer Fix & Language Selector Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the timer-reset bug on rate-limited submit, and make the language selector functional with Python 3 + JavaScript support.

**Architecture:** Two independent fixes. Fix 1 is a frontend-only change to `handleRunSubmit` in RaceEditor. Fix 2 threads a `language` parameter through the full stack: DB migration (starter_code → JSONB, language column on submissions), backend models/piston/runner/routes, LeetCode fetcher, and frontend state/API/UI.

**Tech Stack:** Rust (Axum, SQLx, regex), PostgreSQL, Judge0 CE sandbox, Next.js 16, React 19, TypeScript

---

### Task 1: Fix Timer State on Rate-Limited Submit

**Files:**
- Modify: `frontend/app/components/RaceEditor.tsx:133-156`

- [ ] **Step 1: Fix `handleRunSubmit` — defer stop/phase until success**

Replace the current `handleRunSubmit` function:

```tsx
const handleRunSubmit = async () => {
  if (isRunning || rateLimitCountdown > 0) return;
  setIsRunning(true);
  const ms = elapsedMs;
  try {
    const result = await submitCode(code, problem.id, ms, runAttempts + 1);
    stop();
    setPhase("submitted");
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
    }
  } finally {
    setIsRunning(false);
  }
};
```

Key changes vs current code:
- `const ms = elapsedMs` snapshots the time without stopping the timer
- `stop()` and `setPhase("submitted")` move inside the `try` block after a successful response
- The `catch` block no longer calls `setPhase("racing")` or `start()` — the timer never stopped

- [ ] **Step 2: Verify the fix compiles**

Run: `cd frontend && npx next build --no-lint 2>&1 | tail -20`
Expected: Build succeeds (or only unrelated warnings)

- [ ] **Step 3: Commit**

```bash
git add frontend/app/components/RaceEditor.tsx
git commit -m "fix: preserve timer state when submit is rate-limited"
```

---

### Task 2: DB Migration — JSONB starter_code + language on submissions

**Files:**
- Create: `backend/migrations/004_language_support.sql`

- [ ] **Step 1: Write the migration**

```sql
-- Convert starter_code from plain text to JSONB
ALTER TABLE problems
  ALTER COLUMN starter_code TYPE JSONB
  USING jsonb_build_object('python3', starter_code);

-- Add language to submissions
ALTER TABLE submissions ADD COLUMN language TEXT NOT NULL DEFAULT 'python3';
```

- [ ] **Step 2: Commit**

```bash
git add backend/migrations/004_language_support.sql
git commit -m "feat: migration for JSONB starter_code and submission language"
```

---

### Task 3: Backend Models — JSONB starter_code + language on requests

**Files:**
- Modify: `backend/src/models.rs:1-14` (Problem struct)
- Modify: `backend/src/models.rs:148-167` (RunCodeRequest, SubmitCodeRequest)

- [ ] **Step 1: Change `Problem.starter_code` to `serde_json::Value`**

In `backend/src/models.rs`, update the Problem struct:

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Problem {
    pub id: String,
    pub lc_id: i64,
    pub title: String,
    pub difficulty: String,
    pub description: String,
    pub starter_code: Value,
    pub test_cases: String,
    pub source: String,
    pub cached_at: String,
}
```

- [ ] **Step 2: Add `language` field to request structs**

Update `RunCodeRequest`:

```rust
#[derive(Debug, Deserialize)]
pub struct RunCodeRequest {
    pub code: String,
    pub problem_id: String,
    pub language: Option<String>,
}
```

Update `SubmitCodeRequest`:

```rust
#[derive(Debug, Deserialize)]
pub struct SubmitCodeRequest {
    pub code: String,
    pub problem_id: String,
    pub time_ms: i64,
    pub attempts: i64,
    pub language: Option<String>,
}
```

- [ ] **Step 3: Commit**

```bash
git add backend/src/models.rs
git commit -m "feat: JSONB starter_code type + language field on request models"
```

---

### Task 4: Backend Piston — Generalize to multi-language execution

**Files:**
- Modify: `backend/src/piston.rs`

- [ ] **Step 1: Add language ID mapping and rename `run_python` → `run`**

Replace the full contents of `backend/src/piston.rs`:

```rust
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct Judge0Request {
    language_id: u32,
    source_code: String,
    stdin: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu_time_limit: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wall_time_limit: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory_limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_network: Option<bool>,
}

#[derive(Deserialize)]
struct Judge0Response {
    stdout: Option<String>,
    stderr: Option<String>,
    compile_output: Option<String>,
    status: Judge0Status,
}

#[derive(Deserialize)]
struct Judge0Status {
    id: u32,
}

pub struct PistonRun {
    pub stdout: String,
    pub stderr: String,
    pub code: i64,
}

pub struct PistonClient {
    client: Client,
    base_url: String,
}

fn language_id(language: &str) -> Result<u32> {
    match language {
        "python3" => Ok(71),
        "javascript" => Ok(63),
        _ => anyhow::bail!("unsupported language: {}", language),
    }
}

impl PistonClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
        }
    }

    pub async fn run(&self, language: &str, code: &str, stdin: &str) -> Result<PistonRun> {
        let lang_id = language_id(language)?;
        let url = format!("{}/submissions?wait=true", self.base_url);
        let body = Judge0Request {
            language_id: lang_id,
            source_code: code.to_string(),
            stdin: stdin.to_string(),
            cpu_time_limit: Some(5.0),
            wall_time_limit: Some(10.0),
            memory_limit: Some(128_000),
            enable_network: Some(false),
        };

        let resp: Judge0Response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .timeout(std::time::Duration::from_secs(30))
            .send().await
            .context("judge0 request failed")?
            .error_for_status()
            .context("judge0 returned error status")?
            .json().await
            .context("judge0 response parse failed")?;

        if resp.status.id == 13 {
            anyhow::bail!("judge0 internal error");
        }

        let code = if resp.status.id == 3 { 0 } else { 1 };

        let stderr = [resp.stderr, resp.compile_output]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("\n");

        Ok(PistonRun {
            stdout: resp.stdout.unwrap_or_default(),
            stderr,
            code,
        })
    }
}

/// Dispatch to the correct language wrapper.
pub fn wrap_solution(language: &str, code: &str, input: &str) -> String {
    match language {
        "javascript" => wrap_solution_js(code),
        _ => wrap_solution_py(code, input),
    }
}

fn wrap_solution_py(solution_code: &str, _input: &str) -> String {
    format!(r#"
import json, sys

{solution_code}

# Harness
if __name__ == "__main__":
    lines = sys.stdin.read().strip().split('\n')
    args = [json.loads(line) for line in lines if line.strip()]
    s = Solution()
    for method in [m for m in dir(s) if not m.startswith('_')]:
        try:
            result = getattr(s, method)(*args)
            print(json.dumps(result))
            break
        except Exception as e:
            print(f"ERROR: {{e}}", file=sys.stderr)
"#, solution_code = solution_code)
}

fn wrap_solution_js(code: &str) -> String {
    // Extract function name from LeetCode-style JS:
    //   var twoSum = function(...)   OR   function twoSum(...)
    let fn_name = extract_js_function_name(code).unwrap_or_else(|| "solution".to_string());

    format!(r#"
{code}

const lines = require('fs').readFileSync('/dev/stdin', 'utf8').trim().split('\n');
const args = lines.filter(l => l.trim()).map(JSON.parse);
const result = {fn_name}(...args);
console.log(JSON.stringify(result));
"#, code = code, fn_name = fn_name)
}

fn extract_js_function_name(code: &str) -> Option<String> {
    // Match: var/let/const name = function(
    let re1 = regex::Regex::new(r"(?:var|let|const)\s+(\w+)\s*=\s*function").ok()?;
    if let Some(cap) = re1.captures(code) {
        return Some(cap[1].to_string());
    }
    // Match: function name(
    let re2 = regex::Regex::new(r"function\s+(\w+)\s*\(").ok()?;
    if let Some(cap) = re2.captures(code) {
        return Some(cap[1].to_string());
    }
    // Match: var/let/const name = (...) =>
    let re3 = regex::Regex::new(r"(?:var|let|const)\s+(\w+)\s*=\s*\(").ok()?;
    if let Some(cap) = re3.captures(code) {
        return Some(cap[1].to_string());
    }
    None
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd backend && cargo check 2>&1 | tail -20`
Expected: Errors only from runner.rs (which still calls `run_python` and old `wrap_solution` — fixed in Task 5)

- [ ] **Step 3: Commit**

```bash
git add backend/src/piston.rs
git commit -m "feat: multi-language Judge0 execution with JS wrapper"
```

---

### Task 5: Backend Runner — Thread language through verify

**Files:**
- Modify: `backend/src/runner.rs:97-99` (build_prompt uses starter_code)
- Modify: `backend/src/runner.rs:109` (verify_with_detail call in race_one)
- Modify: `backend/src/runner.rs:174-189` (verify function)
- Modify: `backend/src/runner.rs:192-241` (verify_with_detail function)

- [ ] **Step 1: Update `build_prompt` and `build_retry_prompt` to accept a `&str` starter code**

These functions already accept `&str`. The change is at the *call site* in `race_one` — extract the python3 starter code from the `serde_json::Value`:

In `race_one`, before the retry loop, add:

```rust
let starter = problem.starter_code
    .get("python3")
    .and_then(|v| v.as_str())
    .unwrap_or("class Solution:\n    pass");
```

Then change the two `build_prompt` / `build_retry_prompt` calls from `&problem.starter_code` to `starter`.

- [ ] **Step 2: Add `language` parameter to `verify` and `verify_with_detail`**

Update `verify`:

```rust
async fn verify(&self, code: &str, test_cases: &[TestCase], language: &str) -> Result<bool> {
    if test_cases.is_empty() {
        return Ok(true);
    }
    for tc in test_cases {
        let wrapped = wrap_solution(language, code, &tc.input);
        let run = self.piston.run(language, &wrapped, &tc.input).await?;
        if run.code != 0 { return Ok(false); }
        if !tc.expected_output.is_empty() {
            let got = run.stdout.trim();
            let want = tc.expected_output.trim();
            if got != want { return Ok(false); }
        }
    }
    Ok(true)
}
```

Update `verify_with_detail`:

```rust
pub async fn verify_with_detail(
    &self,
    code: &str,
    test_cases: &[TestCase],
    language: &str,
) -> Result<(bool, Vec<crate::models::TestCaseResult>, String)> {
    let mut all_passed = true;
    let mut case_results = Vec::new();
    let mut last_stderr = String::new();

    if test_cases.is_empty() {
        return Ok((true, case_results, last_stderr));
    }

    for tc in test_cases {
        let wrapped = wrap_solution(language, code, &tc.input);
        match self.piston.run(language, &wrapped, &tc.input).await {
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

- [ ] **Step 3: Update call site in `race_one`**

The call at line ~109 changes from:

```rust
match self.verify_with_detail(&code, test_cases).await {
```

to:

```rust
match self.verify_with_detail(&code, test_cases, "python3").await {
```

AI benchmarks always run Python.

- [ ] **Step 4: Verify it compiles**

Run: `cd backend && cargo check 2>&1 | tail -20`
Expected: Errors only from execution.rs routes (fixed in Task 6)

- [ ] **Step 5: Commit**

```bash
git add backend/src/runner.rs
git commit -m "feat: thread language param through runner verify functions"
```

---

### Task 6: Backend Routes — Accept language, validate, pass through

**Files:**
- Modify: `backend/src/routes/execution.rs`

- [ ] **Step 1: Add language validation helper**

Add at the top of `execution.rs`, after the imports:

```rust
fn validated_language(lang: Option<&str>) -> Result<&str, Response> {
    match lang.unwrap_or("python3") {
        l @ ("python3" | "javascript") => Ok(l),
        _ => Err(StatusCode::BAD_REQUEST.into_response()),
    }
}
```

- [ ] **Step 2: Update `run_code` to pass language**

In the `run_code` function, after the rate limit check and code size check, add language validation:

```rust
let language = match validated_language(body.language.as_deref()) {
    Ok(l) => l,
    Err(r) => return r,
};
```

Then change the `verify_with_detail` call from:

```rust
let (passed, results, stderr) = match state
    .runner
    .verify_with_detail(&body.code, &test_cases)
    .await
```

to:

```rust
let (passed, results, stderr) = match state
    .runner
    .verify_with_detail(&body.code, &test_cases, language)
    .await
```

- [ ] **Step 3: Update `submit_code` to pass language and store it**

Add the same language validation after the rate limit / size / time / attempts checks:

```rust
let language = match validated_language(body.language.as_deref()) {
    Ok(l) => l,
    Err(r) => return r,
};
```

Change the `verify_with_detail` call the same way as in step 2.

Update the INSERT into submissions to include language — change the query from:

```rust
if sqlx::query(
    r#"INSERT INTO submissions (id, problem_id, ip_hash, solved, time_ms, attempts, code, submitted_at)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
)
.bind(&id)
.bind(&body.problem_id)
.bind(&ip_hash)
.bind(true)
.bind(body.time_ms)
.bind(body.attempts)
.bind(&body.code)
.bind(&now)
```

to:

```rust
if sqlx::query(
    r#"INSERT INTO submissions (id, problem_id, ip_hash, solved, time_ms, attempts, code, language, submitted_at)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
)
.bind(&id)
.bind(&body.problem_id)
.bind(&ip_hash)
.bind(true)
.bind(body.time_ms)
.bind(body.attempts)
.bind(&body.code)
.bind(language)
.bind(&now)
```

- [ ] **Step 4: Verify it compiles**

Run: `cd backend && cargo check 2>&1 | tail -20`
Expected: Errors only from leetcode.rs (starter_code type mismatch — fixed in Task 7)

- [ ] **Step 5: Commit**

```bash
git add backend/src/routes/execution.rs
git commit -m "feat: accept and validate language in run/submit routes"
```

---

### Task 7: Backend LeetCode — Fetch multi-language snippets

**Files:**
- Modify: `backend/src/leetcode.rs:89-94` (starter_code extraction)
- Modify: `backend/src/leetcode.rs:104` (Problem construction)

- [ ] **Step 1: Update `fetch_problem_by_slug` to extract both snippets**

Replace the starter_code extraction block (lines 89-94):

```rust
let snippets = q["codeSnippets"].as_array();

let python_code = snippets
    .and_then(|snips| snips.iter().find(|s| s["langSlug"] == "python3"))
    .and_then(|s| s["code"].as_str())
    .unwrap_or("class Solution:\n    pass");

let js_code = snippets
    .and_then(|snips| snips.iter().find(|s| s["langSlug"] == "javascript"))
    .and_then(|s| s["code"].as_str())
    .unwrap_or("// Write your solution here\n");

let starter_code = serde_json::json!({
    "python3": python_code,
    "javascript": js_code,
});
```

Then in the `Problem` construction, change `starter_code` from a `String` to the `Value`:

```rust
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
```

- [ ] **Step 2: Verify the full backend compiles**

Run: `cd backend && cargo check 2>&1 | tail -20`
Expected: No errors. All backend tasks are now consistent.

- [ ] **Step 3: Commit**

```bash
git add backend/src/leetcode.rs
git commit -m "feat: fetch Python and JavaScript starter code from LeetCode"
```

---

### Task 8: Frontend — Wire language selector and API

**Files:**
- Modify: `frontend/app/lib/types.ts:7` (starter_code type)
- Modify: `frontend/app/lib/api.ts:62-93` (runCode, submitCode)
- Modify: `frontend/app/components/RaceEditor.tsx`

- [ ] **Step 1: Update `types.ts` — starter_code type**

Change `starter_code` from `string` to the parsed type:

```typescript
export interface Problem {
  id: string;
  lc_id: number;
  title: string;
  difficulty: "Easy" | "Medium" | "Hard";
  description: string;
  starter_code: Record<string, string>;
  test_cases: string;
  source: string;
  cached_at: string;
}
```

- [ ] **Step 2: Update `api.ts` — add language to runCode and submitCode**

Update `runCode`:

```typescript
export async function runCode(code: string, problemId: string, language: string = "python3"): Promise<RunResult> {
  const res = await fetch(`${BASE}/run`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ code, problem_id: problemId, language }),
  });
  if (res.status === 429) {
    const data = await res.json().catch(() => ({}));
    throw new RateLimitError(data.retry_after ?? 60);
  }
  if (!res.ok) throw new Error("failed to run code");
  return res.json();
}
```

Update `submitCode`:

```typescript
export async function submitCode(
  code: string,
  problemId: string,
  timeMs: number,
  attempts: number,
  language: string = "python3"
): Promise<SubmitResult> {
  const res = await fetch(`${BASE}/submit`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ code, problem_id: problemId, time_ms: timeMs, attempts, language }),
  });
  if (res.status === 429) {
    const data = await res.json().catch(() => ({}));
    throw new RateLimitError(data.retry_after ?? 60);
  }
  if (!res.ok) throw new Error("failed to submit code");
  return res.json();
}
```

- [ ] **Step 3: Update `RaceEditor.tsx` — add language state and wire the selector**

Add `language` state and a helper to get the starter code for the current language. Update the `code` initializer.

Add a helper function at the top of the component body (before any state declarations):

```tsx
const getStarter = (lang: string) => {
  const codes = problem.starter_code ?? {};
  return codes[lang] ?? "";
};
```

Add `language` state after the existing state declarations (around line 48):

```tsx
const [language, setLanguage] = useState<string>("python3");
```

Change the `code` initializer (line 48) from:

```tsx
const [code, setCode] = useState(problem.starter_code ?? "");
```

to:

```tsx
const [code, setCode] = useState(() => getStarter("python3"));
```

- [ ] **Step 4: Update problem-change reset effect**

In the `useEffect` that resets on problem change (around line 61), change:

```tsx
setCode(problem.starter_code ?? "");
```

to:

```tsx
setCode(getStarter(language));
```

Do NOT reset `language` — preserve the user's selection across problem changes.

- [ ] **Step 5: Add language change handler**

Add after the `handleInput` function:

```tsx
const handleLanguageChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
  const newLang = e.target.value;
  const oldStarter = getStarter(language);
  setLanguage(newLang);
  // Only swap to new starter if user hasn't edited the current starter
  if (code === oldStarter || code === "") {
    setCode(getStarter(newLang));
  }
};
```

- [ ] **Step 6: Wire the `<select>` element**

Replace the `<select>` block (around line 280-289):

```tsx
<select
  className="text-xs rounded px-2 py-1"
  style={{ background: "#3a3a3a", border: "1px solid #4a4a4a", color: "var(--text)" }}
  value={language}
  onChange={handleLanguageChange}
  disabled={phase === "submitted"}
>
  <option value="python3">Python 3</option>
  <option value="javascript">JavaScript</option>
</select>
```

- [ ] **Step 7: Pass language to API calls**

In `handleRun`, change:

```tsx
const result = await runCode(code, problem.id);
```

to:

```tsx
const result = await runCode(code, problem.id, language);
```

In `handleRunSubmit`, change:

```tsx
const result = await submitCode(code, problem.id, ms, runAttempts + 1);
```

to:

```tsx
const result = await submitCode(code, problem.id, ms, runAttempts + 1, language);
```

- [ ] **Step 8: Verify frontend compiles**

Run: `cd frontend && npx next build --no-lint 2>&1 | tail -20`
Expected: Build succeeds

- [ ] **Step 9: Commit**

```bash
git add frontend/app/lib/types.ts frontend/app/lib/api.ts frontend/app/components/RaceEditor.tsx
git commit -m "feat: functional language selector with Python + JavaScript support"
```
