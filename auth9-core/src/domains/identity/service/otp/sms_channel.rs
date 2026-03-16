//! SMS OTP channel placeholder
//!
//! Will be implemented when SmsService (infra_sms_provider.md) is ready.

use super::channel::{OtpChannel, OtpChannelType};
use crate::error::{AppError, Result};
use async_trait::async_trait;

/// Placeholder SMS OTP channel
pub struct SmsOtpChannel;

#[async_trait]
impl OtpChannel for SmsOtpChannel {
    fn channel_type(&self) -> OtpChannelType {
        OtpChannelType::Sms
    }

    async fn send_code(&self, _destination: &str, _code: &str, _ttl_minutes: u32) -> Result<()> {
        Err(AppError::BadRequest(
            "SMS OTP channel not yet implemented".to_string(),
        ))
    }
}
