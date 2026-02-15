//! Health check endpoints

use crate::state::HasServices;
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
pub async fn ready<S: HasServices>(State(state): State<S>) -> impl IntoResponse {
    let (db_ok, cache_ok) = state.check_ready().await;

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
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        assert_eq!(response.status, "healthy");
        assert_eq!(response.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("healthy"));
        assert!(json.contains("version"));
        assert!(json.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn test_health_response_deserialization() {
        let json = format!(r#"{{"status": "healthy", "version": "{}"}}"#, env!("CARGO_PKG_VERSION"));
        let response: HealthResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response.status, "healthy");
        assert_eq!(response.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_health_response_json_roundtrip() {
        let original = HealthResponse {
            status: "healthy".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        let json = serde_json::to_string(&original).unwrap();
        let parsed: HealthResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(original.status, parsed.status);
        assert_eq!(original.version, parsed.version);
    }

    #[tokio::test]
    async fn test_health_endpoint_returns_json() {
        let response = health().await;
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
