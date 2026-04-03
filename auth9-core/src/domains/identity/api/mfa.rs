//! MFA API handlers
//!
//! Endpoints for TOTP enrollment/verification, recovery code management,
//! MFA status, MFA challenge verification during login, trusted device
//! management, and adaptive MFA policy configuration.

use crate::cache::CacheOperations;
use crate::domains::identity::service::adaptive_mfa::{AdaptiveMfaMode, AdaptiveMfaPolicy};
use crate::domains::identity::service::totp::TotpEnrollmentResponse;
use crate::domains::identity::service::trusted_device::TrustedDevice;
use crate::error::{AppError, Result};
use crate::http_support::{MessageResponse, SuccessResponse};
use crate::middleware::auth::AuthUser;
use crate::models::common::StringUuid;
use crate::repository::adaptive_mfa_policy::{AdaptiveMfaPolicyRepository, AdaptiveMfaPolicyRow};
use crate::domains::identity::service::required_actions::PendingActionResponse;
use crate::state::{
    HasAdaptiveMfa, HasCache, HasMfa, HasRequiredActions, HasServices, HasSessionManagement,
    HasTrustedDevices, HasWebAuthn,
};
use axum::extract::Path;
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
pub struct TotpEnrollStartRequest {
    pub current_password: String,
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
    #[serde(default)]
    pub trust_device: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MfaChallengeResponse {
    pub mfa_required: bool,
    pub mfa_session_token: String,
    pub mfa_methods: Vec<String>,
    pub expires_in: u64,
    pub trust_device_available: bool,
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
    pub device_fingerprint: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HostedLoginTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_actions: Vec<PendingActionResponse>,
}

/// Adaptive MFA policy response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AdaptiveMfaPolicyResponse {
    pub tenant_id: String,
    pub mode: AdaptiveMfaMode,
    pub risk_threshold: u8,
    pub always_require_for_admins: bool,
    pub trust_device_days: u16,
    pub step_up_operations: Vec<String>,
}

/// Update adaptive MFA policy request
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateAdaptiveMfaPolicyRequest {
    pub mode: Option<AdaptiveMfaMode>,
    pub risk_threshold: Option<u8>,
    pub always_require_for_admins: Option<bool>,
    pub trust_device_days: Option<u16>,
    pub step_up_operations: Option<Vec<String>>,
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
    Json(input): Json<TotpEnrollStartRequest>,
) -> Result<Json<SuccessResponse<TotpEnrollmentResponse>>> {
    let claims = HasServices::jwt_manager(&state).verify_identity_token(bearer.token())?;
    let user_id = &claims.sub;
    let email = &claims.email;

    // Verify current password before allowing MFA enrollment (ASVS V7.3)
    let user = state.user_service().get_by_email(email).await?;
    let password_valid = state
        .identity_engine()
        .user_store()
        .validate_user_password(&user.identity_subject, &input.current_password)
        .await?;
    if !password_valid {
        return Err(AppError::Forbidden(
            "Current password is incorrect".to_string(),
        ));
    }

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
) -> Result<Json<MessageResponse>> {
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

    Ok(Json(MessageResponse::new("TOTP removed successfully.")))
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
    state
        .user_service()
        .set_email_otp_enabled(user.id, true)
        .await?;

    let totp_enabled = state.totp_service().has_totp(user_id).await?;
    let webauthn_creds = state
        .webauthn_service()
        .list_credentials(user_id, None)
        .await?;
    let recovery_codes_remaining = state
        .recovery_code_service()
        .remaining_count(user_id)
        .await?;

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
    state
        .user_service()
        .set_email_otp_enabled(user.id, false)
        .await?;

    let totp_enabled = state.totp_service().has_totp(user_id).await?;
    let webauthn_creds = state
        .webauthn_service()
        .list_credentials(user_id, None)
        .await?;
    let recovery_codes_remaining = state
        .recovery_code_service()
        .remaining_count(user_id)
        .await?;

    Ok(Json(SuccessResponse::new(MfaStatusResponse {
        totp_enabled,
        webauthn_enabled: !webauthn_creds.is_empty(),
        recovery_codes_remaining,
        email_otp_enabled: false,
    })))
}

// ==================== MFA Challenge Endpoints (public, during login) ====================

/// POST /api/v1/mfa/challenge/totp
pub async fn challenge_totp<
    S: HasMfa + HasCache + HasServices + HasSessionManagement + HasTrustedDevices + HasAdaptiveMfa + HasRequiredActions,
>(
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

    // Optionally trust the device after successful MFA
    maybe_trust_device(&state, &session_data, input.trust_device).await;

    issue_token_after_mfa(&state, &session_data).await
}

/// POST /api/v1/mfa/challenge/recovery-code
pub async fn challenge_recovery_code<
    S: HasMfa + HasCache + HasServices + HasSessionManagement + HasTrustedDevices + HasAdaptiveMfa + HasRequiredActions,
>(
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

    // Optionally trust the device after successful MFA
    maybe_trust_device(&state, &session_data, input.trust_device).await;

    issue_token_after_mfa(&state, &session_data).await
}

// ==================== Trusted Device Endpoints (authenticated) ====================

/// GET /api/v1/mfa/trusted-devices
pub async fn list_trusted_devices<S: HasServices + HasTrustedDevices>(
    State(state): State<S>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<SuccessResponse<Vec<TrustedDevice>>>> {
    let claims = HasServices::jwt_manager(&state).verify_identity_token(bearer.token())?;
    let user_id = parse_user_id(&claims.sub)?;

    let devices = state.trusted_device_service().list_devices(user_id).await?;
    Ok(Json(SuccessResponse::new(devices)))
}

/// DELETE /api/v1/mfa/trusted-devices/{id}
pub async fn revoke_trusted_device<S: HasServices + HasTrustedDevices>(
    State(state): State<S>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
    Path(device_id): Path<String>,
) -> Result<Json<MessageResponse>> {
    let _claims = HasServices::jwt_manager(&state).verify_identity_token(bearer.token())?;

    let device_uuid = uuid::Uuid::parse_str(&device_id)
        .map_err(|_| AppError::BadRequest("Invalid device ID".to_string()))?;

    state
        .trusted_device_service()
        .revoke_device(StringUuid::from(device_uuid))
        .await?;

    Ok(Json(MessageResponse::new(
        "Trusted device revoked successfully.",
    )))
}

/// DELETE /api/v1/mfa/trusted-devices
pub async fn revoke_all_trusted_devices<S: HasServices + HasTrustedDevices>(
    State(state): State<S>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<MessageResponse>> {
    let claims = HasServices::jwt_manager(&state).verify_identity_token(bearer.token())?;
    let user_id = parse_user_id(&claims.sub)?;

    let count = state.trusted_device_service().revoke_all(user_id).await?;

    Ok(Json(MessageResponse::new(&format!(
        "Revoked {} trusted device(s).",
        count
    ))))
}

// ==================== Adaptive MFA Policy Endpoints (authenticated) ====================

/// GET /api/v1/mfa/adaptive-policy
pub async fn get_adaptive_mfa_policy<S: HasServices + HasAdaptiveMfa>(
    State(state): State<S>,
    auth: AuthUser,
) -> Result<Json<SuccessResponse<AdaptiveMfaPolicyResponse>>> {
    use crate::policy::{enforce, PolicyAction, PolicyInput, ResourceScope};

    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::SecurityAlertRead,
            scope: ResourceScope::Global,
        },
    )?;

    let tenant_uuid = auth.tenant_id.unwrap_or(uuid::Uuid::nil());
    let tenant_id = StringUuid::from(tenant_uuid);

    let policy = match state
        .adaptive_mfa_policy_repo()
        .find_by_tenant_id(tenant_id)
        .await?
    {
        Some(row) => AdaptiveMfaPolicyResponse {
            tenant_id: row.tenant_id.to_string(),
            mode: row
                .mode
                .parse::<AdaptiveMfaMode>()
                .unwrap_or(AdaptiveMfaMode::Always),
            risk_threshold: row.risk_threshold,
            always_require_for_admins: row.always_require_for_admins,
            trust_device_days: row.trust_device_days,
            step_up_operations: row.step_up_operations,
        },
        None => {
            let default = AdaptiveMfaPolicy::default_for_tenant(&tenant_id.to_string());
            AdaptiveMfaPolicyResponse {
                tenant_id: default.tenant_id,
                mode: default.mode,
                risk_threshold: default.risk_threshold,
                always_require_for_admins: default.always_require_for_admins,
                trust_device_days: default.trust_device_days,
                step_up_operations: default.step_up_operations,
            }
        }
    };

    Ok(Json(SuccessResponse::new(policy)))
}

/// PUT /api/v1/mfa/adaptive-policy
pub async fn update_adaptive_mfa_policy<S: HasServices + HasAdaptiveMfa>(
    State(state): State<S>,
    auth: AuthUser,
    Json(body): Json<UpdateAdaptiveMfaPolicyRequest>,
) -> Result<Json<SuccessResponse<AdaptiveMfaPolicyResponse>>> {
    use crate::policy::{enforce, PolicyAction, PolicyInput, ResourceScope};

    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::SecurityAlertResolve, // write permission
            scope: ResourceScope::Global,
        },
    )?;

    let tenant_uuid = auth.tenant_id.unwrap_or(uuid::Uuid::nil());
    let tenant_id = StringUuid::from(tenant_uuid);

    let existing: Option<AdaptiveMfaPolicyRow> = state
        .adaptive_mfa_policy_repo()
        .find_by_tenant_id(tenant_id)
        .await?;

    let default = AdaptiveMfaPolicy::default_for_tenant(&tenant_id.to_string());

    let mode = body.mode.unwrap_or_else(|| {
        existing
            .as_ref()
            .and_then(|r| r.mode.parse::<AdaptiveMfaMode>().ok())
            .unwrap_or(default.mode)
    });

    let row = AdaptiveMfaPolicyRow {
        id: existing
            .as_ref()
            .map(|r| r.id)
            .unwrap_or_else(StringUuid::new_v4),
        tenant_id,
        mode: mode.to_string(),
        risk_threshold: body.risk_threshold.unwrap_or(
            existing
                .as_ref()
                .map(|r| r.risk_threshold)
                .unwrap_or(default.risk_threshold),
        ),
        always_require_for_admins: body.always_require_for_admins.unwrap_or(
            existing
                .as_ref()
                .map(|r| r.always_require_for_admins)
                .unwrap_or(default.always_require_for_admins),
        ),
        trust_device_days: body.trust_device_days.unwrap_or(
            existing
                .as_ref()
                .map(|r| r.trust_device_days)
                .unwrap_or(default.trust_device_days),
        ),
        step_up_operations: body.step_up_operations.unwrap_or_else(|| {
            existing
                .as_ref()
                .map(|r| r.step_up_operations.clone())
                .unwrap_or(default.step_up_operations)
        }),
        created_at: existing
            .as_ref()
            .map(|r| r.created_at)
            .unwrap_or_else(chrono::Utc::now),
        updated_at: chrono::Utc::now(),
    };

    state.adaptive_mfa_policy_repo().upsert(&row).await?;

    let policy = AdaptiveMfaPolicyResponse {
        tenant_id: row.tenant_id.to_string(),
        mode,
        risk_threshold: row.risk_threshold,
        always_require_for_admins: row.always_require_for_admins,
        trust_device_days: row.trust_device_days,
        step_up_operations: row.step_up_operations,
    };

    Ok(Json(SuccessResponse::new(policy)))
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

/// After successful MFA verification, optionally trust the device.
async fn maybe_trust_device<S: HasServices + HasTrustedDevices + HasAdaptiveMfa>(
    state: &S,
    session_data: &MfaSessionData,
    trust_device: Option<bool>,
) {
    if trust_device != Some(true) {
        return;
    }
    let fingerprint = match &session_data.device_fingerprint {
        Some(fp) => fp.clone(),
        None => return,
    };

    let user_id = match parse_user_id(&session_data.user_id) {
        Ok(uid) => uid,
        Err(_) => return,
    };

    // Load policy to get trust_device_days
    let trust_days = {
        let tenant_memberships = state
            .user_service()
            .get_user_tenants(user_id)
            .await
            .unwrap_or_default();

        if let Some(first) = tenant_memberships.first() {
            match state
                .adaptive_mfa_policy_repo()
                .find_by_tenant_id(first.tenant_id)
                .await
            {
                Ok(Some(row)) => row.trust_device_days,
                _ => 30, // default
            }
        } else {
            30
        }
    };

    let tenant_id = {
        let memberships = state
            .user_service()
            .get_user_tenants(user_id)
            .await
            .unwrap_or_default();
        memberships.first().map(|m| m.tenant_id)
    };

    if let Err(e) = state
        .trusted_device_service()
        .trust_device(
            user_id,
            tenant_id,
            &fingerprint,
            Some("Browser (via MFA)"),
            trust_days,
        )
        .await
    {
        tracing::warn!(error = %e, "Failed to trust device after MFA");
    }
}

async fn issue_token_after_mfa<S: HasServices + HasSessionManagement + HasRequiredActions>(
    state: &S,
    session_data: &MfaSessionData,
) -> Result<Json<HostedLoginTokenResponse>> {
    let user_id = uuid::Uuid::parse_str(&session_data.user_id)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user_id in MFA session")))?;

    let user_id_su: StringUuid = user_id.into();

    let session = state
        .session_service()
        .create_session(
            user_id_su,
            None,
            session_data.ip_address.clone(),
            session_data.user_agent.clone(),
        )
        .await?;

    // Check for pending required actions (password expiry, temporary password, etc.)
    let pending_actions = {
        let user = state.user_service().get(user_id_su).await.ok();
        let tenant_policy = {
            let memberships = state
                .user_service()
                .get_user_tenants(user_id_su)
                .await
                .unwrap_or_default();
            if let Some(first) = memberships.first() {
                state
                    .tenant_service()
                    .get(first.tenant_id)
                    .await
                    .ok()
                    .and_then(|t| t.password_policy)
                    .unwrap_or_default()
            } else {
                crate::models::password::PasswordPolicy::default()
            }
        };
        let password_changed_at = user.and_then(|u| u.password_changed_at);
        match state
            .required_actions_service()
            .check_post_login_actions(
                &session_data.identity_subject,
                true,  // mfa_enabled — always true in MFA flow
                true,  // has_mfa_credential — user just verified
                password_changed_at,
                tenant_policy.max_age_days,
            )
            .await
        {
            Ok(actions) => actions,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to check pending actions after MFA, proceeding without");
                Vec::new()
            }
        }
    };

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
        pending_actions,
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
            device_fingerprint: Some("abc123".to_string()),
        };
        let json = serde_json::to_string(&data).unwrap();
        let parsed: MfaSessionData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.user_id, "user-1");
        assert_eq!(parsed.email, "test@example.com");
        assert_eq!(parsed.device_fingerprint, Some("abc123".to_string()));
    }

    #[test]
    fn test_mfa_session_data_backward_compat() {
        // Ensure old sessions without device_fingerprint still deserialize
        let json = r#"{"user_id":"u1","email":"e@e.com","display_name":null,"identity_subject":"s1","ip_address":null,"user_agent":null}"#;
        let parsed: MfaSessionData = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.device_fingerprint, None);
    }

    #[test]
    fn test_mfa_challenge_response_serde() {
        let response = MfaChallengeResponse {
            mfa_required: true,
            mfa_session_token: "token-123".to_string(),
            mfa_methods: vec!["totp".to_string()],
            expires_in: 300,
            trust_device_available: true,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("mfa_required"));
        assert!(json.contains("mfa_session_token"));
        assert!(json.contains("trust_device_available"));
    }

    #[test]
    fn test_mfa_challenge_verify_request_defaults() {
        let json = r#"{"mfa_session_token":"tok","code":"123456"}"#;
        let req: MfaChallengeVerifyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.trust_device, None);
    }

    #[test]
    fn test_mfa_challenge_verify_request_with_trust() {
        let json = r#"{"mfa_session_token":"tok","code":"123456","trust_device":true}"#;
        let req: MfaChallengeVerifyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.trust_device, Some(true));
    }

    #[test]
    fn test_adaptive_mfa_policy_response_serde() {
        let policy = AdaptiveMfaPolicyResponse {
            tenant_id: "t1".to_string(),
            mode: AdaptiveMfaMode::Adaptive,
            risk_threshold: 40,
            always_require_for_admins: true,
            trust_device_days: 30,
            step_up_operations: vec!["change_password".to_string()],
        };
        let json = serde_json::to_string(&policy).unwrap();
        assert!(json.contains("adaptive"));
        assert!(json.contains("risk_threshold"));
    }
}
