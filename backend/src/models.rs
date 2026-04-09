use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Problem {
    pub id: String,
    pub lc_id: i64,
    pub title: String,
    pub difficulty: String,
    pub description: String,
    pub starter_code: Value,
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
    #[sqlx(skip)]
    #[serde(skip)]
    pub last_code: String,
    #[sqlx(skip)]
    #[serde(skip)]
    pub last_test_results: String,
    #[sqlx(skip)]
    #[serde(skip)]
    pub last_stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceResultWithModel {
    pub id: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseResult {
    pub input: String,
    pub expected: String,
    pub got: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExecutionDetail {
    pub id: String,
    pub result_id: String,
    pub code: String,
    pub test_results: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Submission {
    pub id: String,
    pub problem_id: String,
    pub ip_hash: String,
    pub solved: bool,
    pub time_ms: Option<i64>,
    pub attempts: i64,
    pub code: String,
    pub submitted_at: String,
}

#[derive(Debug, Deserialize)]
pub struct RunCodeRequest {
    pub code: String,
    pub problem_id: String,
    pub language: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RunCodeResponse {
    pub passed: bool,
    pub results: Vec<TestCaseResult>,
    pub stderr: String,
}

#[derive(Debug, Deserialize)]
pub struct SubmitCodeRequest {
    pub code: String,
    pub problem_id: String,
    pub time_ms: i64,
    pub attempts: i64,
    pub language: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SubmitCodeResponse {
    pub passed: bool,
    pub results: Vec<TestCaseResult>,
    pub submission_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExecutionDetailResponse {
    pub code: String,
    pub test_results: Vec<TestCaseResult>,
    pub stderr: String,
}
