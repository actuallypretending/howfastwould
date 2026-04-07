# Frontend Redesign Design

## Overview

Redesign howfastwould.com from its current dark monospace terminal aesthetic to a LeetCode-inspired dark theme with orange accents. Add a live race feature: the user can type a solution in a code editor, and the moment they start typing, pre-benchmarked AI timers begin counting on screen.

---

## Aesthetic

**Source of truth:** LeetCode dark mode.

| Token | Value | Usage |
|-------|-------|-------|
| `--bg` | `#1a1a1a` | Page background |
| `--surface` | `#282828` | Cards, panels |
| `--surface-2` | `#222222` | Race panel, editor bg |
| `--border` | `#3a3a3a` | All dividers |
| `--orange` | `#FFA116` | Winner, accents, You row |
| `--text` | `#eff1f6` | Primary text |
| `--muted` | `#888888` | Secondary text |
| `--green` | `#00b8a3` | Easy badge |
| `--red` | `#ef4743` | Hard badge, failures |

**Font:** System sans-serif (`-apple-system, 'Segoe UI', Arial, sans-serif`) for all UI. `'Courier New', monospace` only inside the code editor.

---

## Layout

### Desktop (≥ 1024px)

Full-height two-column split, like LeetCode's problem page:

```
┌─────────────────────────────────────────────────────┐
│ NAV: logo · search bar (flex-1, max 360px) · links  │
├─────────────────────┬───────────────────────────────┤
│  LEFT (420px fixed) │  RIGHT (flex-1)               │
│                     │                               │
│  ProblemHeader      │  RacePanel                    │
│  WinnerCard         │  RoastBanner (contextual)     │
│  Leaderboard        │  EditorToolbar                │
│                     │  CodeEditor (flex-1)          │
│                     │  SubmitBar                    │
└─────────────────────┴───────────────────────────────┘
```

Left panel scrolls independently. Right panel fills viewport height with the code editor taking the remaining space (`flex: 1`, `min-height: 0`).

### Mobile (< 1024px)

Single column, top to bottom:

1. Nav (logo + search)
2. ProblemHeader
3. RacePanel
4. RoastBanner
5. EditorToolbar + CodeEditor
6. SubmitBar
7. WinnerCard
8. Leaderboard

---

## Components

### `globals.css`

Replace current variables and font:
- Remove JetBrains Mono as the body font
- Add new CSS variable set above
- Keep `.difficulty-easy/medium/hard` classes but update colors to match LeetCode palette

### `page.tsx`

Restructure the root layout:
- Nav becomes full-width with inline search
- Below nav: `flex flex-col lg:flex-row` container
- Left panel: `w-full lg:w-[420px] lg:flex-shrink-0 lg:border-r lg:overflow-y-auto`
- Right panel: `flex-1 flex flex-col`
- On mobile, WinnerCard and Leaderboard render below the editor
- On desktop, WinnerCard and Leaderboard are in the left panel

### `ProblemHeader.tsx`

Update to LeetCode style:
- Row 1: `#[lc_id]` (muted) · difficulty badge (pill, color-coded) · "Accepted" pill if user solved
- Row 2: Problem title (20–22px, bold)
- Row 3: `"How fast would AI solve this?"` in `--orange`, 13px
- Remove the description excerpt (not needed in this design)
- Keep `newModels` badge (subtle, top-right)

### `WinnerCard.tsx` *(new component)*

Extracted from the leaderboard. Shows the fastest solved result prominently:
- Trophy emoji + model name (22px bold) + provider (11px muted)
- Big time number (48px, `--orange`)
- Left border accent: 4px solid `--orange`
- Background: `--surface-2`

### `RaceResults.tsx`

Update leaderboard style to match mockup:
- Section header: "All Results" (10px, `--muted`, uppercase, letter-spacing)
- Each row: rank · name (bold) · horizontal bar · time
  - Bar track: 6px tall, `#2e2e2e` background
  - Winner bar: `--orange` fill
  - Other bars: `#5c5c5c` fill
  - Bar widths: proportional to `time_ms` relative to max, minimum 6%
- "You" row injected at correct rank position after submission, `--orange` color
- Avg human row always last: red bar at 100% width, muted label, deadpan note ("No comment.")
- Remove the `▶ race again` button from this component — move it to the ProblemHeader as a small secondary button ("▶ re-run benchmarks") next to the difficulty badge
- Keep expand/collapse for > 5 results

### `YouBanner.tsx` → `RaceEditor.tsx`

Rename the file to `RaceEditor.tsx`. The component becomes the entire right panel (race + editor + submit). Update the import in `page.tsx` accordingly.

**Race panel** (top of right column):
- Header row: "Live Race" label (uppercase, muted) + "Your time: X.Xs" counter (hidden until race starts)
- Four rows (You + top 3 AIs from results): name · animated bar · status
  - "You" row: `--orange` color
  - AI rows: grey until solved, `--orange` when solved
  - Status: `–` (idle) → `X.Xs` counting (racing, shown for You) → `X.Xs ✓` (solved, shown for AIs)

**Race simulation behavior:**
- Race is **idle** until first keypress in the editor
- On first keypress: record `raceStartMs = Date.now()`, start the user's `useTimer`
- For each AI in `results` that has `time_ms`: schedule a `setTimeout` at `time_ms` to mark that AI as solved
- Bar animation: on race start, set each AI bar's CSS `transition: width {time_ms / 1000}s linear` and target width, so they animate to their final position at the correct speed
- You bar grows in real-time with the user timer

**Roast banner** (between race panel and editor):
- Hidden when idle
- Shows one line of deadpan text, updates as race progresses:
  - Race started (first AI not yet done): `"GPT-4o started {N}s ago."` (uses fastest AI's time_ms)
  - First AI finishes: `"{Model} has already moved on with its life."`
  - All AIs finish: `"All models have submitted. No pressure."`
  - After user submits: `"Noted."`
- Style: dark amber background (`#1e1000`), left border `--orange`, italic, muted text color

**Code editor:**
- Toolbar: language selector dropdown (Python 3 default) + hint text (`"Start typing to begin the race."`, disappears on first keypress)
- `<textarea>` pre-filled with `problem.starter_code`, monospace font, `#1e1e1e` bg
- No syntax highlighting (keep it simple)
- Height: fills remaining right panel space on desktop; fixed `200px` on mobile

**Submit bar:**
- "Submit" button (`--orange` bg, black text) — disabled until race has started
- Sub-label: `"We're not going to run it."` → changes to `"We told you so."` after submission
- "Give up" link (text only, muted) still available

**Timer behavior change from current:**
Current `YouBanner` starts the timer when the problem loads. New behavior: timer starts on first keypress. The `useTimer` hook is unchanged; `start()` is called from the `onInput` handler on first keystroke instead of in a `useEffect`.

---

## Roast Copy

| Trigger | Text |
|---------|------|
| Race started, fastest AI not yet done | `"GPT-4o started {N}s ago."` |
| Fastest AI finishes | `"{Model} has already moved on with its life."` |
| All AIs finished | `"All models have submitted. No pressure."` |
| User submits | `"Noted."` |
| Submit sub-label after submit | `"We told you so."` |
| Avg human leaderboard note | `"No comment."` |

---

## What is NOT changing

- API layer (`lib/api.ts`, `lib/types.ts`) — no changes
- `useTimer` hook — no changes
- `MemeCard.tsx` — no changes (shareable card feature unchanged)
- `SearchBar.tsx` — restyled to match new theme but functionally identical
- Backend — no changes

---

## Mobile Considerations

- Left/right panels stack vertically
- Code editor height is fixed at 200px on mobile (not flex-fill)
- Race panel stays above the editor in the stack
- Winner card and leaderboard move below the editor on mobile
- Nav shows logo + search only (hide nav links on small screens)
