# Problem Content & Auto-Benchmark — Design Spec

**Date:** 2026-04-08
**Status:** Approved

## Problem

1. **Users can't see the problem while coding.** The problem description lives in the left panel, but the code editor takes the full right panel. On smaller screens there's no split view at all.
2. **Benchmarks don't auto-appear.** When a user loads a problem, the backend auto-triggers missing benchmarks — but the frontend only fetches results once. New results silently appear server-side and the user never sees them unless they manually re-run.
3. **Test cases have no expected outputs.** `parse_test_cases()` in `leetcode.rs` only captures inputs from `exampleTestcaseList`. Expected outputs are available in the `content` HTML field but aren't extracted.

## Solution

### 1. Side-by-Side Problem Panel in RaceEditor

Add a collapsible problem panel to the left side of RaceEditor (inside the right panel area).

**Desktop (≥1024px):**
- Problem + test cases: 40% width
- Code editor area: 60% width
- Horizontal split, side-by-side

**Mobile (<1024px):**
- Problem collapses into a toggleable section above the editor
- Collapsed by default to maximize editor space
- Toggle button in the editor toolbar

**Problem panel contents:**
- Problem description (rendered HTML from `problem.description`)
- Test cases with inputs AND expected outputs (once Task 3 lands)
- Scrollable independently from the editor

### 2. Auto-Poll Benchmarks on Page Load

After `fetchProblemResults()` returns, if the result count is less than the number of active non-human models, start polling every 3s. Stop when:
- Result count stabilizes for 3 consecutive polls, OR
- 40 polls (2 minutes) elapse

This reuses the exact same polling logic as `handleRaceAgain` in `page.tsx:46-71`, just triggered automatically on problem load instead of only on button click.

### 3. Parse Expected Outputs from LeetCode

The LeetCode `content` HTML field contains example outputs in this pattern:
```html
<strong>Output:</strong> [value]
```

Extract these values in `leetcode.rs::parse_test_cases()` and pair them with the corresponding inputs from `exampleTestcaseList`. The `TestCase` struct already has an `expected_output: String` field — it's just always empty today.

## Non-Goals

- Resizable split pane (drag handle) — fixed 40/60 is fine
- Syntax highlighting in the problem panel — raw HTML rendering is enough
- Running user code against test cases — this is a meme site, not a judge
