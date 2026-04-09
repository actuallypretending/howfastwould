use axum::{
    extract::{ConnectInfo, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
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

const MAX_CODE_SIZE: usize = 50_000;

fn validated_language(lang: Option<&str>) -> Result<&str, Response> {
    match lang.unwrap_or("python3") {
        l @ ("python3" | "javascript") => Ok(l),
        _ => Err(StatusCode::BAD_REQUEST.into_response()),
    }
}

fn rate_limit_response(retry_after: u64) -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        Json(serde_json::json!({ "retry_after": retry_after })),
    )
        .into_response()
}

/// POST /run — execute user code against test cases, return results (no persistence).
pub async fn run_code(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(body): Json<RunCodeRequest>,
) -> Response {
    if let Err(retry_after) = state.rate_limiter.check(addr.ip()) {
        tracing::warn!("rate limited {}", addr.ip());
        return rate_limit_response(retry_after);
    }

    if body.code.len() > MAX_CODE_SIZE {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let language = match validated_language(body.language.as_deref()) {
        Ok(l) => l,
        Err(r) => return r,
    };

    let problem = match sqlx::query_as!(
        Problem,
        "SELECT * FROM problems WHERE id = $1",
        body.problem_id
    )
    .fetch_optional(&state.pool)
    .await
    {
        Ok(Some(p)) => p,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let test_cases: Vec<TestCase> =
        serde_json::from_str(&problem.test_cases).unwrap_or_default();

    let (passed, results, stderr) = match state
        .runner
        .verify_with_detail(&body.code, &test_cases, language)
        .await
    {
        Ok(r) => r,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    Json(RunCodeResponse {
        passed,
        results,
        stderr,
    })
    .into_response()
}

/// POST /submit — execute user code and persist if all tests pass.
pub async fn submit_code(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(body): Json<SubmitCodeRequest>,
) -> Response {
    if let Err(retry_after) = state.rate_limiter.check(addr.ip()) {
        tracing::warn!("rate limited {}", addr.ip());
        return rate_limit_response(retry_after);
    }

    if body.code.len() > MAX_CODE_SIZE {
        return StatusCode::BAD_REQUEST.into_response();
    }

    if body.time_ms <= 0 || body.time_ms > 3_600_000 {
        return StatusCode::BAD_REQUEST.into_response();
    }

    if body.attempts < 1 || body.attempts > 100 {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let language = match validated_language(body.language.as_deref()) {
        Ok(l) => l,
        Err(r) => return r,
    };

    let problem = match sqlx::query_as!(
        Problem,
        "SELECT * FROM problems WHERE id = $1",
        body.problem_id
    )
    .fetch_optional(&state.pool)
    .await
    {
        Ok(Some(p)) => p,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let test_cases: Vec<TestCase> =
        serde_json::from_str(&problem.test_cases).unwrap_or_default();

    let (passed, results, _stderr) = match state
        .runner
        .verify_with_detail(&body.code, &test_cases, language)
        .await
    {
        Ok(r) => r,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let mut submission_id = None;

    if passed {
        let id = Uuid::new_v4().to_string();
        let ip_hash = format!("{:x}", Sha256::digest(addr.ip().to_string().as_bytes()));
        let now = Utc::now().to_rfc3339();

        if sqlx::query(
            r#"INSERT INTO submissions (id, problem_id, ip_hash, solved, time_ms, attempts, code, language, submitted_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
        )
        .bind(&id)
        .bind(&body.problem_id)
        .bind(&ip_hash)
        .bind(true)
        .bind(body.time_ms)
        .bind(body.attempts)
        .bind(&body.code)
        .bind(language)
        .bind(&now)
        .execute(&state.pool)
        .await
        .is_err()
        {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }

        submission_id = Some(id);
    }

    Json(SubmitCodeResponse {
        passed,
        results,
        submission_id,
    })
    .into_response()
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
