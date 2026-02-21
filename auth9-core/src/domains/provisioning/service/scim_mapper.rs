//! SCIM attribute mapper
//!
//! Maps SCIM User attributes to Auth9 User fields using a hardcoded default mapping.

use crate::domain::scim::{MappedUserFields, ScimUser};

/// Extract Auth9 user fields from a SCIM User resource.
pub fn map_scim_user_to_fields(scim_user: &ScimUser) -> MappedUserFields {
    // Email: userName or primary email
    let mut email = Some(scim_user.user_name.clone());
    if email.as_deref() == Some("") {
        // Fallback to primary email
        email = scim_user
            .emails
            .iter()
            .find(|e| e.primary)
            .or(scim_user.emails.first())
            .map(|e| e.value.clone());
    }

    // Display name
    let display_name = scim_user.display_name.clone().or_else(|| {
        scim_user.name.as_ref().and_then(|n| {
            match (&n.given_name, &n.family_name) {
                (Some(given), Some(family)) => Some(format!("{} {}", given, family)),
                (Some(given), None) => Some(given.clone()),
                (None, Some(family)) => Some(family.clone()),
                (None, None) => n.formatted.clone(),
            }
        })
    });

    // Avatar URL from photos
    let avatar_url = scim_user
        .photos
        .iter()
        .find(|p| p.photo_type.as_deref() == Some("photo"))
        .or(scim_user.photos.first())
        .map(|p| p.value.clone());

    MappedUserFields {
        email,
        display_name,
        external_id: scim_user.external_id.clone(),
        active: Some(scim_user.active),
        avatar_url,
    }
}

/// Apply a SCIM patch value to extract partial updates.
/// Returns the mapped fields from a JSON value (used in PATCH operations).
pub fn map_patch_value_to_fields(
    path: Option<&str>,
    value: &serde_json::Value,
) -> MappedUserFields {
    let mut fields = MappedUserFields::default();

    match path {
        Some("userName") => {
            fields.email = value.as_str().map(|s| s.to_string());
        }
        Some("displayName") => {
            fields.display_name = value.as_str().map(|s| s.to_string());
        }
        Some("externalId") => {
            fields.external_id = value.as_str().map(|s| s.to_string());
        }
        Some("active") => {
            fields.active = value.as_bool();
        }
        Some("name.givenName") | Some("name.familyName") | Some("name") => {
            // For name updates, try to reconstruct display_name
            if let Some(obj) = value.as_object() {
                let given = obj
                    .get("givenName")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let family = obj
                    .get("familyName")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let name = format!("{} {}", given, family).trim().to_string();
                if !name.is_empty() {
                    fields.display_name = Some(name);
                }
            } else if let Some(s) = value.as_str() {
                fields.display_name = Some(s.to_string());
            }
        }
        Some("emails") => {
            // Array of emails - find primary
            if let Some(arr) = value.as_array() {
                fields.email = arr
                    .iter()
                    .find(|e| e.get("primary").and_then(|p| p.as_bool()).unwrap_or(false))
                    .or(arr.first())
                    .and_then(|e| e.get("value"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            }
        }
        Some("photos") => {
            if let Some(arr) = value.as_array() {
                fields.avatar_url = arr
                    .first()
                    .and_then(|p| p.get("value"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            }
        }
        None => {
            // No path - value is a full or partial user object
            if let Some(obj) = value.as_object() {
                if let Some(v) = obj.get("userName").and_then(|v| v.as_str()) {
                    fields.email = Some(v.to_string());
                }
                if let Some(v) = obj.get("displayName").and_then(|v| v.as_str()) {
                    fields.display_name = Some(v.to_string());
                }
                if let Some(v) = obj.get("externalId").and_then(|v| v.as_str()) {
                    fields.external_id = Some(v.to_string());
                }
                if let Some(v) = obj.get("active").and_then(|v| v.as_bool()) {
                    fields.active = Some(v);
                }
            }
        }
        _ => {}
    }

    fields
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::scim::{ScimEmail, ScimName, ScimPhoto, ScimUser};

    fn make_user(
        user_name: &str,
        display_name: Option<&str>,
        active: bool,
    ) -> ScimUser {
        ScimUser {
            schemas: vec![ScimUser::SCHEMA.to_string()],
            id: None,
            external_id: Some("ext-1".to_string()),
            user_name: user_name.to_string(),
            name: None,
            display_name: display_name.map(|s| s.to_string()),
            emails: vec![],
            photos: vec![],
            active,
            meta: None,
            groups: vec![],
        }
    }

    #[test]
    fn test_map_basic_user() {
        let user = make_user("john@example.com", Some("John Doe"), true);
        let fields = map_scim_user_to_fields(&user);
        assert_eq!(fields.email.as_deref(), Some("john@example.com"));
        assert_eq!(fields.display_name.as_deref(), Some("John Doe"));
        assert_eq!(fields.external_id.as_deref(), Some("ext-1"));
        assert_eq!(fields.active, Some(true));
    }

    #[test]
    fn test_map_display_name_from_name_parts() {
        let user = ScimUser {
            schemas: vec![ScimUser::SCHEMA.to_string()],
            id: None,
            external_id: None,
            user_name: "test@example.com".to_string(),
            name: Some(ScimName {
                given_name: Some("Jane".to_string()),
                family_name: Some("Smith".to_string()),
                formatted: None,
            }),
            display_name: None,
            emails: vec![],
            photos: vec![],
            active: true,
            meta: None,
            groups: vec![],
        };
        let fields = map_scim_user_to_fields(&user);
        assert_eq!(fields.display_name.as_deref(), Some("Jane Smith"));
    }

    #[test]
    fn test_map_display_name_from_formatted() {
        let user = ScimUser {
            schemas: vec![ScimUser::SCHEMA.to_string()],
            id: None,
            external_id: None,
            user_name: "test@example.com".to_string(),
            name: Some(ScimName {
                given_name: None,
                family_name: None,
                formatted: Some("Dr. John Smith".to_string()),
            }),
            display_name: None,
            emails: vec![],
            photos: vec![],
            active: true,
            meta: None,
            groups: vec![],
        };
        let fields = map_scim_user_to_fields(&user);
        assert_eq!(fields.display_name.as_deref(), Some("Dr. John Smith"));
    }

    #[test]
    fn test_map_email_fallback_to_primary_email() {
        let user = ScimUser {
            schemas: vec![ScimUser::SCHEMA.to_string()],
            id: None,
            external_id: None,
            user_name: "".to_string(), // empty userName
            name: None,
            display_name: None,
            emails: vec![
                ScimEmail {
                    value: "secondary@example.com".to_string(),
                    email_type: Some("home".to_string()),
                    primary: false,
                },
                ScimEmail {
                    value: "primary@example.com".to_string(),
                    email_type: Some("work".to_string()),
                    primary: true,
                },
            ],
            photos: vec![],
            active: true,
            meta: None,
            groups: vec![],
        };
        let fields = map_scim_user_to_fields(&user);
        assert_eq!(fields.email.as_deref(), Some("primary@example.com"));
    }

    #[test]
    fn test_map_avatar_from_photos() {
        let user = ScimUser {
            schemas: vec![ScimUser::SCHEMA.to_string()],
            id: None,
            external_id: None,
            user_name: "test@example.com".to_string(),
            name: None,
            display_name: None,
            emails: vec![],
            photos: vec![ScimPhoto {
                value: "https://cdn.example.com/avatar.png".to_string(),
                photo_type: Some("photo".to_string()),
            }],
            active: true,
            meta: None,
            groups: vec![],
        };
        let fields = map_scim_user_to_fields(&user);
        assert_eq!(
            fields.avatar_url.as_deref(),
            Some("https://cdn.example.com/avatar.png")
        );
    }

    #[test]
    fn test_map_inactive_user() {
        let user = make_user("test@example.com", None, false);
        let fields = map_scim_user_to_fields(&user);
        assert_eq!(fields.active, Some(false));
    }

    #[test]
    fn test_map_patch_username() {
        let value = serde_json::json!("new@example.com");
        let fields = map_patch_value_to_fields(Some("userName"), &value);
        assert_eq!(fields.email.as_deref(), Some("new@example.com"));
    }

    #[test]
    fn test_map_patch_display_name() {
        let value = serde_json::json!("New Name");
        let fields = map_patch_value_to_fields(Some("displayName"), &value);
        assert_eq!(fields.display_name.as_deref(), Some("New Name"));
    }

    #[test]
    fn test_map_patch_active() {
        let value = serde_json::json!(false);
        let fields = map_patch_value_to_fields(Some("active"), &value);
        assert_eq!(fields.active, Some(false));
    }

    #[test]
    fn test_map_patch_no_path() {
        let value = serde_json::json!({
            "userName": "patched@example.com",
            "displayName": "Patched User",
            "active": false
        });
        let fields = map_patch_value_to_fields(None, &value);
        assert_eq!(fields.email.as_deref(), Some("patched@example.com"));
        assert_eq!(fields.display_name.as_deref(), Some("Patched User"));
        assert_eq!(fields.active, Some(false));
    }

    #[test]
    fn test_map_patch_emails_array() {
        let value = serde_json::json!([
            {"value": "primary@example.com", "primary": true},
            {"value": "secondary@example.com", "primary": false}
        ]);
        let fields = map_patch_value_to_fields(Some("emails"), &value);
        assert_eq!(fields.email.as_deref(), Some("primary@example.com"));
    }

    #[test]
    fn test_map_patch_name_object() {
        let value = serde_json::json!({"givenName": "Alice", "familyName": "Wonder"});
        let fields = map_patch_value_to_fields(Some("name"), &value);
        assert_eq!(fields.display_name.as_deref(), Some("Alice Wonder"));
    }

    #[test]
    fn test_map_patch_unknown_path() {
        let value = serde_json::json!("something");
        let fields = map_patch_value_to_fields(Some("unknownPath"), &value);
        assert!(fields.email.is_none());
        assert!(fields.display_name.is_none());
        assert!(fields.active.is_none());
    }
}
