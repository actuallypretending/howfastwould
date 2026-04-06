use axum::{extract::State, Json};
use crate::{models::Model, routes::AppState};

pub async fn list(State(state): State<AppState>) -> Json<Vec<Model>> {
    let models = sqlx::query_as!(Model,
        r#"SELECT id as "id!", provider as "provider!", name as "name!", display_name as "display_name!",
           api_key_env as "api_key_env!", is_active as "is_active!", is_new as "is_new!",
           is_human as "is_human!", human_times, added_at as "added_at!"
           FROM models WHERE is_active = 1 ORDER BY is_human, provider, name"#
    ).fetch_all(&state.pool).await.unwrap_or_default();
    Json(models)
}
