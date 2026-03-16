//! OTP channel abstraction

use crate::error::Result;
use async_trait::async_trait;
use std::fmt;

/// OTP delivery channel type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OtpChannelType {
    Email,
    Sms,
}

impl fmt::Display for OtpChannelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Email => write!(f, "email"),
            Self::Sms => write!(f, "sms"),
        }
    }
}

/// OTP delivery channel trait
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait OtpChannel: Send + Sync {
    /// Channel type identifier
    fn channel_type(&self) -> OtpChannelType;

    /// Send a verification code to the destination (email address or phone number)
    async fn send_code(&self, destination: &str, code: &str, ttl_minutes: u32) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_type_display() {
        assert_eq!(OtpChannelType::Email.to_string(), "email");
        assert_eq!(OtpChannelType::Sms.to_string(), "sms");
    }

    #[test]
    fn test_channel_type_equality() {
        assert_eq!(OtpChannelType::Email, OtpChannelType::Email);
        assert_ne!(OtpChannelType::Email, OtpChannelType::Sms);
    }
}
