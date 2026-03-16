//! OTP (One-Time Password) service layer
//!
//! Provides shared infrastructure for Email OTP and SMS OTP:
//! - Channel abstraction (email, SMS)
//! - Code generation and Redis storage
//! - One-time verification and consumption
//! - Rate limiting (cooldown, daily cap, failure lockout)

pub mod channel;
pub mod email_channel;
pub mod manager;
pub mod rate_limit;
pub mod sms_channel;

pub use channel::{OtpChannel, OtpChannelType};
pub use email_channel::EmailOtpChannel;
pub use manager::OtpManager;
pub use rate_limit::OtpRateLimitConfig;
pub use sms_channel::SmsOtpChannel;
