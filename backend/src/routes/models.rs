use axum::{extract::State, Json};
use crate::{models::Model, routes::AppState};

pub async fn list(State(state): State<AppState>) -> Json<Vec<Model>> {
    let models = sqlx::query_as!(Model,
        "SELECT * FROM models WHERE is_active = true ORDER BY is_human, provider, name"
    ).fetch_all(&state.pool).await.unwrap_or_default();
    Json(models)
}
