# Frontend Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign howfastwould.com to a LeetCode-inspired dark theme with a two-column desktop layout and a live AI race feature triggered by the user's first keypress.

**Architecture:** Update CSS variables and font in `globals.css`, then restyle each component in dependency order (leaf components first, `page.tsx` last). The new `RaceEditor` component replaces `YouBanner` and owns the entire right panel — race simulation, code editor, and submit. Race simulation is purely client-side: `setTimeout` callbacks fire at each AI's pre-benchmarked `time_ms` and CSS transitions animate the bars.

**Tech Stack:** Next.js 14 (App Router), React, Tailwind CSS, TypeScript. No new dependencies.

---

## File Map

| File | Change |
|------|--------|
| `frontend/app/globals.css` | Replace CSS vars + font |
| `frontend/app/layout.tsx` | No change |
| `frontend/app/page.tsx` | Two-column layout, nav with inline search |
| `frontend/app/components/ProblemHeader.tsx` | LeetCode-style header |
| `frontend/app/components/WinnerCard.tsx` | **New** — winner hero display |
| `frontend/app/components/RaceResults.tsx` | Updated leaderboard style |
| `frontend/app/components/SearchBar.tsx` | Restyle for inline nav use |
| `frontend/app/components/YouBanner.tsx` | **Delete** — replaced by RaceEditor |
| `frontend/app/components/RaceEditor.tsx` | **New** — right panel: race + editor + submit |

`useTimer.ts`, `lib/api.ts`, `lib/types.ts`, `MemeCard.tsx` — untouched.

---

## Task 1: Update CSS variables and font

**Files:**
- Modify: `frontend/app/globals.css`

- [ ] **Step 1: Replace the file contents**

```css
@import "tailwindcss";

:root {
  --bg: #1a1a1a;
  --surface: #282828;
  --surface-2: #222222;
  --border: #3a3a3a;
  --orange: #FFA116;
  --text: #eff1f6;
  --muted: #888888;
  --green: #00b8a3;
  --red: #ef4743;
}

body {
  background: var(--bg);
  color: var(--text);
  font-family: -apple-system, 'Segoe UI', Arial, sans-serif;
}

.difficulty-easy  { color: var(--green); background: rgba(0,184,163,0.1); }
.difficulty-medium { color: var(--orange); background: rgba(255,161,22,0.1); }
.difficulty-hard  { color: var(--red); background: rgba(239,71,67,0.1); }
```

- [ ] **Step 2: Run dev server and verify**

```bash
cd frontend && npm run dev
```

Open http://localhost:3000. The background should now be `#1a1a1a` and body font should be system sans-serif (no more JetBrains Mono everywhere).

- [ ] **Step 3: Commit**

```bash
git add frontend/app/globals.css
git commit -m "style: update CSS variables to LeetCode theme"
```

---

## Task 2: Restyle ProblemHeader

**Files:**
- Modify: `frontend/app/components/ProblemHeader.tsx`

- [ ] **Step 1: Replace the component**

```tsx
import { Problem, Model } from "@/app/lib/types";

interface Props {
  problem: Problem;
  newModels: Model[];
  solved: boolean;
  onRaceAgain: () => void;
  isRacing: boolean;
}

export default function ProblemHeader({ problem, newModels, solved, onRaceAgain, isRacing }: Props) {
  return (
    <div className="px-5 py-4 border-b" style={{ borderColor: "var(--border)" }}>
      <div className="flex items-center gap-2 flex-wrap mb-2">
        <span className="text-xs" style={{ color: "var(--muted)" }}>#{problem.lc_id}</span>
        <span
          className="text-xs rounded-full px-2 py-0.5 font-semibold"
          style={
            problem.difficulty === "Easy"
              ? { color: "var(--green)", background: "rgba(0,184,163,0.1)" }
              : problem.difficulty === "Medium"
              ? { color: "var(--orange)", background: "rgba(255,161,22,0.1)" }
              : { color: "var(--red)", background: "rgba(239,71,67,0.1)" }
          }
        >
          {problem.difficulty}
        </span>
        {solved && (
          <span
            className="text-xs rounded-full px-2 py-0.5 font-semibold"
            style={{ color: "var(--green)", background: "rgba(0,184,163,0.1)" }}
          >
            Solved
          </span>
        )}
        {newModels.length > 0 && (
          <span
            className="ml-auto text-xs rounded px-2 py-0.5"
            style={{ background: "#1a1a00", color: "#ffdd57" }}
          >
            🆕 {newModels[0].display_name} just dropped
          </span>
        )}
        <button
          onClick={onRaceAgain}
          disabled={isRacing}
          className="text-xs rounded px-2 py-0.5"
          style={{
            color: isRacing ? "var(--muted)" : "var(--orange)",
            border: `1px solid ${isRacing ? "var(--border)" : "var(--orange)"}`,
            background: "transparent",
            cursor: isRacing ? "not-allowed" : "pointer",
            marginLeft: newModels.length > 0 ? "0" : "auto",
          }}
        >
          {isRacing ? "running…" : "▶ re-run benchmarks"}
        </button>
      </div>
      <div className="text-xl font-bold mb-1" style={{ color: "var(--text)" }}>
        {problem.title}
      </div>
      <div className="text-sm font-semibold" style={{ color: "var(--orange)" }}>
        How fast would AI solve this?
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify dev server shows updated header**

The problem header should now show: `#1 · Easy badge · problem title · orange subtitle`. The `solved` prop and `onRaceAgain`/`isRacing` props are new — `page.tsx` will pass them in Task 7. For now TypeScript will complain about missing props; that's fine, it'll be fixed in Task 7.

- [ ] **Step 3: Commit**

```bash
git add frontend/app/components/ProblemHeader.tsx
git commit -m "style: restyle ProblemHeader to LeetCode aesthetic"
```

---

## Task 3: Create WinnerCard

**Files:**
- Create: `frontend/app/components/WinnerCard.tsx`

- [ ] **Step 1: Create the file**

```tsx
import { RaceResultWithModel } from "@/app/lib/types";
import { formatTime } from "@/app/lib/api";

interface Props {
  winner: RaceResultWithModel;
}

export default function WinnerCard({ winner }: Props) {
  return (
    <div
      className="flex items-center gap-4 px-5 py-4 border-b"
      style={{
        borderColor: "var(--border)",
        borderLeft: "4px solid var(--orange)",
        background: "var(--surface-2)",
      }}
    >
      <span className="text-2xl">🥇</span>
      <div className="flex-1 min-w-0">
        <div className="text-xs mb-1" style={{ color: "var(--muted)", letterSpacing: "0.1em", textTransform: "uppercase" }}>
          Winner
        </div>
        <div className="text-xl font-bold truncate" style={{ color: "var(--text)" }}>
          {winner.display_name}
        </div>
        <div className="text-xs mt-0.5" style={{ color: "var(--muted)" }}>
          {winner.provider}
        </div>
      </div>
      <div className="text-right flex-shrink-0">
        <div
          className="font-extrabold leading-none"
          style={{ fontSize: "48px", color: "var(--orange)", letterSpacing: "-2px" }}
        >
          {formatTime(winner.time_ms)}
        </div>
        <div className="text-xs mt-1" style={{ color: "var(--muted)" }}>runtime</div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cd frontend && npx tsc --noEmit
```

Expected: no errors related to `WinnerCard.tsx`. (Other errors from Task 2's new props are fine.)

- [ ] **Step 3: Commit**

```bash
git add frontend/app/components/WinnerCard.tsx
git commit -m "feat: add WinnerCard component"
```

---

## Task 4: Update RaceResults leaderboard

**Files:**
- Modify: `frontend/app/components/RaceResults.tsx`

- [ ] **Step 1: Replace the component**

```tsx
"use client";
import { useState } from "react";
import { formatTime } from "@/app/lib/api";
import { RaceResultWithModel } from "@/app/lib/types";

interface Props {
  results: RaceResultWithModel[];
  userResult: { ms: number; gaveUp: boolean } | null;
  onSelectResult: (result: RaceResultWithModel) => void;
}

export default function RaceResults({ results, userResult, onSelectResult }: Props) {
  const [expanded, setExpanded] = useState(false);

  const sorted = [...results]
    .filter(r => !r.is_human)
    .sort((a, b) => {
      if (a.solved && b.solved) return (a.time_ms ?? 0) - (b.time_ms ?? 0);
      if (a.solved) return -1;
      if (b.solved) return 1;
      return 0;
    });

  const maxTime = Math.max(...sorted.filter(r => r.solved).map(r => r.time_ms ?? 0), 1);

  // Insert user result at correct rank position
  const withUser: Array<RaceResultWithModel | { isUser: true; ms: number; gaveUp: boolean }> = [...sorted];
  if (userResult) {
    const insertAt = userResult.gaveUp
      ? withUser.length
      : withUser.findIndex(r => r.solved && (r.time_ms ?? 0) > userResult.ms);
    const idx = insertAt === -1 ? withUser.length : insertAt;
    withUser.splice(idx, 0, { isUser: true, ...userResult });
  }

  const visible = expanded ? withUser : withUser.slice(0, 5);
  const hasMore = withUser.length > 5;

  return (
    <div>
      <div
        className="px-5 py-2 text-xs font-semibold border-b"
        style={{ color: "var(--muted)", letterSpacing: "0.15em", textTransform: "uppercase", borderColor: "var(--border)" }}
      >
        All Results
      </div>

      {visible.map((entry, i) => {
        const isUser = "isUser" in entry;

        if (isUser) {
          const pct = entry.gaveUp ? 100 : Math.max(6, (entry.ms / maxTime) * 100);
          return (
            <div
              key="you"
              className="px-5 py-2.5 border-b"
              style={{ borderColor: "var(--border)" }}
            >
              <div className="flex items-center gap-2 mb-1.5">
                <span className="text-xs w-4" style={{ color: "var(--muted)" }}>{i + 1}</span>
                <span className="text-sm font-semibold flex-1" style={{ color: "var(--orange)" }}>You</span>
                <span className="text-sm font-bold" style={{ color: entry.gaveUp ? "var(--red)" : "var(--orange)" }}>
                  {entry.gaveUp ? "gave up" : formatTime(entry.ms)}
                </span>
              </div>
              <div className="ml-6 h-1.5 rounded overflow-hidden" style={{ background: "#2e2e2e" }}>
                <div className="h-full rounded" style={{ width: `${pct}%`, background: entry.gaveUp ? "var(--red)" : "var(--orange)" }} />
              </div>
            </div>
          );
        }

        const r = entry as RaceResultWithModel;
        const barPct = r.solved && r.time_ms ? Math.max(6, (r.time_ms / maxTime) * 100) : 100;
        const isWinner = i === 0 && r.solved;

        return (
          <button
            key={r.model_id}
            className="w-full px-5 py-2.5 border-b text-left"
            style={{ borderColor: "var(--border)" }}
            onClick={() => onSelectResult(r)}
          >
            <div className="flex items-center gap-2 mb-1.5">
              <span className="text-xs w-4" style={{ color: "var(--muted)" }}>
                {i === 0 ? "🥇" : i === 1 ? "🥈" : i === 2 ? "🥉" : i + 1}
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
        );
      })}

      {/* Avg human row — always last */}
      <div className="px-5 py-2.5" style={{ background: "rgba(239,71,67,0.04)" }}>
        <div className="flex items-center gap-2 mb-1.5">
          <span className="text-xs w-4">👤</span>
          <span className="text-sm flex-1" style={{ color: "var(--muted)" }}>avg human</span>
          <span className="text-sm font-bold" style={{ color: "var(--red)" }}>~15 min</span>
        </div>
        <div className="ml-6 h-1.5 rounded overflow-hidden" style={{ background: "#2e2e2e" }}>
          <div className="h-full rounded" style={{ width: "100%", background: "var(--red)", opacity: 0.35 }} />
        </div>
        <div className="ml-6 mt-1 text-xs" style={{ color: "var(--muted)" }}>No comment.</div>
      </div>

      {hasMore && (
        <button
          className="w-full py-2 text-xs border-t"
          style={{ color: "var(--muted)", borderColor: "var(--border)" }}
          onClick={() => setExpanded(e => !e)}
        >
          {expanded ? "show less ↑" : `show ${withUser.length - 5} more ↓`}
        </button>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cd frontend && npx tsc --noEmit
```

Expected: no errors in `RaceResults.tsx`. (The `onRaceAgain` and `isRacing` props were removed from this component — they moved to `ProblemHeader`. The `page.tsx` error about those props is expected and will be fixed in Task 7.)

- [ ] **Step 3: Commit**

```bash
git add frontend/app/components/RaceResults.tsx
git commit -m "style: restyle RaceResults leaderboard to LeetCode aesthetic"
```

---

## Task 5: Restyle SearchBar for nav

**Files:**
- Modify: `frontend/app/components/SearchBar.tsx`

- [ ] **Step 1: Replace only the JSX return and `diffColor` function**

The logic (debounce, outside-click) stays identical. Only the visual markup changes.

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

  return (
    <div ref={ref} className="relative flex items-center gap-2 flex-1" style={{ maxWidth: "360px" }}>
      <input
        className="w-full rounded px-3 py-1.5 text-sm outline-none"
        style={{
          background: "#3a3a3a",
          border: "1px solid #4a4a4a",
          color: "var(--text)",
        }}
        placeholder="Search problems… Two Sum, #42, Hard"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        onFocus={() => results.length > 0 && setOpen(true)}
      />
      <button
        onClick={onRandom}
        className="rounded px-3 py-1.5 text-sm flex-shrink-0"
        style={{ background: "#3a3a3a", border: "1px solid #4a4a4a", color: "var(--text)" }}
      >
        🎲
      </button>

      {open && (
        <div
          className="absolute left-0 top-full z-20 rounded-b w-full text-sm mt-0.5"
          style={{ background: "var(--surface)", border: "1px solid var(--border)" }}
        >
          {results.map((p) => (
            <button
              key={p.id}
              className="flex w-full items-center gap-3 px-3 py-2 text-left border-b"
              style={{ borderColor: "var(--border)" }}
              onClick={() => { onSelect(p); setOpen(false); setQuery(""); }}
            >
              <span className="text-xs flex-shrink-0" style={{ color: "var(--muted)" }}>#{p.lc_id}</span>
              <span className="flex-1 truncate" style={{ color: "var(--text)" }}>{p.title}</span>
              <span
                className="text-xs flex-shrink-0"
                style={
                  p.difficulty === "Easy"
                    ? { color: "var(--green)" }
                    : p.difficulty === "Medium"
                    ? { color: "var(--orange)" }
                    : { color: "var(--red)" }
                }
              >
                {p.difficulty}
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add frontend/app/components/SearchBar.tsx
git commit -m "style: restyle SearchBar for inline nav use"
```

---

## Task 6: Create RaceEditor (replaces YouBanner)

**Files:**
- Create: `frontend/app/components/RaceEditor.tsx`
- Delete: `frontend/app/components/YouBanner.tsx`

This is the right panel: race simulation + code editor + submit.

- [ ] **Step 1: Create `RaceEditor.tsx`**

```tsx
"use client";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTimer } from "@/app/hooks/useTimer";
import { formatTime } from "@/app/lib/api";
import { Problem, RaceResultWithModel } from "@/app/lib/types";

interface Props {
  problem: Problem;
  results: RaceResultWithModel[];
  onSolve: (ms: number) => void;
  onGiveUp: (ms: number) => void;
  userResult: { ms: number; gaveUp: boolean } | null;
}

type RacePhase = "idle" | "racing" | "submitted";

// Top 3 AI results (non-human, solved, sorted by time)
function getTopAIs(results: RaceResultWithModel[]): RaceResultWithModel[] {
  return results
    .filter(r => !r.is_human && r.solved && r.time_ms != null)
    .sort((a, b) => (a.time_ms ?? 0) - (b.time_ms ?? 0))
    .slice(0, 3);
}

function getRoastText(
  phase: RacePhase,
  solvedIds: Set<string>,
  topAIs: RaceResultWithModel[]
): string | null {
  if (phase === "submitted") return "Noted.";
  if (phase === "idle") return null;
  // racing
  if (solvedIds.size === 0) {
    const fastest = topAIs[0];
    if (!fastest) return null;
    return `${fastest.display_name} finishes in ${((fastest.time_ms ?? 0) / 1000).toFixed(1)}s.`;
  }
  if (solvedIds.size === topAIs.length) return "All models have submitted. No pressure.";
  const firstSolved = topAIs.find(r => solvedIds.has(r.model_id));
  return firstSolved ? `${firstSolved.display_name} has already moved on with its life.` : null;
}

export default function RaceEditor({ problem, results, onSolve, onGiveUp, userResult }: Props) {
  const [phase, setPhase] = useState<RacePhase>("idle");
  const [solvedIds, setSolvedIds] = useState<Set<string>>(new Set());
  const [code, setCode] = useState(problem.starter_code ?? "");
  const timeoutRefs = useRef<ReturnType<typeof setTimeout>[]>([]);
  const { state: timerState, elapsedMs, start, stop, reset } = useTimer();

  const topAIs = getTopAIs(results);

  // Reset when problem changes
  useEffect(() => {
    setPhase("idle");
    setSolvedIds(new Set());
    setCode(problem.starter_code ?? "");
    reset();
    timeoutRefs.current.forEach(clearTimeout);
    timeoutRefs.current = [];
  }, [problem.id, reset]);

  // Clear timeouts on unmount
  useEffect(() => {
    return () => { timeoutRefs.current.forEach(clearTimeout); };
  }, []);

  const startRace = useCallback(() => {
    if (phase !== "idle") return;
    setPhase("racing");
    start();
    // Schedule each AI's "solved" event at their benchmarked time
    topAIs.forEach(ai => {
      const t = setTimeout(() => {
        setSolvedIds(prev => new Set([...prev, ai.model_id]));
      }, ai.time_ms!);
      timeoutRefs.current.push(t);
    });
  }, [phase, start, topAIs]);

  const handleInput = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setCode(e.target.value);
    startRace();
  };

  const handleSubmit = () => {
    const ms = stop();
    setPhase("submitted");
    onSolve(ms);
  };

  const handleGiveUp = () => {
    const ms = stop();
    setPhase("submitted");
    onGiveUp(ms);
  };

  const roastText = getRoastText(phase, solvedIds, topAIs);
  const maxAITime = topAIs.length > 0 ? (topAIs[topAIs.length - 1].time_ms ?? 1) : 1;
  const userPct = timerState === "running" && maxAITime > 0
    ? Math.min(100, (elapsedMs / maxAITime) * 100)
    : timerState === "stopped"
    ? Math.min(100, ((userResult?.ms ?? elapsedMs) / maxAITime) * 100)
    : 0;

  return (
    <div className="flex flex-col h-full">

      {/* Race panel */}
      <div className="px-5 py-3 border-b" style={{ background: "var(--surface-2)", borderColor: "var(--border)" }}>
        <div className="flex items-center justify-between mb-3">
          <span className="text-xs font-semibold" style={{ color: "var(--muted)", letterSpacing: "0.15em", textTransform: "uppercase" }}>
            Live Race
          </span>
          {phase !== "idle" && (
            <span className="text-xs" style={{ color: "var(--muted)" }}>
              Your time:{" "}
              <span className="font-bold" style={{ color: "var(--orange)", fontVariantNumeric: "tabular-nums" }}>
                {timerState === "stopped" && userResult
                  ? formatTime(userResult.ms)
                  : formatTime(elapsedMs)}
              </span>
            </span>
          )}
        </div>

        <div className="flex flex-col gap-2">
          {/* You row */}
          <div className="flex items-center gap-3">
            <span className="text-xs font-semibold w-20 flex-shrink-0" style={{ color: "var(--orange)" }}>You</span>
            <div className="flex-1 h-1.5 rounded overflow-hidden" style={{ background: "#3a3a3a" }}>
              <div
                className="h-full rounded"
                style={{ width: `${userPct}%`, background: "var(--orange)", transition: "width 0.1s linear" }}
              />
            </div>
            <span className="text-xs font-bold w-14 text-right flex-shrink-0" style={{ color: phase === "idle" ? "#555" : "var(--orange)", fontVariantNumeric: "tabular-nums" }}>
              {phase === "idle" ? "–" : timerState === "stopped" && userResult ? formatTime(userResult.ms) : formatTime(elapsedMs)}
            </span>
          </div>

          {/* AI rows */}
          {topAIs.map(ai => {
            const solved = solvedIds.has(ai.model_id);
            const targetPct = (ai.time_ms! / maxAITime) * 100;
            return (
              <div key={ai.model_id} className="flex items-center gap-3">
                <span className="text-xs font-semibold w-20 flex-shrink-0 truncate" style={{ color: solved ? "var(--orange)" : "var(--text)" }}>
                  {ai.display_name}
                </span>
                <div className="flex-1 h-1.5 rounded overflow-hidden" style={{ background: "#3a3a3a" }}>
                  <div
                    className="h-full rounded"
                    style={{
                      width: phase === "idle" ? "0%" : `${targetPct}%`,
                      background: solved ? "var(--orange)" : "#5c5c5c",
                      transition: phase === "idle" ? "none" : `width ${(ai.time_ms! / 1000).toFixed(2)}s linear`,
                    }}
                  />
                </div>
                <span
                  className="text-xs font-bold w-14 text-right flex-shrink-0"
                  style={{ color: solved ? "var(--orange)" : "#555", fontVariantNumeric: "tabular-nums" }}
                >
                  {phase === "idle" ? "–" : solved ? `${((ai.time_ms!) / 1000).toFixed(1)}s ✓` : "…"}
                </span>
              </div>
            );
          })}
        </div>
      </div>

      {/* Roast banner */}
      {roastText && (
        <div
          className="px-5 py-2 text-xs italic border-b"
          style={{
            background: "#1e1000",
            borderLeft: "3px solid var(--orange)",
            borderColor: "#3a2800",
            borderBottomColor: "#3a2800",
            color: "var(--muted)",
          }}
        >
          {roastText}
        </div>
      )}

      {/* Editor toolbar */}
      <div className="flex items-center gap-3 px-4 py-2 border-b" style={{ background: "#2d2d2d", borderColor: "var(--border)" }}>
        <select
          className="text-xs rounded px-2 py-1"
          style={{ background: "#3a3a3a", border: "1px solid #4a4a4a", color: "var(--text)" }}
        >
          <option>Python 3</option>
          <option>JavaScript</option>
          <option>TypeScript</option>
          <option>Java</option>
          <option>C++</option>
        </select>
        {phase === "idle" && (
          <span className="text-xs ml-auto" style={{ color: "#555", fontStyle: "italic" }}>
            Start typing to begin the race.
          </span>
        )}
      </div>

      {/* Code editor */}
      <textarea
        className="flex-1 px-4 py-3 text-sm outline-none resize-none min-h-0"
        style={{
          background: "#1e1e1e",
          color: "var(--text)",
          fontFamily: "'Courier New', monospace",
          lineHeight: "1.65",
          // Mobile fallback height
          minHeight: "200px",
        }}
        value={code}
        onChange={handleInput}
        disabled={phase === "submitted"}
        spellCheck={false}
      />

      {/* Submit bar */}
      <div className="flex items-center gap-3 px-4 py-2.5 border-t" style={{ background: "var(--surface)", borderColor: "var(--border)" }}>
        <button
          onClick={handleSubmit}
          disabled={phase !== "racing"}
          className="rounded px-5 py-1.5 text-sm font-bold"
          style={{
            background: phase === "racing" ? "var(--orange)" : "#3a3a3a",
            color: phase === "racing" ? "#000" : "var(--muted)",
            cursor: phase === "racing" ? "pointer" : "not-allowed",
          }}
        >
          Submit
        </button>
        <span className="text-xs italic flex-1" style={{ color: "#555" }}>
          {phase === "submitted" ? "We told you so." : "We're not going to run it."}
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

    </div>
  );
}
```

- [ ] **Step 2: Delete `YouBanner.tsx`**

```bash
rm frontend/app/components/YouBanner.tsx
```

- [ ] **Step 3: Verify it compiles**

```bash
cd frontend && npx tsc --noEmit 2>&1 | grep -v "page.tsx"
```

Expected: no errors in `RaceEditor.tsx`. Errors in `page.tsx` about `YouBanner` are expected and fixed in Task 7.

- [ ] **Step 4: Commit**

```bash
git add frontend/app/components/RaceEditor.tsx
git rm frontend/app/components/YouBanner.tsx
git commit -m "feat: add RaceEditor component (replaces YouBanner)"
```

---

## Task 7: Restructure page.tsx — two-column layout

**Files:**
- Modify: `frontend/app/page.tsx`

- [ ] **Step 1: Replace the full file**

```tsx
"use client";
import { useCallback, useEffect, useRef, useState } from "react";
import { createRace, fetchModels, fetchProblemResults, fetchRandomProblem } from "./lib/api";
import { Model, Problem, RaceResultWithModel } from "./lib/types";
import MemeCard from "./components/MemeCard";
import ProblemHeader from "./components/ProblemHeader";
import RaceResults from "./components/RaceResults";
import SearchBar from "./components/SearchBar";
import WinnerCard from "./components/WinnerCard";
import RaceEditor from "./components/RaceEditor";

export default function Home() {
  const [problem, setProblem] = useState<Problem | null>(null);
  const [results, setResults] = useState<RaceResultWithModel[]>([]);
  const [models, setModels] = useState<Model[]>([]);
  const [isRacing, setIsRacing] = useState(false);
  const [userResult, setUserResult] = useState<{ ms: number; gaveUp: boolean } | null>(null);
  const [memeTarget, setMemeTarget] = useState<RaceResultWithModel | null>(null);
  const [roast, setRoast] = useState("");
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

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

  useEffect(() => {
    return () => { if (pollRef.current) clearInterval(pollRef.current); };
  }, []);

  const handleRaceAgain = async () => {
    if (!problem || isRacing) return;
    setIsRacing(true);
    try {
      await createRace(problem.id);
      let attempts = 0;
      let stableCount = 0;
      let lastCount = results.length;
      pollRef.current = setInterval(async () => {
        const r = await fetchProblemResults(problem.id);
        setResults(r);
        attempts++;
        if (r.length > lastCount) { lastCount = r.length; stableCount = 0; }
        else { stableCount++; }
        if (attempts > 40 || stableCount >= 3) {
          clearInterval(pollRef.current!);
          pollRef.current = null;
          setIsRacing(false);
        }
      }, 3000);
    } catch {
      setIsRacing(false);
    }
  };

  const handleSelectResult = (r: RaceResultWithModel) => {
    if (!problem) return;
    setMemeTarget(r);
    const loser = results.find(x => x.model_id !== r.model_id && x.solved) ?? results[results.length - 1];
    if (loser) setRoast(`${r.display_name} left ${loser.display_name} in the dust`);
  };

  const winner = results
    .filter(r => !r.is_human && r.solved && r.time_ms != null)
    .sort((a, b) => (a.time_ms ?? 0) - (b.time_ms ?? 0))[0] ?? null;

  const newModels = models.filter(m => m.is_new);

  if (!problem) {
    return (
      <div className="flex items-center justify-center min-h-screen text-sm" style={{ color: "var(--muted)" }}>
        loading...
      </div>
    );
  }

  return (
    <div className="flex flex-col" style={{ height: "100dvh" }}>

      {/* Nav */}
      <nav
        className="flex items-center gap-4 px-5 shrink-0 border-b"
        style={{ height: "44px", background: "var(--surface)", borderColor: "var(--border)" }}
      >
        <div className="font-extrabold text-sm whitespace-nowrap" style={{ color: "var(--text)" }}>
          howfast<span style={{ color: "var(--orange)" }}>would</span>.com
        </div>
        <SearchBar onSelect={loadProblem} onRandom={loadRandom} />
        <div className="hidden lg:flex gap-5 text-sm ml-auto" style={{ color: "var(--muted)" }}>
          <span>Leaderboard</span>
          <span>About</span>
        </div>
      </nav>

      {/* Content */}
      <div className="flex flex-col lg:flex-row flex-1 min-h-0">

        {/* Left panel — problem info + leaderboard */}
        <div
          className="w-full lg:w-[420px] lg:flex-shrink-0 lg:border-r lg:overflow-y-auto flex flex-col"
          style={{ borderColor: "var(--border)" }}
        >
          <ProblemHeader
            problem={problem}
            newModels={newModels}
            solved={userResult !== null && !userResult.gaveUp}
            onRaceAgain={handleRaceAgain}
            isRacing={isRacing}
          />
          {winner && <WinnerCard winner={winner} />}
          <RaceResults
            results={results}
            userResult={userResult}
            onSelectResult={handleSelectResult}
          />
        </div>

        {/* Right panel — race editor (desktop: fixed height, mobile: natural) */}
        <div className="flex-1 flex flex-col min-h-0">
          <RaceEditor
            problem={problem}
            results={results}
            onSolve={(ms) => setUserResult({ ms, gaveUp: false })}
            onGiveUp={(ms) => setUserResult({ ms, gaveUp: true })}
            userResult={userResult}
          />
        </div>

      </div>

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

- [ ] **Step 2: Verify it compiles cleanly**

```bash
cd frontend && npx tsc --noEmit
```

Expected: 0 errors.

- [ ] **Step 3: Run dev server and do a full visual check**

```bash
cd frontend && npm run dev
```

Verify on desktop (browser ≥ 1024px wide):
- Two-column layout: left = problem info + leaderboard, right = editor
- Nav has logo + search bar + links
- Orange "How fast would AI solve this?" in problem header
- WinnerCard shows with big orange time
- Race bars are idle (`–`) until you type
- Typing in the editor starts the race — AI bars animate
- Submit button activates once you start typing
- Roast banner appears after first AI "finishes"
- After submit, "You" row appears in leaderboard at correct rank

Verify on mobile (browser < 1024px):
- Single column: nav → problem header → race panel → editor → leaderboard
- Editor is at least 200px tall
- Nav links hidden

- [ ] **Step 4: Commit**

```bash
git add frontend/app/page.tsx
git commit -m "feat: two-column LeetCode layout with RaceEditor"
```

---

## Task 8: Deploy and verify production

**Files:** none (deployment only)

- [ ] **Step 1: Push to master**

```bash
git push origin master
```

- [ ] **Step 2: Wait for Vercel deployment**

Watch the Vercel dashboard or run:
```bash
gh run list --limit 5
```

Expected: deployment completes in ~2 minutes.

- [ ] **Step 3: Verify production at howfastwould.com**

- [ ] Desktop two-column layout renders correctly
- [ ] Race simulation works (type in editor → AI bars animate)
- [ ] No double-slash API calls in browser network tab (the existing `NEXT_PUBLIC_API_URL` trailing slash issue — if you see `//models` in network requests, remove the trailing slash from the `NEXT_PUBLIC_API_URL` env var in Vercel)

- [ ] **Step 4: Done**
