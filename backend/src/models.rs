use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Problem {
    pub id: String,
    pub lc_id: i64,
    pub title: String,
    pub difficulty: String,
    pub description: String,
    pub starter_code: String,
    pub test_cases: String,
    pub source: String,
    pub cached_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub input: String,
    pub expected_output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Model {
    pub id: String,
    pub provider: String,
    pub name: String,
    pub display_name: String,
    #[serde(skip)]
    pub api_key_env: String,
    pub is_active: bool,
    pub is_new: bool,
    pub is_human: bool,
    pub human_times: Option<String>,
    pub added_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanTimes {
    #[serde(rename = "Easy")]
    pub easy: i64,
    #[serde(rename = "Medium")]
    pub medium: i64,
    #[serde(rename = "Hard")]
    pub hard: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RaceResult {
    pub id: String,
    pub problem_id: String,
    pub model_id: String,
    pub solved: bool,
    pub time_ms: Option<i64>,
    pub attempts: i64,
    pub run_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceResultWithModel {
    pub model_id: String,
    pub model_name: String,
    pub display_name: String,
    pub provider: String,
    pub is_human: bool,
    pub solved: bool,
    pub time_ms: Option<i64>,
    pub attempts: i64,
    pub run_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceEvent {
    pub race_id: String,
    pub model_id: String,
    pub display_name: String,
    pub status: RaceStatus,
    pub time_ms: Option<i64>,
    pub attempts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RaceStatus {
    Running,
    Solved,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Race {
    pub id: String,
    pub problem_id: String,
    pub started_at: String,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LeaderboardEntry {
    pub model_id: String,
    pub display_name: String,
    pub provider: String,
    pub total: i64,
    pub solved: i64,
    pub avg_time_ms: Option<i64>,
    pub median_time_ms: Option<i64>,
    pub win_count: i64,
}
