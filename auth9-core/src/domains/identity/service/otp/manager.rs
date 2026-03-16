//! OTP generation, storage, and verification

use super::channel::OtpChannelType;
use super::rate_limit::OtpRateLimitConfig;
use crate::cache::CacheOperations;
use crate::error::{AppError, Result};
use rand::Rng;
use std::sync::Arc;

/// OTP manager handling code generation, storage, verification, and rate limiting
pub struct OtpManager<C: CacheOperations> {
    cache: Arc<C>,
}

impl<C: CacheOperations> OtpManager<C> {
    pub fn new(cache: Arc<C>) -> Self {
        Self { cache }
    }

    /// Generate a 6-digit cryptographically secure random verification code
    pub fn generate_code() -> String {
        let n: u32 = rand::thread_rng().gen_range(0..1_000_000);
        format!("{:06}", n)
    }

    /// Store an OTP code with rate limiting checks
    ///
    /// Checks: failure lockout → cooldown → daily limit → store
    pub async fn store(
        &self,
        channel: OtpChannelType,
        destination: &str,
        code: &str,
        ttl_secs: u64,
        rate_limit: &OtpRateLimitConfig,
    ) -> Result<()> {
        let fail_key = format!("{}:{}:{}", crate::cache::keys::OTP_FAIL, channel, destination);
        let cooldown_key = format!(
            "{}:{}:{}",
            crate::cache::keys::OTP_COOLDOWN,
            channel,
            destination
        );
        let daily_key = format!(
            "{}:{}:{}",
            crate::cache::keys::OTP_DAILY,
            channel,
            destination
        );
        let otp_key = format!("{}:{}:{}", crate::cache::keys::OTP, channel, destination);

        // 1. Check failure lockout
        let failures = self.cache.get_counter(&fail_key).await?;
        if failures >= rate_limit.max_failures {
            return Err(AppError::BadRequest(
                "Too many failed attempts. Please try again later.".to_string(),
            ));
        }

        // 2. Check cooldown (skip if cooldown is 0)
        if rate_limit.cooldown_secs > 0 {
            let already_set = self
                .cache
                .set_flag(&cooldown_key, rate_limit.cooldown_secs)
                .await?;
            if already_set {
                return Err(AppError::BadRequest(
                    "Please wait before requesting a new code.".to_string(),
                ));
            }
        }

        // 3. Check daily limit
        let daily_count = self.cache.increment_counter(&daily_key, 86400).await?;
        if daily_count > rate_limit.daily_max {
            return Err(AppError::BadRequest(
                "Daily OTP limit reached.".to_string(),
            ));
        }

        // 4. Store OTP
        self.cache.store_otp(&otp_key, code, ttl_secs).await
    }

    /// Verify and consume an OTP code (one-time use)
    ///
    /// Returns Ok(()) on success, Err on failure (wrong code, expired, locked out)
    pub async fn verify_and_consume(
        &self,
        channel: OtpChannelType,
        destination: &str,
        code: &str,
        rate_limit: &OtpRateLimitConfig,
    ) -> Result<()> {
        let fail_key = format!("{}:{}:{}", crate::cache::keys::OTP_FAIL, channel, destination);
        let otp_key = format!("{}:{}:{}", crate::cache::keys::OTP, channel, destination);

        // 1. Check failure lockout
        let failures = self.cache.get_counter(&fail_key).await?;
        if failures >= rate_limit.max_failures {
            return Err(AppError::BadRequest(
                "Too many failed attempts. Please try again later.".to_string(),
            ));
        }

        // 2. Get stored OTP
        let stored = self.cache.get_otp(&otp_key).await?;

        match stored {
            Some(stored_code) if stored_code == code => {
                // Correct code — consume it
                self.cache.remove_otp(&otp_key).await?;
                Ok(())
            }
            _ => {
                // Wrong or expired code — increment failure counter
                self.cache
                    .increment_counter(&fail_key, rate_limit.lockout_secs)
                    .await?;
                Err(AppError::BadRequest(
                    "Invalid or expired verification code.".to_string(),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::NoOpCacheManager;
    use std::collections::HashSet;

    fn test_cache() -> Arc<NoOpCacheManager> {
        Arc::new(NoOpCacheManager::new())
    }

    fn test_rate_limit() -> OtpRateLimitConfig {
        OtpRateLimitConfig {
            cooldown_secs: 60,
            daily_max: 3,
            max_failures: 3,
            lockout_secs: 900,
        }
    }

    #[test]
    fn test_generate_code_six_digits() {
        let code = OtpManager::<NoOpCacheManager>::generate_code();
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_generate_code_unique() {
        let codes: HashSet<String> = (0..100)
            .map(|_| OtpManager::<NoOpCacheManager>::generate_code())
            .collect();
        // With 1M possible codes, 100 draws should yield at least 95 unique
        assert!(codes.len() >= 95);
    }

    #[tokio::test]
    async fn test_store_and_verify_success() {
        let cache = test_cache();
        let manager = OtpManager::new(cache);
        let rl = test_rate_limit();

        manager
            .store(OtpChannelType::Email, "user@example.com", "123456", 600, &rl)
            .await
            .unwrap();

        manager
            .verify_and_consume(OtpChannelType::Email, "user@example.com", "123456", &rl)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_verify_wrong_code() {
        let cache = test_cache();
        let manager = OtpManager::new(cache);
        let rl = test_rate_limit();

        manager
            .store(OtpChannelType::Email, "user@example.com", "123456", 600, &rl)
            .await
            .unwrap();

        let result = manager
            .verify_and_consume(OtpChannelType::Email, "user@example.com", "999999", &rl)
            .await;
        assert!(result.is_err());
        if let Err(AppError::BadRequest(msg)) = result {
            assert!(msg.contains("Invalid or expired"));
        }
    }

    #[tokio::test]
    async fn test_verify_consumed_code() {
        let cache = test_cache();
        let manager = OtpManager::new(cache);
        let rl = test_rate_limit();

        manager
            .store(OtpChannelType::Email, "user@example.com", "123456", 600, &rl)
            .await
            .unwrap();

        manager
            .verify_and_consume(OtpChannelType::Email, "user@example.com", "123456", &rl)
            .await
            .unwrap();

        // Second verify should fail — code consumed
        let result = manager
            .verify_and_consume(OtpChannelType::Email, "user@example.com", "123456", &rl)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_verify_no_code_stored() {
        let cache = test_cache();
        let manager = OtpManager::new(cache);
        let rl = test_rate_limit();

        let result = manager
            .verify_and_consume(OtpChannelType::Email, "user@example.com", "123456", &rl)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rate_limit_cooldown() {
        let cache = test_cache();
        let manager = OtpManager::new(cache);
        let rl = test_rate_limit();

        // First store succeeds
        manager
            .store(OtpChannelType::Email, "user@example.com", "111111", 600, &rl)
            .await
            .unwrap();

        // Second store immediately fails due to cooldown
        let result = manager
            .store(OtpChannelType::Email, "user@example.com", "222222", 600, &rl)
            .await;
        assert!(result.is_err());
        if let Err(AppError::BadRequest(msg)) = result {
            assert!(msg.contains("wait"));
        }
    }

    #[tokio::test]
    async fn test_rate_limit_daily_max() {
        let cache = test_cache();
        let manager = OtpManager::new(cache);
        // No cooldown so we can test daily limit
        let rl = OtpRateLimitConfig {
            cooldown_secs: 0,
            daily_max: 2,
            max_failures: 10,
            lockout_secs: 900,
        };

        // Store 1 and 2 succeed
        manager
            .store(OtpChannelType::Email, "user@example.com", "111111", 600, &rl)
            .await
            .unwrap();
        manager
            .store(OtpChannelType::Email, "user@example.com", "222222", 600, &rl)
            .await
            .unwrap();

        // Store 3 exceeds daily limit
        let result = manager
            .store(OtpChannelType::Email, "user@example.com", "333333", 600, &rl)
            .await;
        assert!(result.is_err());
        if let Err(AppError::BadRequest(msg)) = result {
            assert!(msg.contains("Daily OTP limit"));
        }
    }

    #[tokio::test]
    async fn test_rate_limit_failure_lockout() {
        let cache = test_cache();
        let manager = OtpManager::new(cache);
        let rl = OtpRateLimitConfig {
            cooldown_secs: 0,
            daily_max: 100,
            max_failures: 2,
            lockout_secs: 900,
        };

        manager
            .store(OtpChannelType::Email, "user@example.com", "123456", 600, &rl)
            .await
            .unwrap();

        // Fail twice
        let _ = manager
            .verify_and_consume(OtpChannelType::Email, "user@example.com", "000000", &rl)
            .await;
        let _ = manager
            .verify_and_consume(OtpChannelType::Email, "user@example.com", "000000", &rl)
            .await;

        // Third attempt should be locked out even with correct code
        let result = manager
            .verify_and_consume(OtpChannelType::Email, "user@example.com", "123456", &rl)
            .await;
        assert!(result.is_err());
        if let Err(AppError::BadRequest(msg)) = result {
            assert!(msg.contains("Too many failed attempts"));
        }
    }

    #[tokio::test]
    async fn test_different_channels_isolated() {
        let cache = test_cache();
        let manager = OtpManager::new(cache);
        let rl = OtpRateLimitConfig {
            cooldown_secs: 0,
            daily_max: 100,
            max_failures: 10,
            lockout_secs: 900,
        };

        manager
            .store(OtpChannelType::Email, "user@example.com", "111111", 600, &rl)
            .await
            .unwrap();

        manager
            .store(OtpChannelType::Sms, "user@example.com", "222222", 600, &rl)
            .await
            .unwrap();

        // Verify email channel code
        manager
            .verify_and_consume(OtpChannelType::Email, "user@example.com", "111111", &rl)
            .await
            .unwrap();

        // Verify SMS channel code
        manager
            .verify_and_consume(OtpChannelType::Sms, "user@example.com", "222222", &rl)
            .await
            .unwrap();
    }
}
