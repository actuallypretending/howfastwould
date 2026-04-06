export interface Problem {
  id: string;
  lc_id: number;
  title: string;
  difficulty: "Easy" | "Medium" | "Hard";
  description: string;
  starter_code: string;
  test_cases: string;
  source: string;
  cached_at: string;
}

export interface Model {
  id: string;
  provider: string;
  name: string;
  display_name: string;
  is_active: boolean;
  is_new: boolean;
  is_human: boolean;
  human_times: string | null;
  added_at: string;
}

export interface RaceResultWithModel {
  model_id: string;
  model_name: string;
  display_name: string;
  provider: string;
  is_human: boolean;
  solved: boolean;
  time_ms: number | null;
  attempts: number;
  run_at: string;
}

export interface RaceEvent {
  race_id: string;
  model_id: string;
  display_name: string;
  status: "running" | "solved" | "failed";
  time_ms: number | null;
  attempts: number;
}

export interface CreateRaceResponse {
  race_id: string;
}
