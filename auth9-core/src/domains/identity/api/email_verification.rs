//! Email verification API handlers.
//!
//! Public endpoints for sending and verifying email verification tokens.

use crate::error::{AppError, Result};
use crate::http_support::{write_audit_log_generic, MessageResponse};
use crate::state::{HasEmailVerification, HasServices, HasSystemSettings};
use axum::{extract::State, http::HeaderMap, Json};
use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct SendVerificationRequest {
    pub email: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyEmailRequest {
    pub token: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/hosted-login/send-verification",
    tag = "Identity",
    request_body = SendVerificationRequest,
    responses(
        (status = 200, description = "Verification email sent", body = MessageResponse),
    )
)]
/// Send an email verification link to the user's email address.
///
/// POST /api/v1/hosted-login/send-verification
pub async fn send_verification<S: HasServices + HasEmailVerification + HasSystemSettings>(
    State(state): State<S>,
    headers: HeaderMap,
    Json(input): Json<SendVerificationRequest>,
) -> Result<Json<MessageResponse>> {
    let email = input.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(AppError::BadRequest("Invalid email address.".to_string()));
    }

    // Look up user — always return success to prevent email enumeration
    let user = match state.user_service().get_by_email(&email).await {
        Ok(user) => user,
        Err(_) => {
            return Ok(Json(MessageResponse::new(
                "If an account exists with this email, a verification link has been sent.",
            )));
        }
    };

    // Create verification token
    let (_, link) = state
        .email_verification_service()
        .create_verification_token(&user.identity_subject)
        .await?;

    // Build and send the email
    use crate::email::templates::{EmailTemplate, TemplateEngine};
    use crate::models::email::{EmailAddress, EmailMessage};

    let mut engine = TemplateEngine::new();
    engine
        .set("user_name", user.display_name.as_deref().unwrap_or(&email))
        .set("verification_link", &link)
        .set("expires_in_hours", "24")
        .set("year", chrono::Utc::now().format("%Y").to_string())
        .set("app_name", "Auth9");

    let rendered = engine.render_template(EmailTemplate::EmailVerification);

    let message = EmailMessage::new(
        EmailAddress::with_name(&email, user.display_name.as_deref().unwrap_or("")),
        rendered.subject,
        rendered.html_body,
    )
    .with_text_body(rendered.text_body);

    if let Err(e) = state.email_service().send(&message, None).await {
        tracing::error!(error = %e, "Failed to send verification email to {}", email);
    }

    let _ = write_audit_log_generic(
        &state,
        &headers,
        "email_verification.sent",
        "user",
        Some(*user.id),
        None,
        None,
    )
    .await;

    Ok(Json(MessageResponse::new(
        "If an account exists with this email, a verification link has been sent.",
    )))
}

#[utoipa::path(
    post,
    path = "/api/v1/hosted-login/verify-email",
    tag = "Identity",
    request_body = VerifyEmailRequest,
    responses(
        (status = 200, description = "Email verified successfully", body = MessageResponse),
        (status = 400, description = "Invalid or expired token"),
    )
)]
/// Verify an email address using a token from the verification link.
///
/// POST /api/v1/hosted-login/verify-email
pub async fn verify_email<S: HasServices + HasEmailVerification>(
    State(state): State<S>,
    headers: HeaderMap,
    Json(input): Json<VerifyEmailRequest>,
) -> Result<Json<MessageResponse>> {
    if input.token.is_empty() {
        return Err(AppError::BadRequest(
            "Verification token is required.".to_string(),
        ));
    }

    let user_id = state
        .email_verification_service()
        .verify_email(&input.token)
        .await?;

    let _ = write_audit_log_generic(
        &state,
        &headers,
        "email_verification.completed",
        "user",
        uuid::Uuid::parse_str(&user_id).ok(),
        None,
        None,
    )
    .await;

    Ok(Json(MessageResponse::new("Email verified successfully.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_verification_request_deserialization() {
        let json = r#"{"email": "test@example.com"}"#;
        let input: SendVerificationRequest = serde_json::from_str(json).unwrap();
        assert_eq!(input.email, "test@example.com");
    }

    #[test]
    fn verify_email_request_deserialization() {
        let json = r#"{"token": "abc123token"}"#;
        let input: VerifyEmailRequest = serde_json::from_str(json).unwrap();
        assert_eq!(input.token, "abc123token");
    }
}
