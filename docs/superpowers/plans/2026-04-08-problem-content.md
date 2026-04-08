# Problem Content & Auto-Benchmark Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let users see the problem description alongside the code editor, auto-poll for new benchmark results on page load, and parse expected outputs from LeetCode HTML.

**Architecture:** Three independent changes: (1) a new `ProblemPanel` component rendered inside the RaceEditor area as a 40/60 split, (2) auto-poll logic extracted from `handleRaceAgain` and triggered in `loadProblem`, (3) regex extraction of expected outputs in `leetcode.rs::parse_test_cases()`.

**Tech Stack:** Next.js 16 / React 19 / Tailwind 4 (frontend), Rust / Axum / regex (backend)

**Important:** Read `frontend/node_modules/next/dist/docs/` before writing any Next.js code — this version has breaking changes from training data.

---

### Task 1: Parse Expected Outputs from LeetCode HTML (Backend)

**Files:**
- Modify: `backend/src/leetcode.rs:111-123` (parse_test_cases)
- Modify: `backend/Cargo.toml` (add `regex` dep if not present)

This task changes `parse_test_cases()` to also accept the `content` HTML string, extract `<strong>Output:</strong>` values via regex, and pair them with inputs.

- [ ] **Step 1: Add regex dependency if needed**

Check `backend/Cargo.toml` for `regex`. If missing, add it:

```toml
regex = "1"
```

Run: `cd backend && cargo check`
Expected: compiles

- [ ] **Step 2: Update parse_test_cases signature and implementation**

Change `parse_test_cases` in `backend/src/leetcode.rs` from:

```rust
fn parse_test_cases(&self, example_list: &Value) -> Vec<TestCase> {
    let inputs: Vec<String> = example_list
        .as_array()
        .map(|arr| arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect())
        .unwrap_or_default();

    inputs.iter().map(|input| TestCase {
        input: input.clone(),
        expected_output: String::new(),
    }).collect()
}
```

To:

```rust
fn parse_test_cases(&self, example_list: &Value, content: &str) -> Vec<TestCase> {
    let inputs: Vec<String> = example_list
        .as_array()
        .map(|arr| arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect())
        .unwrap_or_default();

    // Extract expected outputs from HTML content.
    // Pattern: <strong>Output:</strong> VALUE (up to next newline or <)
    let re = regex::Regex::new(r#"<strong>Output:</strong>\s*(.+?)(?:\s*<|$)"#).unwrap();
    let outputs: Vec<String> = re.captures_iter(content)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
        .collect();

    inputs.iter().enumerate().map(|(i, input)| TestCase {
        input: input.clone(),
        expected_output: outputs.get(i).cloned().unwrap_or_default(),
    }).collect()
}
```

- [ ] **Step 3: Update the call site in fetch_problem_by_slug**

In `backend/src/leetcode.rs`, change line 96 from:

```rust
let test_cases = self.parse_test_cases(&q["exampleTestcaseList"]);
```

To:

```rust
let test_cases = self.parse_test_cases(&q["exampleTestcaseList"], &description);
```

- [ ] **Step 4: Build and verify**

Run: `cd backend && cargo build`
Expected: compiles with no errors

- [ ] **Step 5: Commit**

```bash
git add backend/src/leetcode.rs backend/Cargo.toml backend/Cargo.lock
git commit -m "feat: parse expected outputs from LeetCode HTML in test cases"
```

---

### Task 2: Auto-Poll Benchmarks on Page Load (Frontend)

**Files:**
- Modify: `frontend/app/page.tsx:24-30` (loadProblem) and lines 46-71 (extract shared poll logic)

Extract the polling logic from `handleRaceAgain` into a shared function, then call it from `loadProblem` when results are incomplete.

- [ ] **Step 1: Extract polling into a reusable function**

In `frontend/app/page.tsx`, add a `pollForResults` function above `loadProblem` and refactor `handleRaceAgain` to use it. Replace lines 24-71 with:

```typescript
const pollForResults = useCallback((problemId: string, currentCount: number) => {
  if (pollRef.current) clearInterval(pollRef.current);
  let attempts = 0;
  let stableCount = 0;
  let lastCount = currentCount;
  pollRef.current = setInterval(async () => {
    const r = await fetchProblemResults(problemId);
    setResults(r);
    attempts++;
    if (r.length > lastCount) { lastCount = r.length; stableCount = 0; }
    else { stableCount++; }
    if (attempts > 40 || stableCount >= 3) {
      clearInterval(pollRef.current!);
      pollRef.current = null;
      setIsRacing(false);
      setRaceKey(k => k + 1);
    }
  }, 3000);
}, []);

const loadProblem = useCallback(async (p: Problem) => {
  setProblem(p);
  setUserResult(null);
  setMemeTarget(null);
  const r = await fetchProblemResults(p.id);
  setResults(r);
  // Auto-poll if results look incomplete (fewer than active non-human models)
  const activeAICount = models.filter(m => !m.is_human && m.is_active).length;
  if (r.length < activeAICount && activeAICount > 0) {
    setIsRacing(true);
    pollForResults(p.id, r.length);
  }
}, [pollForResults, models]);

const loadRandom = useCallback(async () => {
  const p = await fetchRandomProblem();
  await loadProblem(p);
}, [loadProblem]);

useEffect(() => {
  loadRandom();
  fetchModels().then(setModels);
}, [loadRandom]);

useEffect(() => {
  return () => { if (pollRef.current) clearInterval(pollRef.current); };
}, []);

const handleRaceAgain = async () => {
  if (!problem || isRacing) return;
  setIsRacing(true);
  setUserResult(null);
  try {
    await createRace(problem.id);
    pollForResults(problem.id, results.length);
  } catch {
    setIsRacing(false);
    setRaceKey(k => k + 1);
  }
};
```

- [ ] **Step 2: Verify the app builds**

Run: `cd frontend && npm run build`
Expected: compiles with no errors

- [ ] **Step 3: Commit**

```bash
git add frontend/app/page.tsx
git commit -m "feat: auto-poll for benchmark results on problem load"
```

---

### Task 3: Side-by-Side Problem Panel in RaceEditor (Frontend)

**Files:**
- Create: `frontend/app/components/ProblemPanel.tsx`
- Modify: `frontend/app/components/RaceEditor.tsx`
- Modify: `frontend/app/globals.css` (or wherever global styles live)

- [ ] **Step 1: Install DOMPurify for HTML sanitization**

Run: `cd frontend && npm install dompurify && npm install -D @types/dompurify`
Expected: installs successfully

- [ ] **Step 2: Create ProblemPanel component**

Create `frontend/app/components/ProblemPanel.tsx`:

```tsx
"use client";
import { useState } from "react";
import DOMPurify from "dompurify";
import { Problem } from "@/app/lib/types";

interface TestCase {
  input: string;
  expected_output: string;
}

interface Props {
  problem: Problem;
}

export default function ProblemPanel({ problem }: Props) {
  const [tab, setTab] = useState<"description" | "testcases">("description");

  let testCases: TestCase[] = [];
  try {
    testCases = JSON.parse(problem.test_cases);
  } catch {}

  const sanitizedHtml = DOMPurify.sanitize(problem.description);

  return (
    <div className="flex flex-col h-full w-full overflow-hidden" style={{ borderRight: "1px solid var(--border)" }}>
      {/* Tabs */}
      <div className="flex border-b px-3 shrink-0" style={{ background: "var(--surface-2)", borderColor: "var(--border)" }}>
        <button
          onClick={() => setTab("description")}
          className="text-xs px-3 py-2 font-semibold"
          style={{
            color: tab === "description" ? "var(--orange)" : "var(--muted)",
            borderBottom: tab === "description" ? "2px solid var(--orange)" : "2px solid transparent",
            background: "transparent",
            cursor: "pointer",
          }}
        >
          Description
        </button>
        <button
          onClick={() => setTab("testcases")}
          className="text-xs px-3 py-2 font-semibold"
          style={{
            color: tab === "testcases" ? "var(--orange)" : "var(--muted)",
            borderBottom: tab === "testcases" ? "2px solid var(--orange)" : "2px solid transparent",
            background: "transparent",
            cursor: "pointer",
          }}
        >
          Test Cases
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-4">
        {tab === "description" ? (
          <div
            className="text-sm leading-relaxed problem-html"
            style={{ color: "var(--text)" }}
            dangerouslySetInnerHTML={{ __html: sanitizedHtml }}
          />
        ) : (
          <div className="flex flex-col gap-3">
            {testCases.map((tc, i) => (
              <div key={i} className="rounded p-3 text-xs" style={{ background: "#2a2a2a", border: "1px solid var(--border)" }}>
                <div className="font-semibold mb-1" style={{ color: "var(--muted)" }}>Case {i + 1}</div>
                <div className="mb-1">
                  <span style={{ color: "var(--muted)" }}>Input: </span>
                  <code style={{ color: "var(--text)" }}>{tc.input}</code>
                </div>
                {tc.expected_output && (
                  <div>
                    <span style={{ color: "var(--muted)" }}>Expected: </span>
                    <code style={{ color: "var(--green, #00b8a3)" }}>{tc.expected_output}</code>
                  </div>
                )}
              </div>
            ))}
            {testCases.length === 0 && (
              <div className="text-xs" style={{ color: "var(--muted)" }}>No test cases available.</div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Add problem-html styles**

Find the global CSS file (likely `frontend/app/globals.css`). Append these styles:

```css
.problem-html pre {
  background: #2a2a2a;
  border: 1px solid var(--border);
  border-radius: 0.375rem;
  padding: 0.75rem;
  overflow-x: auto;
  font-size: 0.8rem;
}
.problem-html code {
  font-size: 0.85em;
}
.problem-html ul, .problem-html ol {
  padding-left: 1.5rem;
  margin: 0.5rem 0;
}
.problem-html li {
  margin: 0.25rem 0;
}
.problem-html p {
  margin: 0.5rem 0;
}
.problem-html img {
  max-width: 100%;
}
```

- [ ] **Step 4: Modify RaceEditor to include ProblemPanel in a side-by-side layout**

In `frontend/app/components/RaceEditor.tsx`:

Add import at the top:
```tsx
import ProblemPanel from "./ProblemPanel";
```

Add mobile toggle state after the existing state declarations (after line 48):
```tsx
const [showProblem, setShowProblem] = useState(false);
```

Replace the outer `return` wrapper. Change:

```tsx
return (
    <div className="flex flex-col h-full">
```

To:

```tsx
return (
    <div className="flex h-full">
      {/* Problem panel — desktop: 40% side-by-side, mobile: toggleable overlay */}
      <div className="hidden lg:flex lg:w-[40%] lg:flex-shrink-0">
        <ProblemPanel problem={problem} />
      </div>
      {/* Mobile problem panel */}
      {showProblem && (
        <div className="lg:hidden fixed inset-0 z-50 flex flex-col" style={{ background: "var(--bg, #1a1a1a)" }}>
          <div className="flex items-center justify-between px-4 py-2 border-b" style={{ borderColor: "var(--border)" }}>
            <span className="text-sm font-bold" style={{ color: "var(--text)" }}>Problem</span>
            <button
              onClick={() => setShowProblem(false)}
              className="text-xs px-2 py-1 rounded"
              style={{ color: "var(--muted)", background: "#3a3a3a", cursor: "pointer" }}
            >
              Close
            </button>
          </div>
          <div className="flex-1 overflow-hidden">
            <ProblemPanel problem={problem} />
          </div>
        </div>
      )}
      <div className="flex flex-col flex-1 min-w-0">
```

Add a closing `</div>` right before the final `</div>` of the return (after the submit bar `</div>`), to close the new `flex-col flex-1` wrapper.

In the editor toolbar section (the `div` with "Editor toolbar" comment), after the language `<select>`, add a mobile toggle button:

```tsx
<button
  onClick={() => setShowProblem(true)}
  className="lg:hidden text-xs px-2 py-1 rounded"
  style={{ color: "var(--orange)", background: "#3a3a3a", cursor: "pointer" }}
>
  View Problem
</button>
```

- [ ] **Step 5: Verify the app builds**

Run: `cd frontend && npm run build`
Expected: compiles with no errors

- [ ] **Step 6: Commit**

```bash
git add frontend/app/components/ProblemPanel.tsx frontend/app/components/RaceEditor.tsx frontend/app/globals.css frontend/package.json frontend/package-lock.json
git commit -m "feat: side-by-side problem panel in race editor with mobile toggle"
```

---

### Task 4: Code Review Fixes (PR #8)

**Files:**
- Modify: `frontend/app/components/RaceEditor.tsx:75` (null safety)
- Modify: `frontend/app/components/RaceEditor.tsx:161` (min animation duration)

These are small fixes from code review on PR #8.

- [ ] **Step 1: Fix null safety on ai.time_ms in startRace**

In `frontend/app/components/RaceEditor.tsx`, in the `startRace` callback, change:

```typescript
const t = setTimeout(() => {
  setSolvedIds(prev => new Set([...prev, ai.model_id]));
}, ai.time_ms!);
```

To:

```typescript
const t = setTimeout(() => {
  setSolvedIds(prev => new Set([...prev, ai.model_id]));
}, ai.time_ms ?? 0);
```

- [ ] **Step 2: Clamp minimum animation duration to 0.3s**

In the AI row bar transition, change:

```typescript
: `width ${(ai.time_ms! / 1000).toFixed(2)}s linear`,
```

To:

```typescript
: `width ${Math.max(0.3, (ai.time_ms ?? 0) / 1000).toFixed(2)}s linear`,
```

- [ ] **Step 3: Verify the app builds**

Run: `cd frontend && npm run build`
Expected: compiles with no errors

- [ ] **Step 4: Commit**

```bash
git add frontend/app/components/RaceEditor.tsx
git commit -m "fix: null safety on time_ms and clamp min animation to 0.3s"
```
