//! Branding configuration domain types

use super::common::validate_url_no_ssrf_strict_option;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Default primary color
pub const DEFAULT_PRIMARY_COLOR: &str = "#007AFF";
/// Default secondary color
pub const DEFAULT_SECONDARY_COLOR: &str = "#5856D6";
/// Default background color
pub const DEFAULT_BACKGROUND_COLOR: &str = "#F5F5F7";
/// Default text color
pub const DEFAULT_TEXT_COLOR: &str = "#1D1D1F";
/// Maximum custom CSS size (50KB)
pub const MAX_CUSTOM_CSS_SIZE: usize = 50 * 1024;

lazy_static::lazy_static! {
    /// Regex for validating hex color codes
    static ref HEX_COLOR_REGEX: regex::Regex = regex::Regex::new(r"^#[0-9A-Fa-f]{6}$").unwrap();
}

/// Validate hex color format
fn validate_hex_color(color: &str) -> Result<(), validator::ValidationError> {
    if HEX_COLOR_REGEX.is_match(color) {
        Ok(())
    } else {
        Err(validator::ValidationError::new("invalid_hex_color"))
    }
}

/// Branding configuration for login pages
#[derive(Debug, Clone, Serialize, Deserialize, Validate, PartialEq)]
pub struct BrandingConfig {
    /// Logo URL (with SSRF protection)
    #[validate(custom(function = "validate_url_no_ssrf_strict_option"))]
    pub logo_url: Option<String>,

    /// Primary color (hex format, e.g., "#007AFF")
    #[validate(custom(function = "validate_hex_color"))]
    pub primary_color: String,

    /// Secondary color (hex format)
    #[validate(custom(function = "validate_hex_color"))]
    pub secondary_color: String,

    /// Background color (hex format)
    #[validate(custom(function = "validate_hex_color"))]
    pub background_color: String,

    /// Text color (hex format)
    #[validate(custom(function = "validate_hex_color"))]
    pub text_color: String,

    /// Custom CSS (max 50KB)
    #[validate(length(max = 51200, message = "Custom CSS exceeds maximum size of 50KB"))]
    pub custom_css: Option<String>,

    /// Company name displayed on login page
    #[validate(length(max = 100, message = "Company name exceeds maximum length"))]
    pub company_name: Option<String>,

    /// Favicon URL (with SSRF protection)
    #[validate(custom(function = "validate_url_no_ssrf_strict_option"))]
    pub favicon_url: Option<String>,

    /// Whether to allow showing registration link on login page (default: false)
    #[serde(default)]
    pub allow_registration: bool,
}

impl Default for BrandingConfig {
    fn default() -> Self {
        Self {
            logo_url: None,
            primary_color: DEFAULT_PRIMARY_COLOR.to_string(),
            secondary_color: DEFAULT_SECONDARY_COLOR.to_string(),
            background_color: DEFAULT_BACKGROUND_COLOR.to_string(),
            text_color: DEFAULT_TEXT_COLOR.to_string(),
            custom_css: None,
            company_name: None,
            favicon_url: None,
            allow_registration: false,
        }
    }
}

impl BrandingConfig {
    /// Create a new branding config with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if this is the default configuration (nothing customized)
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

/// Request body for updating branding settings
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateBrandingRequest {
    #[validate(nested)]
    pub config: BrandingConfig,
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_default_branding_config() {
        let config = BrandingConfig::default();
        assert_eq!(config.primary_color, DEFAULT_PRIMARY_COLOR);
        assert_eq!(config.secondary_color, DEFAULT_SECONDARY_COLOR);
        assert_eq!(config.background_color, DEFAULT_BACKGROUND_COLOR);
        assert_eq!(config.text_color, DEFAULT_TEXT_COLOR);
        assert!(config.logo_url.is_none());
        assert!(config.custom_css.is_none());
        assert!(config.company_name.is_none());
        assert!(config.favicon_url.is_none());
        assert!(!config.allow_registration);
    }

    #[test]
    fn test_is_default() {
        let config = BrandingConfig::default();
        assert!(config.is_default());

        let custom = BrandingConfig {
            company_name: Some("Test Corp".to_string()),
            ..BrandingConfig::default()
        };
        assert!(!custom.is_default());
    }

    #[test]
    fn test_valid_hex_colors() {
        let config = BrandingConfig {
            logo_url: None,
            primary_color: "#FF5733".to_string(),
            secondary_color: "#33FF57".to_string(),
            background_color: "#AABBCC".to_string(),
            text_color: "#000000".to_string(),
            custom_css: None,
            company_name: None,
            favicon_url: None,
            allow_registration: false,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_hex_color_no_hash() {
        let config = BrandingConfig {
            primary_color: "FF5733".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_hex_color_wrong_length() {
        let config = BrandingConfig {
            primary_color: "#FFF".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_hex_color_invalid_chars() {
        let config = BrandingConfig {
            primary_color: "#GGGGGG".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_valid_logo_url() {
        let config = BrandingConfig {
            logo_url: Some("https://example.com/logo.png".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_logo_url() {
        let config = BrandingConfig {
            logo_url: Some("not-a-url".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_logo_url_ssrf_private_ip_blocked() {
        let config = BrandingConfig {
            logo_url: Some("http://192.168.1.1/logo.png".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_logo_url_ssrf_cloud_metadata_blocked() {
        let config = BrandingConfig {
            logo_url: Some("http://169.254.169.254/latest/meta-data/".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_favicon_url_ssrf_private_ip_blocked() {
        let config = BrandingConfig {
            favicon_url: Some("http://192.168.1.1/favicon.ico".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_logo_url_localhost_http_blocked() {
        // Branding uses strict validation - no localhost/HTTP allowed
        let config = BrandingConfig {
            logo_url: Some("http://localhost:8080/logo.png".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_serialization() {
        let config = BrandingConfig {
            logo_url: Some("https://example.com/logo.png".to_string()),
            primary_color: "#007AFF".to_string(),
            secondary_color: "#5856D6".to_string(),
            background_color: "#F5F5F7".to_string(),
            text_color: "#1D1D1F".to_string(),
            custom_css: Some(".login { color: red; }".to_string()),
            company_name: Some("Test Corp".to_string()),
            favicon_url: Some("https://example.com/favicon.ico".to_string()),
            allow_registration: true,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("logo_url"));
        assert!(json.contains("#007AFF"));
        assert!(json.contains("Test Corp"));
        assert!(json.contains("allow_registration"));

        let parsed: BrandingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, config);
    }

    #[test]
    fn test_deserialization_with_defaults() {
        let json = r##"{"primary_color":"#FF0000","secondary_color":"#00FF00","background_color":"#0000FF","text_color":"#AABBCC"}"##;

        let config: BrandingConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.primary_color, "#FF0000");
        assert!(config.logo_url.is_none());
        assert!(config.custom_css.is_none());
    }

    #[test]
    fn test_company_name_max_length() {
        let config = BrandingConfig {
            company_name: Some("A".repeat(101)),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_company_name_within_limit() {
        let config = BrandingConfig {
            company_name: Some("A".repeat(100)),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_hex_color_lowercase() {
        let config = BrandingConfig {
            primary_color: "#aabbcc".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_update_branding_request() {
        let json = r##"{"config":{"primary_color":"#FF0000","secondary_color":"#00FF00","background_color":"#0000FF","text_color":"#AABBCC"}}"##;

        let request: UpdateBrandingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.config.primary_color, "#FF0000");
        assert!(request.validate().is_ok());
    }
}
