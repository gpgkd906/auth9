//! Email OTP (passwordless) authentication handlers

use crate::domains::identity::service::otp::{OtpChannelType, OtpManager, OtpRateLimitConfig};
use crate::email::{EmailTemplate, TemplateEngine};
use crate::error::{AppError, Result};
use crate::models::email::{EmailAddress, EmailMessage};
use crate::state::{HasBranding, HasCache, HasServices, HasSessionManagement, HasSystemSettings};
use axum::{extract::State, http::HeaderMap, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

const OTP_TTL_SECS: u64 = 600; // 10 minutes
const OTP_TTL_MINUTES: u32 = 10;

// ==================== Types ====================

#[derive(Debug, Deserialize, ToSchema)]
pub struct SendEmailOtpRequest {
    pub email: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SendEmailOtpResponse {
    pub message: String,
    pub expires_in_seconds: u64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyEmailOtpRequest {
    pub email: String,
    pub code: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EmailOtpTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

// ==================== Helpers ====================

async fn check_email_otp_enabled<S: HasBranding>(state: &S) -> Result<()> {
    let branding = state.branding_service().get_branding().await?;
    if !branding.email_otp_enabled {
        return Err(AppError::NotFound("Not found".to_string()));
    }
    Ok(())
}

fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(String::from)
        })
}

// ==================== Handlers ====================

#[utoipa::path(
    post,
    path = "/api/v1/auth/email-otp/send",
    tag = "Identity",
    responses(
        (status = 200, description = "OTP sent (or silently ignored if email not registered)")
    )
)]
/// Send Email OTP verification code
///
/// POST /api/v1/auth/email-otp/send
pub async fn send_email_otp<S: HasCache + HasSystemSettings + HasBranding>(
    State(state): State<S>,
    Json(input): Json<SendEmailOtpRequest>,
) -> Result<Json<SendEmailOtpResponse>> {
    check_email_otp_enabled(&state).await?;

    let email = input.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(AppError::BadRequest("Invalid email address.".to_string()));
    }

    let rate_limit = OtpRateLimitConfig::email_defaults();
    let manager = OtpManager::new(Arc::new(state.cache().clone()));
    let code = OtpManager::<S::Cache>::generate_code();

    // Store OTP (rate limiting is handled inside OtpManager::store)
    manager
        .store(
            OtpChannelType::Email,
            &email,
            &code,
            OTP_TTL_SECS,
            &rate_limit,
        )
        .await?;

    // Send email using the EmailMfa template (same as EmailOtpChannel)
    let mut engine = TemplateEngine::new();
    engine
        .set("user_name", &email)
        .set("verification_code", &code)
        .set("expires_in_minutes", OTP_TTL_MINUTES.to_string())
        .set("app_name", "Auth9")
        .set("year", chrono::Utc::now().format("%Y").to_string());

    let rendered = engine.render_template(EmailTemplate::EmailMfa);

    let message = EmailMessage::new(
        EmailAddress::new(&email),
        &rendered.subject,
        &rendered.html_body,
    )
    .with_text_body(&rendered.text_body);

    // Send email — errors are silenced to prevent enumeration
    let _ = state.email_service().send(&message, None).await;

    Ok(Json(SendEmailOtpResponse {
        message: "If this email is registered, a verification code has been sent.".to_string(),
        expires_in_seconds: OTP_TTL_SECS,
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/email-otp/verify",
    tag = "Identity",
    responses(
        (status = 200, description = "Authentication token")
    )
)]
/// Verify Email OTP and issue identity token
///
/// POST /api/v1/auth/email-otp/verify
pub async fn verify_email_otp<S: HasCache + HasServices + HasSessionManagement + HasBranding>(
    State(state): State<S>,
    headers: HeaderMap,
    Json(input): Json<VerifyEmailOtpRequest>,
) -> Result<Json<EmailOtpTokenResponse>> {
    check_email_otp_enabled(&state).await?;

    let email = input.email.trim().to_lowercase();
    let code = input.code.trim().to_string();

    if email.is_empty() || code.is_empty() {
        return Err(AppError::BadRequest(
            "Invalid or expired verification code.".to_string(),
        ));
    }

    let rate_limit = OtpRateLimitConfig::email_defaults();
    let manager = OtpManager::new(Arc::new(state.cache().clone()));

    // Verify and consume the OTP
    manager
        .verify_and_consume(OtpChannelType::Email, &email, &code, &rate_limit)
        .await?;

    // OTP is valid — look up user (return generic error if not found to prevent enumeration)
    let user = match state.user_service().get_by_email(&email).await {
        Ok(user) => user,
        Err(_) => {
            return Err(AppError::Unauthorized("Authentication failed.".to_string()));
        }
    };

    // Create session
    let ip_address = extract_client_ip(&headers);
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let session = state
        .session_service()
        .create_session(user.id, None, ip_address, user_agent)
        .await?;

    // Issue identity token
    let jwt_manager = HasServices::jwt_manager(&state);
    let identity_token = jwt_manager.create_identity_token_with_session(
        *user.id,
        &user.email,
        user.display_name.as_deref(),
        Some(*session.id),
    )?;

    Ok(Json(EmailOtpTokenResponse {
        access_token: identity_token,
        token_type: "Bearer".to_string(),
        expires_in: jwt_manager.access_token_ttl(),
    }))
}
