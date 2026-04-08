import { CreateRaceResponse, LeaderboardEntry, Model, Problem, RaceResultWithModel } from "./types";

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
