# Code Execution & Verification Design

## Goal

Two features that make howfastwould interactive rather than display-only:

1. **Execution visibility** — show the actual code each AI model generated, per-test-case pass/fail with input/expected/got, and stderr. Users can expand any result row to see the full execution trace.
2. **User code execution** — users write Python in the RaceEditor, run it against test cases, and submit their solution. Their time appears alongside AI results in the same table.

## Backend

### New endpoint: `POST /api/run`

Executes user code against a problem's test cases without persisting anything.

```
Request:  { code: string, problem_id: string }
Response: {
  passed: bool,
  results: [{ input: string, expected: string, got: string, passed: bool }],
  stderr: string
}
```

- Reuses existing `PistonClient` and `wrap_solution()` harness
- Runs against all test cases for the problem
- Rate-limited: 10 runs per IP per minute (in-memory counter via `tokio::time`, no Redis)
- No authentication required

### New endpoint: `POST /api/submit`

Executes user code and persists the result if all tests pass.

```
Request:  { code: string, problem_id: string, time_ms: i64, attempts: i64 }
Response: {
  passed: bool,
  results: [{ input: string, expected: string, got: string, passed: bool }],
  submission_id: string | null
}
```

- Same execution as `/run`, plus writes to `submissions` table on success
- `time_ms` from frontend timer — server validates > 0 and < 3,600,000 (1 hour)
- `attempts` is the number of "Run" clicks before "Submit"
- Returns `submission_id` only if all tests pass

### New endpoint: `GET /api/results/:result_id/details`

Returns the full execution trace for a single AI benchmark result.

```
Response: {
  code: string,
  test_results: [{ input: string, expected: string, got: string, passed: bool }],
  stderr: string
}
```

- Called on-demand when a user expands a result row
- No extra data sent on the main results fetch

### Rate limiting

Simple in-memory rate limiter using a `DashMap<IpAddr, (u32, Instant)>`:
- 10 requests per minute per IP for `/run` and `/submit` combined
- Returns `429 Too Many Requests` with `Retry-After` header when exceeded
- Counter resets after 60 seconds of inactivity
- No persistence needed — resets on server restart, which is fine

### Runner changes

`race_one()` in `runner.rs` currently returns `RaceResult` with solved/time/attempts. Changes:

- Add `verify_with_detail()` alongside existing `verify()` — returns `Vec<TestCaseResult>` with per-case input/expected/got/passed instead of just a bool
- After writing to `results` table, write execution detail to `execution_details` table
- Capture the generated code from the last attempt (successful or not)
- Capture stderr from the last attempt

The existing `verify()` method stays unchanged for backward compatibility with the auto-seed flow.

## Data Model

### New table: `submissions`

```sql
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
```

- Separate from AI `results` table — keeps AI benchmarks clean
- `ip_hash`: SHA-256 of the client IP — enough to group submissions without storing PII
- No unique constraint on (problem_id, ip_hash) — users can submit multiple times, best time wins for display
- When fetching submissions for display, query: `SELECT * FROM submissions WHERE problem_id = $1 AND solved = true ORDER BY time_ms ASC LIMIT 20`

### New table: `execution_details`

```sql
CREATE TABLE execution_details (
    id           TEXT PRIMARY KEY,
    result_id    TEXT NOT NULL REFERENCES results(id) ON DELETE CASCADE,
    code         TEXT NOT NULL,
    test_results TEXT NOT NULL,
    stderr       TEXT NOT NULL DEFAULT ''
);

CREATE INDEX idx_execution_details_result_id ON execution_details(result_id);
```

- `test_results`: JSON string — `[{input, expected, got, passed}]`
- One row per AI result — populated during benchmark runs
- Queried on-demand via `GET /api/results/:result_id/details`

## Frontend

### RaceEditor changes

The editor already has a code input area and timer (`useTimer` hook). Add:

- **"Run" button** — calls `POST /api/run`, displays per-test-case results below the editor. Does not stop the timer. User can iterate freely.
- **"Submit" button** — calls `POST /api/submit` with the current timer value and attempt count. Stops the timer, locks the editor. If all tests pass, the result appears in the results table.

### Test results panel (below editor)

Shown after clicking "Run" or "Submit":

```
Test 1: ✓  Input: [2,7,11,15], 9  →  Expected: [0,1]  Got: [0,1]
Test 2: ✗  Input: [3,2,4], 6     →  Expected: [1,2]  Got: [0,1]
```

- Green check / red X per test case
- Wrong answers shown with strikethrough in red
- Stderr displayed in a red-tinted box if present
- "Run" results are ephemeral (not stored), "Submit" results are persisted
- Summary line: "2 passed · 1 failed"

### RaceResults table changes

- Each result row gets an expand/collapse chevron
- **Collapsed** (default): model name, time, solved/failed badge, attempt count — same as today
- **Expanded**: fetches `GET /api/results/:result_id/details` and shows generated code (syntax-highlighted), per-test-case results, stderr
- Human submissions appear with a green "You" / "Human" badge, sorted by time alongside AI results
- If user failed, shows "Failed" with attempt count, same treatment as failed AI models

### Rate limit feedback

When the user hits the 10/min rate limit:
- "Run" and "Submit" buttons disable
- Show countdown: "Try again in Xs"
- Re-enable automatically when the limit resets

## API client additions (`app/lib/api.ts`)

```typescript
runCode(code: string, problemId: string): Promise<RunResult>
submitCode(code: string, problemId: string, timeMs: number, attempts: number): Promise<SubmitResult>
fetchResultDetails(resultId: string): Promise<ExecutionDetails>
fetchSubmissions(problemId: string): Promise<Submission[]>
```

## Future Work

- **User comparisons** — compare solve times between users. Requires some form of user identity (anonymous persistent tokens or accounts). The `submissions` table already stores the data needed; this is a UI/API feature on top of existing data.
- **Solution sharing** — let users share their submitted solution via a permalink. `code` is already stored in `submissions`.
- **Multi-language support** — currently Python-only (Judge0 supports 60+ languages). Would need per-language `wrap_solution()` harnesses.
