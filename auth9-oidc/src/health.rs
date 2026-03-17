use crate::server::AppState;
use axum::{extract::State, Json};
use serde::Serialize;
use sqlx::Row;
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
    pub identity_backend: String,
    pub database: &'static str,
}

pub async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let db_status = match sqlx::query("SELECT 1").fetch_one(&state.db_pool).await {
        Ok(row) if row.try_get::<i32, _>(0).is_ok() => "up",
        _ => "down",
    };

    let _ = state.identity_engine.health_probe().await;

    Json(HealthResponse {
        status: "healthy",
        service: "auth9-oidc",
        identity_backend: state.config.identity_backend.clone(),
        database: db_status,
    })
}
