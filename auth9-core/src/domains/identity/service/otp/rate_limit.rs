//! OTP rate limiting configuration

/// Rate limiting configuration for OTP operations
#[derive(Debug, Clone)]
pub struct OtpRateLimitConfig {
    /// Cooldown period between sends (seconds)
    pub cooldown_secs: u64,
    /// Maximum sends within 24 hours
    pub daily_max: u64,
    /// Maximum consecutive verification failures before lockout
    pub max_failures: u64,
    /// Lockout duration after max failures (seconds)
    pub lockout_secs: u64,
}

impl OtpRateLimitConfig {
    /// Default rate limits for Email OTP
    pub fn email_defaults() -> Self {
        Self {
            cooldown_secs: 60,
            daily_max: 10,
            max_failures: 5,
            lockout_secs: 900, // 15 min
        }
    }

    /// Default rate limits for SMS OTP
    pub fn sms_defaults() -> Self {
        Self {
            cooldown_secs: 120,
            daily_max: 5,
            max_failures: 3,
            lockout_secs: 1800, // 30 min
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_defaults() {
        let config = OtpRateLimitConfig::email_defaults();
        assert_eq!(config.cooldown_secs, 60);
        assert_eq!(config.daily_max, 10);
        assert_eq!(config.max_failures, 5);
        assert_eq!(config.lockout_secs, 900);
    }

    #[test]
    fn test_sms_defaults() {
        let config = OtpRateLimitConfig::sms_defaults();
        assert_eq!(config.cooldown_secs, 120);
        assert_eq!(config.daily_max, 5);
        assert_eq!(config.max_failures, 3);
        assert_eq!(config.lockout_secs, 1800);
    }
}
