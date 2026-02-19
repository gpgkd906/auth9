//! Branding configuration domain types

use super::common::validate_url_no_ssrf_strict_option;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

lazy_static::lazy_static! {
    /// Patterns that are dangerous in custom CSS and must be rejected.
    /// Matches: @import, url(), expression(), -moz-binding, behavior, javascript:,
    /// content property, ::before/::after pseudo-elements, and element-hiding tricks.
    static ref DANGEROUS_CSS_PATTERNS: Vec<regex::Regex> = vec![
        // @import - external stylesheet loading
        regex::Regex::new(r"(?i)@import\b").unwrap(),
        // url() - external resource references (images, fonts, etc.)
        regex::Regex::new(r"(?i)\burl\s*\(").unwrap(),
        // expression() - IE CSS expressions (JavaScript execution)
        regex::Regex::new(r"(?i)\bexpression\s*\(").unwrap(),
        // -moz-binding - Firefox XBL binding (JavaScript execution)
        regex::Regex::new(r"(?i)-moz-binding\s*:").unwrap(),
        // behavior - IE behavior (JavaScript execution)
        regex::Regex::new(r"(?i)\bbehavior\s*:").unwrap(),
        // javascript: protocol in any value
        regex::Regex::new(r"(?i)javascript\s*:").unwrap(),
        // content property - used with ::before/::after to inject fake text/UI
        regex::Regex::new(r"(?i)\bcontent\s*:").unwrap(),
        // ::before / ::after pseudo-elements - primary vector for content injection
        regex::Regex::new(r"(?i)::?\s*(before|after)\b").unwrap(),
        // display: none - used to hide security-critical form elements
        regex::Regex::new(r"(?i)\bdisplay\s*:\s*none\b").unwrap(),
        // visibility: hidden - used to hide elements while preserving layout
        regex::Regex::new(r"(?i)\bvisibility\s*:\s*hidden\b").unwrap(),
        // opacity: 0 - used to make elements invisible
        regex::Regex::new(r"(?i)\bopacity\s*:\s*0([^.]|$)").unwrap(),
    ];
}

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

/// Validate custom CSS for dangerous patterns.
///
/// Blocks: @import, url(), expression(), -moz-binding, behavior, javascript:,
/// content (pseudo-element text injection), ::before/::after, display:none,
/// visibility:hidden, opacity:0 (element hiding/spoofing attacks).
fn validate_custom_css(css: &str) -> Result<(), validator::ValidationError> {
    for pattern in DANGEROUS_CSS_PATTERNS.iter() {
        if pattern.is_match(css) {
            let mut err = validator::ValidationError::new("dangerous_css_pattern");
            err.message = Some(
                "Custom CSS contains forbidden patterns (content injection, element hiding, or resource loading are not allowed)"
                    .into(),
            );
            return Err(err);
        }
    }
    Ok(())
}

/// Wrapper for Option<String> custom CSS validation (called by validator derive on inner &String)
fn validate_custom_css_option(css: &str) -> Result<(), validator::ValidationError> {
    validate_custom_css(css)
}

/// Branding configuration for login pages
#[derive(Debug, Clone, Serialize, Deserialize, Validate, PartialEq, ToSchema)]
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

    /// Custom CSS (max 50KB, no external resource loading)
    #[validate(length(max = 51200, message = "Custom CSS exceeds maximum size of 50KB"))]
    #[validate(custom(function = "validate_custom_css_option"))]
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
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
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
    fn test_custom_css_blocks_import() {
        let config = BrandingConfig {
            custom_css: Some("@import url(https://attacker.example/x.css);".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_custom_css_blocks_import_case_insensitive() {
        let config = BrandingConfig {
            custom_css: Some("@IMPORT url('https://evil.com/x.css');".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_custom_css_blocks_url() {
        let config = BrandingConfig {
            custom_css: Some("body { background: url(https://evil.com/track.gif); }".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_custom_css_blocks_expression() {
        let config = BrandingConfig {
            custom_css: Some("div { width: expression(alert(1)); }".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_custom_css_blocks_javascript() {
        let config = BrandingConfig {
            custom_css: Some("div { background: javascript:alert(1); }".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_custom_css_blocks_moz_binding() {
        let config = BrandingConfig {
            custom_css: Some("div { -moz-binding: url('xbl'); }".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_custom_css_blocks_behavior() {
        let config = BrandingConfig {
            custom_css: Some("div { behavior: url(xss.htc); }".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_custom_css_blocks_content_property() {
        let config = BrandingConfig {
            custom_css: Some(
                r#"#password::after { content: "YOUR PASSWORD HAS BEEN COMPROMISED!"; }"#
                    .to_string(),
            ),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_custom_css_blocks_before_pseudo_element() {
        let config = BrandingConfig {
            custom_css: Some(".login-title::before { content: 'SECURE BANK LOGIN'; }".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_custom_css_blocks_after_pseudo_element() {
        let config = BrandingConfig {
            custom_css: Some(".header::after { color: red; }".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_custom_css_blocks_display_none() {
        let config = BrandingConfig {
            custom_css: Some("#password { display: none !important; }".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_custom_css_blocks_visibility_hidden() {
        let config = BrandingConfig {
            custom_css: Some("#password { visibility: hidden; }".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_custom_css_blocks_opacity_zero() {
        let config = BrandingConfig {
            custom_css: Some("#password { opacity: 0; }".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_custom_css_allows_nonzero_opacity() {
        let config = BrandingConfig {
            custom_css: Some(".card { opacity: 0.8; }".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_custom_css_blocks_display_none_case_insensitive() {
        let config = BrandingConfig {
            custom_css: Some("#password { DISPLAY: NONE; }".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_custom_css_allows_safe_css() {
        let config = BrandingConfig {
            custom_css: Some(
                ".login-form { border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); color: #333; font-size: 14px; }"
                    .to_string(),
            ),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_custom_css_allows_none() {
        let config = BrandingConfig {
            custom_css: None,
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
