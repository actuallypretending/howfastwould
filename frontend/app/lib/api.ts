import { CreateRaceResponse, ExecutionDetails, LeaderboardEntry, Model, Problem, RaceResultWithModel, RunResult, SubmitResult } from "./types";

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

export async function fetchLeaderboard(): Promise<LeaderboardEntry[]> {
  const res = await fetch(`${BASE}/leaderboard`);
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

export class RateLimitError extends Error {
  retryAfter: number;
  constructor(retryAfter: number) {
    super(`Rate limited. Try again in ${retryAfter}s`);
    this.retryAfter = retryAfter;
  }
}

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

export async function fetchResultDetails(resultId: string): Promise<ExecutionDetails> {
  const res = await fetch(`${BASE}/results/${resultId}/details`);
  if (!res.ok) throw new Error("no execution details");
  return res.json();
}
