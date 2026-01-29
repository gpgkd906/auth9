//! Health check endpoints

use crate::server::AppState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Health check endpoint
pub async fn health() -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Readiness check endpoint
pub async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    let db_ok = sqlx::query("SELECT 1")
        .execute(&state.db_pool)
        .await
        .is_ok();

    let cache_ok = state.cache_manager.ping().await.is_ok();

    if db_ok && cache_ok {
        (StatusCode::OK, "ready")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not_ready")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_structure() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            version: "0.1.0".to_string(),
        };
        
        assert_eq!(response.status, "healthy");
        assert_eq!(response.version, "0.1.0");
    }

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };
        
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("healthy"));
        assert!(json.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn test_health_response_deserialization() {
        let json = r#"{"status": "healthy", "version": "0.1.0"}"#;
        let response: HealthResponse = serde_json::from_str(json).unwrap();
        
        assert_eq!(response.status, "healthy");
        assert_eq!(response.version, "0.1.0");
    }
}
