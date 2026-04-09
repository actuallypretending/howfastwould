# Timer Fix & Language Selector Design

Two fixes: (1) preserve timer state when submit is rate-limited, (2) make the language selector functional with Python + JavaScript support.

## Fix 1: Timer State Preserved on Rate-Limited Submit

### Problem

In `RaceEditor.tsx`, `handleRunSubmit` calls `stop()` before the API call, which sets `startRef.current` to null and transitions timer state to "stopped". If the request is rate-limited, the catch block calls `start()`, which resets `startRef.current = Date.now()` — losing all elapsed time.

### Solution

Don't stop the timer or change phase until the API call succeeds. Snapshot `elapsedMs` for the request payload, but leave the timer running. On success, call `stop()` and `setPhase("submitted")`. On rate limit, the timer is still running with its original start time — nothing to restore.

### Changes

**`frontend/app/components/RaceEditor.tsx`** — `handleRunSubmit`:
- Remove the early `stop()` and `setPhase("submitted")` before the try block
- Snapshot `elapsedMs` for the `time_ms` parameter
- Move `stop()` and `setPhase("submitted")` into the success path (after response)
- In the catch block for `RateLimitError`, remove `setPhase("racing")` and `start()` — they're no longer needed since the timer never stopped

## Fix 2: Language Selector (Python + JavaScript)

### Overview

Wire the non-functional language dropdown to state, thread language through the API, and add JavaScript execution support in the Judge0 sandbox. Scope: Python 3 and JavaScript only.

### DB Migration (`backend/migrations/004_language_support.sql`)

```sql
-- Convert starter_code from plain text to JSONB
ALTER TABLE problems
  ALTER COLUMN starter_code TYPE JSONB
  USING jsonb_build_object('python3', starter_code);

-- Add language to submissions
ALTER TABLE submissions ADD COLUMN language TEXT NOT NULL DEFAULT 'python3';
```

### Backend — Models (`backend/src/models.rs`)

- `Problem.starter_code`: change to `serde_json::Value` to match the JSONB column. Update `query_as!` usages — sqlx maps Postgres JSONB to `serde_json::Value` natively. Serialize to JSON string for the API response.
- `RunCodeRequest`: add `language: Option<String>` (defaults to `"python3"`)
- `SubmitCodeRequest`: add `language: Option<String>` (defaults to `"python3"`)

Accepted language values: `"python3"`, `"javascript"`. Reject anything else with 400.

### Backend — Piston Client (`backend/src/piston.rs`)

**Generalize execution:**
- Rename `run_python` → `run` with signature `run(&self, language: &str, code: &str, stdin: &str)`
- Language-to-Judge0-ID mapping:
  - `"python3"` → 71
  - `"javascript"` → 63
- Return error for unsupported languages

**Add JavaScript wrapper** (`wrap_solution_js`):
- Extract function name from code using regex: `var\s+(\w+)\s*=\s*function` or `function\s+(\w+)\s*\(`
- Generate Node.js harness:

```javascript
// {user_code}

const lines = require('fs').readFileSync('/dev/stdin', 'utf8').trim().split('\n');
const args = lines.filter(l => l.trim()).map(JSON.parse);
const result = {functionName}(...args);
console.log(JSON.stringify(result));
```

**Update `wrap_solution`** (existing Python wrapper): rename to `wrap_solution_py` for clarity.

**Add top-level dispatch function:** `wrap_solution(language: &str, code: &str, input: &str) -> String` that calls the appropriate wrapper.

### Backend — Runner (`backend/src/runner.rs`)

- `verify` and `verify_with_detail`: accept a `language: &str` parameter
- Call `wrap_solution(language, code, input)` instead of `wrap_solution(code, input)`
- Call `self.piston.run(language, &wrapped, &tc.input)` instead of `run_python`
- AI benchmark runs still use `"python3"` (unchanged behavior)

### Backend — Execution Routes (`backend/src/routes/execution.rs`)

- `run_code`: read `body.language`, default to `"python3"`, validate, pass to `runner.verify_with_detail`
- `submit_code`: same, plus store language in submissions table
- Reject unsupported languages with 400 Bad Request

### Backend — LeetCode Fetcher (`backend/src/leetcode.rs`)

- `fetch_problem_by_slug`: extract both `python3` and `javascript` snippets from `codeSnippets`
- Build `starter_code` as JSON: `{"python3": "...", "javascript": "..."}`
- Default stubs: `"class Solution:\n    pass"` for Python, `"// Write your solution here\n"` for JavaScript
- `cache_problem`: `starter_code` is now JSON string, no query changes needed beyond the type

### Frontend — Types (`frontend/app/lib/types.ts`)

- `Problem.starter_code` remains `string` but now contains JSON. Add a helper or parse inline.

### Frontend — API Client (`frontend/app/lib/api.ts`)

- `runCode(code, problemId, language)`: add `language` param, include in request body
- `submitCode(code, problemId, timeMs, attempts, language)`: add `language` param, include in request body

### Frontend — RaceEditor (`frontend/app/components/RaceEditor.tsx`)

**State:**
- Add `const [language, setLanguage] = useState<string>("python3")`
- Parse starter codes: `const starterCodes = JSON.parse(problem.starter_code)` (with fallback)
- Initialize code from `starterCodes[language]`

**Language select:**
- Wire `<select>` to `language` state with `onChange`
- Only show two options: Python 3, JavaScript
- On language change: update `code` to `starterCodes[newLanguage]` (only if code equals current language's starter, to avoid overwriting user edits)

**API calls:**
- Pass `language` to `runCode(code, problem.id, language)`
- Pass `language` to `submitCode(code, problem.id, ms, runAttempts + 1, language)`

**Problem change reset:**
- Preserve the user's language selection across problem changes (don't reset to python3)
