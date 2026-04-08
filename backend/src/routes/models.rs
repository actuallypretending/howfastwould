use axum::{extract::State, Json};
use crate::{models::{Model, LeaderboardEntry}, routes::AppState};

pub async fn list(State(state): State<AppState>) -> Json<Vec<Model>> {
    let models = sqlx::query_as!(Model,
        "SELECT * FROM models WHERE is_active = true ORDER BY is_human, provider, name"
    ).fetch_all(&state.pool).await.unwrap_or_default();
    Json(models)
}

pub async fn leaderboard(State(state): State<AppState>) -> Json<Vec<LeaderboardEntry>> {
    let entries: Vec<LeaderboardEntry> = sqlx::query_as::<_, LeaderboardEntry>(
        r#"WITH model_stats AS (
            SELECT
                r.model_id,
                m.display_name,
                m.provider,
                COUNT(*)::bigint as total,
                (COUNT(*) FILTER (WHERE r.solved = true))::bigint as solved,
                (AVG(r.time_ms) FILTER (WHERE r.solved = true))::bigint as avg_time_ms,
                (PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY r.time_ms)
                    FILTER (WHERE r.solved = true))::bigint as median_time_ms
            FROM results r
            JOIN models m ON r.model_id = m.id
            WHERE m.is_human = false AND m.is_active = true
            GROUP BY r.model_id, m.display_name, m.provider
        ),
        wins AS (
            SELECT DISTINCT ON (r.problem_id) r.model_id
            FROM results r
            JOIN models m ON r.model_id = m.id
            WHERE r.solved = true AND m.is_human = false AND m.is_active = true
            ORDER BY r.problem_id, r.time_ms ASC, r.model_id ASC
        ),
        win_counts AS (
            SELECT model_id, COUNT(*)::bigint as win_count
            FROM wins
            GROUP BY model_id
        )
        SELECT
            s.model_id,
            s.display_name,
            s.provider,
            s.total,
            s.solved,
            s.avg_time_ms,
            s.median_time_ms,
            COALESCE(w.win_count, 0)::bigint as win_count
        FROM model_stats s
        LEFT JOIN win_counts w ON s.model_id = w.model_id
        ORDER BY w.win_count DESC NULLS LAST, s.solved DESC, s.avg_time_ms ASC NULLS LAST"#
    ).fetch_all(&state.pool).await.unwrap_or_default();

    Json(entries)
}
