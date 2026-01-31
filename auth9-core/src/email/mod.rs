//! Email sending functionality for Auth9
//!
//! This module provides email sending capabilities with multiple provider support:
//! - SMTP (using lettre)
//! - AWS SES
//! - Oracle Email Delivery (via SMTP)

pub mod provider;
pub mod smtp;
pub mod templates;

pub use provider::{EmailProvider, EmailProviderError};
pub use smtp::SmtpEmailProvider;
pub use templates::{EmailTemplate, TemplateEngine};
