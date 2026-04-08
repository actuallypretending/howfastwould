use axum::{
    extract::{ConnectInfo, Path, State},
    http::StatusCode,
    Json,
};
use sha2::{Sha256, Digest};
use sqlx::Row;
use std::net::SocketAddr;
use uuid::Uuid;
use chrono::Utc;
use crate::{
    models::{
        ExecutionDetailResponse, Problem, RunCodeRequest, RunCodeResponse,
        SubmitCodeRequest, SubmitCodeResponse, TestCase, TestCaseResult,
    },
    routes::AppState,
};

/// POST /run — execute user code against test cases, return results (no persistence).
pub async fn run_code(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(body): Json<RunCodeRequest>,
) -> Result<Json<RunCodeResponse>, StatusCode> {
    if let Err(_retry_after) = state.rate_limiter.check(addr.ip()) {
        tracing::warn!("rate limited {}", addr.ip());
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    let problem = sqlx::query_as!(
        Problem,
        "SELECT * FROM problems WHERE id = $1",
        body.problem_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let test_cases: Vec<TestCase> =
        serde_json::from_str(&problem.test_cases).unwrap_or_default();

    let (passed, results, stderr) = state
        .runner
        .verify_with_detail(&body.code, &test_cases)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RunCodeResponse {
        passed,
        results,
        stderr,
    }))
}

/// POST /submit — execute user code and persist if all tests pass.
pub async fn submit_code(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(body): Json<SubmitCodeRequest>,
) -> Result<Json<SubmitCodeResponse>, StatusCode> {
    if let Err(_retry_after) = state.rate_limiter.check(addr.ip()) {
        tracing::warn!("rate limited {}", addr.ip());
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    if body.time_ms <= 0 || body.time_ms > 3_600_000 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let problem = sqlx::query_as!(
        Problem,
        "SELECT * FROM problems WHERE id = $1",
        body.problem_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let test_cases: Vec<TestCase> =
        serde_json::from_str(&problem.test_cases).unwrap_or_default();

    let (passed, results, _stderr) = state
        .runner
        .verify_with_detail(&body.code, &test_cases)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut submission_id = None;

    if passed {
        let id = Uuid::new_v4().to_string();
        let ip_hash = format!("{:x}", Sha256::digest(addr.ip().to_string().as_bytes()));
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"INSERT INTO submissions (id, problem_id, ip_hash, solved, time_ms, attempts, code, submitted_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
        )
        .bind(&id)
        .bind(&body.problem_id)
        .bind(&ip_hash)
        .bind(true)
        .bind(body.time_ms)
        .bind(body.attempts)
        .bind(&body.code)
        .bind(&now)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        submission_id = Some(id);
    }

    Ok(Json(SubmitCodeResponse {
        passed,
        results,
        submission_id,
    }))
}

/// GET /results/:result_id/details — fetch execution details for an AI benchmark result.
pub async fn result_details(
    State(state): State<AppState>,
    Path(result_id): Path<String>,
) -> Result<Json<ExecutionDetailResponse>, StatusCode> {
    let row = sqlx::query(
        "SELECT code, test_results, stderr FROM execution_details WHERE result_id = $1",
    )
    .bind(&result_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let code: String = row.get("code");
    let test_results_json: String = row.get("test_results");
    let stderr: String = row.get("stderr");

    let test_results: Vec<TestCaseResult> =
        serde_json::from_str(&test_results_json).unwrap_or_default();

    Ok(Json(ExecutionDetailResponse {
        code,
        test_results,
        stderr,
    }))
}
