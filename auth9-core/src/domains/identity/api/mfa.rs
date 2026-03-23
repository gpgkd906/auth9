//! MFA API handlers
//!
//! Endpoints for TOTP enrollment/verification, recovery code management,
//! MFA status, and MFA challenge verification during login.

use crate::cache::CacheOperations;
use crate::domains::identity::service::totp::TotpEnrollmentResponse;
use crate::error::{AppError, Result};
use crate::http_support::SuccessResponse;
use crate::models::common::StringUuid;
use crate::state::{HasCache, HasMfa, HasServices, HasSessionManagement, HasWebAuthn};
use axum::{extract::State, Json};
use axum_extra::headers::{authorization::Bearer, Authorization};
use axum_extra::TypedHeader;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Parse JWT sub claim (user UUID string) into StringUuid for user lookups.
fn parse_user_id(user_id: &str) -> Result<StringUuid> {
    let uuid = uuid::Uuid::parse_str(user_id)
        .map_err(|_| AppError::BadRequest("Invalid user ID in token".to_string()))?;
    Ok(StringUuid::from(uuid))
}

// ==================== Types ====================

#[derive(Debug, Serialize, ToSchema)]
pub struct MfaStatusResponse {
    pub totp_enabled: bool,
    pub webauthn_enabled: bool,
    pub recovery_codes_remaining: usize,
    pub email_otp_enabled: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TotpEnrollVerifyRequest {
    pub setup_token: String,
    pub code: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MfaChallengeVerifyRequest {
    pub mfa_session_token: String,
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MfaChallengeResponse {
    pub mfa_required: bool,
    pub mfa_session_token: String,
    pub mfa_methods: Vec<String>,
    pub expires_in: u64,
}

/// MFA session data stored in Redis
#[derive(Debug, Serialize, Deserialize)]
pub struct MfaSessionData {
    pub user_id: String,
    pub email: String,
    pub display_name: Option<String>,
    pub identity_subject: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HostedLoginTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

pub const MFA_SESSION_TTL_SECS: u64 = 300;

// ==================== Protected Endpoints (authenticated user) ====================

/// GET /api/v1/mfa/status
pub async fn mfa_status<S: HasMfa + HasWebAuthn + HasServices>(
    State(state): State<S>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<SuccessResponse<MfaStatusResponse>>> {
    let claims = HasServices::jwt_manager(&state).verify_identity_token(bearer.token())?;
    let user_id = &claims.sub;

    let totp_enabled = state.totp_service().has_totp(user_id).await?;

    let webauthn_creds = state
        .webauthn_service()
        .list_credentials(user_id, None)
        .await?;
    let webauthn_enabled = !webauthn_creds.is_empty();

    let recovery_codes_remaining = state
        .recovery_code_service()
        .remaining_count(user_id)
        .await?;

    let email_otp_enabled = match parse_user_id(user_id) {
        Ok(uid) => state
            .user_service()
            .get(uid)
            .await
            .map(|u| u.email_otp_enabled)
            .unwrap_or(false),
        Err(_) => false,
    };

    Ok(Json(SuccessResponse::new(MfaStatusResponse {
        totp_enabled,
        webauthn_enabled,
        recovery_codes_remaining,
        email_otp_enabled,
    })))
}

/// POST /api/v1/mfa/totp/enroll
pub async fn totp_enroll_start<S: HasMfa + HasServices>(
    State(state): State<S>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<SuccessResponse<TotpEnrollmentResponse>>> {
    let claims = HasServices::jwt_manager(&state).verify_identity_token(bearer.token())?;
    let user_id = &claims.sub;
    let email = &claims.email;

    let enrollment = state
        .totp_service()
        .start_enrollment(user_id, email)
        .await?;
    Ok(Json(SuccessResponse::new(enrollment)))
}

/// POST /api/v1/mfa/totp/enroll/verify
pub async fn totp_enroll_verify<S: HasMfa + HasServices>(
    State(state): State<S>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
    Json(input): Json<TotpEnrollVerifyRequest>,
) -> Result<Json<SuccessResponse<MfaStatusResponse>>> {
    let claims = HasServices::jwt_manager(&state).verify_identity_token(bearer.token())?;
    let user_id = &claims.sub;

    state
        .totp_service()
        .complete_enrollment(user_id, &input.setup_token, &input.code)
        .await?;

    // Update mfa_enabled on user
    if let Ok(uid) = parse_user_id(user_id) {
        if let Ok(user) = state.user_service().get(uid).await {
            let _ = state.user_service().set_mfa_enabled(user.id, true).await;
        }
    }

    let email_otp_enabled = match parse_user_id(user_id) {
        Ok(uid) => state
            .user_service()
            .get(uid)
            .await
            .map(|u| u.email_otp_enabled)
            .unwrap_or(false),
        Err(_) => false,
    };

    Ok(Json(SuccessResponse::new(MfaStatusResponse {
        totp_enabled: true,
        webauthn_enabled: false,
        recovery_codes_remaining: 0,
        email_otp_enabled,
    })))
}

/// DELETE /api/v1/mfa/totp
pub async fn totp_remove<S: HasMfa + HasServices + HasWebAuthn>(
    State(state): State<S>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<crate::http_support::MessageResponse>> {
    let claims = HasServices::jwt_manager(&state).verify_identity_token(bearer.token())?;
    let user_id = &claims.sub;

    state.totp_service().remove_totp(user_id).await?;

    // Check if any MFA methods remain
    let webauthn_creds = state
        .webauthn_service()
        .list_credentials(user_id, None)
        .await?;
    if webauthn_creds.is_empty() {
        if let Ok(uid) = parse_user_id(user_id) {
            if let Ok(user) = state.user_service().get(uid).await {
                let _ = state.user_service().set_mfa_enabled(user.id, false).await;
            }
        }
    }

    Ok(Json(crate::http_support::MessageResponse::new(
        "TOTP removed successfully.",
    )))
}

/// POST /api/v1/mfa/recovery-codes/generate
pub async fn recovery_codes_generate<S: HasMfa + HasServices>(
    State(state): State<S>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<SuccessResponse<Vec<String>>>> {
    let claims = HasServices::jwt_manager(&state).verify_identity_token(bearer.token())?;
    let user_id = &claims.sub;

    let codes = state
        .recovery_code_service()
        .generate_codes(user_id)
        .await?;
    Ok(Json(SuccessResponse::new(codes)))
}

/// GET /api/v1/mfa/recovery-codes/remaining
pub async fn recovery_codes_remaining<S: HasMfa + HasServices>(
    State(state): State<S>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<SuccessResponse<usize>>> {
    let claims = HasServices::jwt_manager(&state).verify_identity_token(bearer.token())?;
    let user_id = &claims.sub;

    let count = state
        .recovery_code_service()
        .remaining_count(user_id)
        .await?;
    Ok(Json(SuccessResponse::new(count)))
}

/// POST /api/v1/mfa/email-otp/enable
pub async fn email_otp_enable<S: HasMfa + HasWebAuthn + HasServices>(
    State(state): State<S>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<SuccessResponse<MfaStatusResponse>>> {
    let claims = HasServices::jwt_manager(&state).verify_identity_token(bearer.token())?;
    let user_id = &claims.sub;

    let uid = parse_user_id(user_id)?;
    let user = state.user_service().get(uid).await?;
    state.user_service().set_email_otp_enabled(user.id, true).await?;

    let totp_enabled = state.totp_service().has_totp(user_id).await?;
    let webauthn_creds = state.webauthn_service().list_credentials(user_id, None).await?;
    let recovery_codes_remaining = state.recovery_code_service().remaining_count(user_id).await?;

    Ok(Json(SuccessResponse::new(MfaStatusResponse {
        totp_enabled,
        webauthn_enabled: !webauthn_creds.is_empty(),
        recovery_codes_remaining,
        email_otp_enabled: true,
    })))
}

/// POST /api/v1/mfa/email-otp/disable
pub async fn email_otp_disable<S: HasMfa + HasWebAuthn + HasServices>(
    State(state): State<S>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<SuccessResponse<MfaStatusResponse>>> {
    let claims = HasServices::jwt_manager(&state).verify_identity_token(bearer.token())?;
    let user_id = &claims.sub;

    let uid = parse_user_id(user_id)?;
    let user = state.user_service().get(uid).await?;
    state.user_service().set_email_otp_enabled(user.id, false).await?;

    let totp_enabled = state.totp_service().has_totp(user_id).await?;
    let webauthn_creds = state.webauthn_service().list_credentials(user_id, None).await?;
    let recovery_codes_remaining = state.recovery_code_service().remaining_count(user_id).await?;

    Ok(Json(SuccessResponse::new(MfaStatusResponse {
        totp_enabled,
        webauthn_enabled: !webauthn_creds.is_empty(),
        recovery_codes_remaining,
        email_otp_enabled: false,
    })))
}

// ==================== MFA Challenge Endpoints (public, during login) ====================

/// POST /api/v1/mfa/challenge/totp
pub async fn challenge_totp<S: HasMfa + HasCache + HasServices + HasSessionManagement>(
    State(state): State<S>,
    Json(input): Json<MfaChallengeVerifyRequest>,
) -> Result<Json<HostedLoginTokenResponse>> {
    let session_data = consume_mfa_session(&state, &input.mfa_session_token).await?;

    let valid = state
        .totp_service()
        .verify_code(&session_data.user_id, &input.code)
        .await?;

    if !valid {
        return Err(AppError::Unauthorized("Invalid TOTP code.".to_string()));
    }

    issue_token_after_mfa(&state, &session_data).await
}

/// POST /api/v1/mfa/challenge/recovery-code
pub async fn challenge_recovery_code<S: HasMfa + HasCache + HasServices + HasSessionManagement>(
    State(state): State<S>,
    Json(input): Json<MfaChallengeVerifyRequest>,
) -> Result<Json<HostedLoginTokenResponse>> {
    let session_data = consume_mfa_session(&state, &input.mfa_session_token).await?;

    let valid = state
        .recovery_code_service()
        .verify_and_consume(&session_data.user_id, &input.code)
        .await?;

    if !valid {
        return Err(AppError::Unauthorized(
            "Invalid or already used recovery code.".to_string(),
        ));
    }

    issue_token_after_mfa(&state, &session_data).await
}

// ==================== Helpers ====================

async fn consume_mfa_session<S: HasCache>(state: &S, token: &str) -> Result<MfaSessionData> {
    let session_json = state
        .cache()
        .consume_mfa_session(token)
        .await?
        .ok_or_else(|| {
            AppError::Unauthorized(
                "MFA session expired or invalid. Please log in again.".to_string(),
            )
        })?;

    serde_json::from_str(&session_json)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse MFA session data: {}", e)))
}

async fn issue_token_after_mfa<S: HasServices + HasSessionManagement>(
    state: &S,
    session_data: &MfaSessionData,
) -> Result<Json<HostedLoginTokenResponse>> {
    let user_id = uuid::Uuid::parse_str(&session_data.user_id)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user_id in MFA session")))?;

    let session = state
        .session_service()
        .create_session(
            user_id.into(),
            None,
            session_data.ip_address.clone(),
            session_data.user_agent.clone(),
        )
        .await?;

    let jwt_manager = HasServices::jwt_manager(state);
    let identity_token = jwt_manager.create_identity_token_with_session(
        user_id,
        &session_data.email,
        session_data.display_name.as_deref(),
        Some(*session.id),
    )?;

    Ok(Json(HostedLoginTokenResponse {
        access_token: identity_token,
        token_type: "Bearer".to_string(),
        expires_in: jwt_manager.access_token_ttl(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mfa_session_data_serde() {
        let data = MfaSessionData {
            user_id: "user-1".to_string(),
            email: "test@example.com".to_string(),
            display_name: Some("Test User".to_string()),
            identity_subject: "sub-1".to_string(),
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("Mozilla/5.0".to_string()),
        };
        let json = serde_json::to_string(&data).unwrap();
        let parsed: MfaSessionData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.user_id, "user-1");
        assert_eq!(parsed.email, "test@example.com");
    }

    #[test]
    fn test_mfa_challenge_response_serde() {
        let response = MfaChallengeResponse {
            mfa_required: true,
            mfa_session_token: "token-123".to_string(),
            mfa_methods: vec!["totp".to_string()],
            expires_in: 300,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("mfa_required"));
        assert!(json.contains("mfa_session_token"));
    }
}
