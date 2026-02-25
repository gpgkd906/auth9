//! Unified error handling for Auth9 Core

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;
use utoipa::ToSchema;

/// Application-wide result type
pub type Result<T> = std::result::Result<T, AppError>;

/// Application error types
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("Keycloak error: {0}")]
    Keycloak(String),

    #[error("Action execution failed: {0}")]
    ActionExecutionFailed(String),

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

/// Error response body
#[derive(Serialize, ToSchema)]
pub(crate) struct ErrorResponse {
    error: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad_request", msg.clone()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "unauthorized", msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "forbidden", msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "conflict", msg.clone()),
            AppError::Validation(msg) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "validation", msg.clone())
            }
            AppError::Database(ref e) => {
                // Map duplicate entry errors (MySQL 1062 / SQLSTATE 23000) to 409 Conflict
                if let sqlx::Error::Database(ref db_err) = e {
                    let is_duplicate = db_err.code().as_deref() == Some("23000")
                        || db_err.code().as_deref() == Some("1062")
                        || db_err.message().contains("Duplicate entry");
                    if is_duplicate {
                        return AppError::Conflict(
                            "Resource already exists".to_string(),
                        )
                        .into_response();
                    }
                }
                tracing::error!("Database error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "database_error",
                    "A database error occurred".to_string(),
                )
            }
            AppError::Redis(e) => {
                tracing::error!("Redis error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "cache_error",
                    "A cache error occurred".to_string(),
                )
            }
            AppError::Jwt(e) => {
                tracing::error!("JWT error: {:?}", e);
                (
                    StatusCode::UNAUTHORIZED,
                    "jwt_error",
                    "Invalid or expired token".to_string(),
                )
            }
            AppError::Keycloak(msg) => {
                tracing::error!("Keycloak error: {}", msg);
                (
                    StatusCode::BAD_GATEWAY,
                    "keycloak_error",
                    "Authentication service error".to_string(),
                )
            }
            AppError::ActionExecutionFailed(msg) => {
                tracing::error!("Action execution failed: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "action_execution_failed",
                    msg.clone(),
                )
            }
            AppError::Internal(e) => {
                tracing::error!("Internal error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "An internal error occurred".to_string(),
                )
            }
        };

        let body = Json(ErrorResponse {
            error: error_type.to_string(),
            message,
            details: None,
        });

        (status, body).into_response()
    }
}

// Conversion from validation errors
impl From<validator::ValidationErrors> for AppError {
    fn from(errors: validator::ValidationErrors) -> Self {
        let messages: Vec<String> = errors
            .field_errors()
            .iter()
            .map(|(field, errs)| {
                let field_name = format_field_name(field);
                let err_msg = errs
                    .first()
                    .map(|e| format_validation_error(e, &field_name))
                    .unwrap_or_else(|| format!("{} is invalid", field_name));
                err_msg
            })
            .collect();
        AppError::Validation(messages.join("; "))
    }
}

fn format_field_name(field: &str) -> String {
    match field {
        "current_password" => "Current password".to_string(),
        "new_password" => "New password".to_string(),
        "confirm_password" => "Confirm password".to_string(),
        "email" => "Email".to_string(),
        "name" => "Name".to_string(),
        "display_name" => "Display name".to_string(),
        s => s
            .replace('_', " ")
            .split_whitespace()
            .map(|w| {
                let mut chars = w.chars();
                match chars.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn format_validation_error(err: &validator::ValidationError, field_name: &str) -> String {
    match err.code.as_ref() {
        "length" => {
            let min = err.params.get("min").and_then(|v| v.as_u64()).unwrap_or(0);
            let max = err.params.get("max").and_then(|v| v.as_u64());
            if let Some(max_val) = max {
                format!(
                    "{} must be between {} and {} characters",
                    field_name, min, max_val
                )
            } else {
                format!("{} must be at least {} characters", field_name, min)
            }
        }
        "email" => format!("{} must be a valid email address", field_name),
        "url" => format!("{} must be a valid URL", field_name),
        "required" => format!("{} is required", field_name),
        "must_match" => format!("{} does not match", field_name),
        code => format!("{} is invalid: {}", field_name, code),
    }
}

// Conversion from axum JSON rejection - hide internal parser details
impl From<axum::extract::rejection::JsonRejection> for AppError {
    fn from(rejection: axum::extract::rejection::JsonRejection) -> Self {
        tracing::debug!("JSON rejection: {:?}", rejection);
        AppError::BadRequest("Invalid request body".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AppError::NotFound("User not found".to_string());
        assert_eq!(err.to_string(), "Not found: User not found");
    }

    #[test]
    fn test_error_conversion() {
        let err: AppError = anyhow::anyhow!("Something went wrong").into();
        assert!(matches!(err, AppError::Internal(_)));
    }

    #[test]
    fn test_bad_request_display() {
        let err = AppError::BadRequest("Invalid input".to_string());
        assert_eq!(err.to_string(), "Bad request: Invalid input");
    }

    #[test]
    fn test_unauthorized_display() {
        let err = AppError::Unauthorized("Token expired".to_string());
        assert_eq!(err.to_string(), "Unauthorized: Token expired");
    }

    #[test]
    fn test_forbidden_display() {
        let err = AppError::Forbidden("Access denied".to_string());
        assert_eq!(err.to_string(), "Forbidden: Access denied");
    }

    #[test]
    fn test_conflict_display() {
        let err = AppError::Conflict("Resource already exists".to_string());
        assert_eq!(err.to_string(), "Conflict: Resource already exists");
    }

    #[test]
    fn test_validation_display() {
        let err = AppError::Validation("Email is required".to_string());
        assert_eq!(err.to_string(), "Validation error: Email is required");
    }

    #[test]
    fn test_keycloak_display() {
        let err = AppError::Keycloak("Connection refused".to_string());
        assert_eq!(err.to_string(), "Keycloak error: Connection refused");
    }

    #[test]
    fn test_error_response_serialization() {
        let response = ErrorResponse {
            error: "not_found".to_string(),
            message: "User not found".to_string(),
            details: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"error\":\"not_found\""));
        assert!(json.contains("\"message\":\"User not found\""));
        assert!(!json.contains("\"details\"")); // Skip serialization when None
    }

    #[test]
    fn test_error_response_with_details() {
        let response = ErrorResponse {
            error: "validation".to_string(),
            message: "Validation failed".to_string(),
            details: Some(serde_json::json!({"field": "email", "reason": "invalid format"})),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"details\""));
        assert!(json.contains("\"field\":\"email\""));
    }

    #[test]
    fn test_app_error_debug() {
        let err = AppError::NotFound("Test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("NotFound"));
        assert!(debug_str.contains("Test"));
    }

    #[test]
    fn test_not_found_error_variant() {
        let err = AppError::NotFound("Resource".to_string());
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[test]
    fn test_bad_request_error_variant() {
        let err = AppError::BadRequest("Invalid".to_string());
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[test]
    fn test_unauthorized_error_variant() {
        let err = AppError::Unauthorized("No token".to_string());
        assert!(matches!(err, AppError::Unauthorized(_)));
    }

    #[test]
    fn test_forbidden_error_variant() {
        let err = AppError::Forbidden("Not allowed".to_string());
        assert!(matches!(err, AppError::Forbidden(_)));
    }

    #[test]
    fn test_conflict_error_variant() {
        let err = AppError::Conflict("Duplicate".to_string());
        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[test]
    fn test_validation_error_variant() {
        let err = AppError::Validation("Invalid field".to_string());
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn test_keycloak_error_variant() {
        let err = AppError::Keycloak("Auth failed".to_string());
        assert!(matches!(err, AppError::Keycloak(_)));
    }

    #[test]
    fn test_internal_error_variant() {
        let err = AppError::Internal(anyhow::anyhow!("Internal issue"));
        assert!(matches!(err, AppError::Internal(_)));
    }

    #[test]
    fn test_jwt_error_conversion() {
        // Create an invalid JWT error through verification
        let invalid_result: std::result::Result<jsonwebtoken::TokenData<serde_json::Value>, _> =
            jsonwebtoken::decode(
                "invalid",
                &jsonwebtoken::DecodingKey::from_secret(b"secret"),
                &jsonwebtoken::Validation::default(),
            );

        if let Err(jwt_err) = invalid_result {
            let app_err: AppError = jwt_err.into();
            assert!(matches!(app_err, AppError::Jwt(_)));
        }
    }

    #[test]
    fn test_error_messages_are_descriptive() {
        let errors = vec![
            AppError::NotFound("User with ID 123".to_string()),
            AppError::BadRequest("Missing required field: email".to_string()),
            AppError::Unauthorized("Session expired".to_string()),
            AppError::Forbidden("Admin access required".to_string()),
            AppError::Conflict("Email already registered".to_string()),
            AppError::Validation("Password too short".to_string()),
            AppError::Keycloak("Failed to authenticate".to_string()),
        ];

        for err in errors {
            let msg = err.to_string();
            assert!(!msg.is_empty());
            assert!(msg.len() > 10); // Should have meaningful message
        }
    }
}
