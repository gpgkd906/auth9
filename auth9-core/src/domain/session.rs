//! Session management domain models

use super::common::StringUuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

/// User session entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Session {
    pub id: StringUuid,
    pub user_id: StringUuid,
    pub keycloak_session_id: Option<String>,
    pub device_type: Option<String>,
    pub device_name: Option<String>,
    pub ip_address: Option<String>,
    pub location: Option<String>,
    pub user_agent: Option<String>,
    pub last_active_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl Default for Session {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: StringUuid::new_v4(),
            user_id: StringUuid::new_v4(),
            keycloak_session_id: None,
            device_type: None,
            device_name: None,
            ip_address: None,
            location: None,
            user_agent: None,
            last_active_at: now,
            created_at: now,
            revoked_at: None,
        }
    }
}

/// Input for creating a new session
#[derive(Debug, Clone)]
pub struct CreateSessionInput {
    pub user_id: StringUuid,
    pub keycloak_session_id: Option<String>,
    pub device_type: Option<String>,
    pub device_name: Option<String>,
    pub ip_address: Option<String>,
    pub location: Option<String>,
    pub user_agent: Option<String>,
}

/// Keycloak session representation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakSession {
    pub id: String,
    pub username: Option<String>,
    pub user_id: Option<String>,
    pub ip_address: Option<String>,
    pub start: Option<i64>,
    pub last_access: Option<i64>,
    #[serde(default)]
    pub clients: std::collections::HashMap<String, String>,
}

/// Session info returned to clients
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionInfo {
    pub id: String,
    pub device_type: Option<String>,
    pub device_name: Option<String>,
    pub ip_address: Option<String>,
    pub location: Option<String>,
    pub last_active_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub is_current: bool,
}

impl From<Session> for SessionInfo {
    fn from(session: Session) -> Self {
        Self {
            id: session.id.to_string(),
            device_type: session.device_type,
            device_name: session.device_name,
            ip_address: session.ip_address,
            location: session.location,
            last_active_at: session.last_active_at,
            created_at: session.created_at,
            is_current: false,
        }
    }
}

/// Parse user agent string to extract device info
pub fn parse_user_agent(user_agent: &str) -> (Option<String>, Option<String>) {
    let ua = user_agent.to_lowercase();

    // Detect device type - check tablet/ipad first to prevent mobile from matching
    let device_type = if ua.contains("tablet") || ua.contains("ipad") {
        Some("tablet".to_string())
    } else if ua.contains("mobile") || (ua.contains("android") && !ua.contains("tablet")) {
        Some("mobile".to_string())
    } else {
        Some("desktop".to_string())
    };

    // Extract browser and OS info
    // Note: Android user agents contain "Linux", so check for Android first
    let device_name = if ua.contains("chrome") && !ua.contains("edg") {
        if ua.contains("android") {
            Some("Chrome on Android".to_string())
        } else if ua.contains("windows") {
            Some("Chrome on Windows".to_string())
        } else if ua.contains("mac") {
            Some("Chrome on macOS".to_string())
        } else if ua.contains("linux") {
            Some("Chrome on Linux".to_string())
        } else {
            Some("Chrome".to_string())
        }
    } else if ua.contains("firefox") {
        if ua.contains("android") {
            Some("Firefox on Android".to_string())
        } else if ua.contains("windows") {
            Some("Firefox on Windows".to_string())
        } else if ua.contains("mac") {
            Some("Firefox on macOS".to_string())
        } else if ua.contains("linux") {
            Some("Firefox on Linux".to_string())
        } else {
            Some("Firefox".to_string())
        }
    } else if ua.contains("safari") && !ua.contains("chrome") {
        if ua.contains("iphone") {
            Some("Safari on iPhone".to_string())
        } else if ua.contains("ipad") {
            Some("Safari on iPad".to_string())
        } else if ua.contains("mac") {
            Some("Safari on macOS".to_string())
        } else {
            Some("Safari".to_string())
        }
    } else if ua.contains("edg") {
        if ua.contains("windows") {
            Some("Edge on Windows".to_string())
        } else if ua.contains("mac") {
            Some("Edge on macOS".to_string())
        } else {
            Some("Edge".to_string())
        }
    } else {
        Some("Unknown Browser".to_string())
    };

    (device_type, device_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_default() {
        let session = Session::default();
        assert!(!session.id.is_nil());
        assert!(!session.user_id.is_nil());
        assert!(session.keycloak_session_id.is_none());
        assert!(session.revoked_at.is_none());
    }

    #[test]
    fn test_session_info_from_session() {
        let session = Session {
            device_type: Some("desktop".to_string()),
            device_name: Some("Chrome on macOS".to_string()),
            ip_address: Some("192.168.1.1".to_string()),
            ..Default::default()
        };

        let info: SessionInfo = session.into();
        assert_eq!(info.device_type, Some("desktop".to_string()));
        assert_eq!(info.device_name, Some("Chrome on macOS".to_string()));
        assert!(!info.is_current);
    }

    #[test]
    fn test_parse_user_agent_chrome_windows() {
        let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
        let (device_type, device_name) = parse_user_agent(ua);

        assert_eq!(device_type, Some("desktop".to_string()));
        assert_eq!(device_name, Some("Chrome on Windows".to_string()));
    }

    #[test]
    fn test_parse_user_agent_chrome_mac() {
        let ua = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
        let (device_type, device_name) = parse_user_agent(ua);

        assert_eq!(device_type, Some("desktop".to_string()));
        assert_eq!(device_name, Some("Chrome on macOS".to_string()));
    }

    #[test]
    fn test_parse_user_agent_firefox() {
        let ua = "Mozilla/5.0 (X11; Linux x86_64; rv:120.0) Gecko/20100101 Firefox/120.0";
        let (device_type, device_name) = parse_user_agent(ua);

        assert_eq!(device_type, Some("desktop".to_string()));
        assert_eq!(device_name, Some("Firefox on Linux".to_string()));
    }

    #[test]
    fn test_parse_user_agent_safari_iphone() {
        let ua = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1";
        let (device_type, device_name) = parse_user_agent(ua);

        assert_eq!(device_type, Some("mobile".to_string()));
        assert_eq!(device_name, Some("Safari on iPhone".to_string()));
    }

    #[test]
    fn test_parse_user_agent_safari_ipad() {
        let ua = "Mozilla/5.0 (iPad; CPU OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1";
        let (device_type, device_name) = parse_user_agent(ua);

        assert_eq!(device_type, Some("tablet".to_string()));
        assert_eq!(device_name, Some("Safari on iPad".to_string()));
    }

    #[test]
    fn test_parse_user_agent_edge_windows() {
        let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0";
        let (device_type, device_name) = parse_user_agent(ua);

        assert_eq!(device_type, Some("desktop".to_string()));
        assert_eq!(device_name, Some("Edge on Windows".to_string()));
    }

    #[test]
    fn test_parse_user_agent_android_chrome() {
        let ua = "Mozilla/5.0 (Linux; Android 13) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36";
        let (device_type, device_name) = parse_user_agent(ua);

        assert_eq!(device_type, Some("mobile".to_string()));
        assert_eq!(device_name, Some("Chrome on Android".to_string()));
    }

    #[test]
    fn test_keycloak_session_deserialization() {
        let json = r#"{
            "id": "session-123",
            "username": "john",
            "userId": "user-456",
            "ipAddress": "192.168.1.1",
            "start": 1700000000000,
            "lastAccess": 1700001000000,
            "clients": {"client-1": "app1"}
        }"#;

        let session: KeycloakSession = serde_json::from_str(json).unwrap();
        assert_eq!(session.id, "session-123");
        assert_eq!(session.username, Some("john".to_string()));
        assert_eq!(session.ip_address, Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_keycloak_session_deserialization_minimal() {
        let json = r#"{"id": "session-123"}"#;

        let session: KeycloakSession = serde_json::from_str(json).unwrap();
        assert_eq!(session.id, "session-123");
        assert!(session.username.is_none());
        assert!(session.clients.is_empty());
    }

    #[test]
    fn test_session_info_serialization() {
        let info = SessionInfo {
            id: "session-123".to_string(),
            device_type: Some("desktop".to_string()),
            device_name: Some("Chrome".to_string()),
            ip_address: Some("192.168.1.1".to_string()),
            location: Some("San Francisco, US".to_string()),
            last_active_at: Utc::now(),
            created_at: Utc::now(),
            is_current: true,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("session-123"));
        assert!(json.contains("desktop"));
        assert!(json.contains("is_current"));
    }
}
