//! Common types for domain models

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Wrapper type for UUID stored as CHAR(36) in MySQL/TiDB
/// sqlx's uuid feature expects BINARY(16), but we use CHAR(36)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StringUuid(pub Uuid);

impl StringUuid {
    pub fn new_v4() -> Self {
        StringUuid(Uuid::new_v4())
    }

    pub fn nil() -> Self {
        StringUuid(Uuid::nil())
    }

    pub fn is_nil(&self) -> bool {
        self.0.is_nil()
    }

    /// Parse a UUID string
    pub fn parse_str(s: &str) -> Result<Self, uuid::Error> {
        Ok(StringUuid(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for StringUuid {
    fn from(uuid: Uuid) -> Self {
        StringUuid(uuid)
    }
}

impl From<StringUuid> for Uuid {
    fn from(s: StringUuid) -> Self {
        s.0
    }
}

impl std::ops::Deref for StringUuid {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for StringUuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for StringUuid {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(StringUuid(Uuid::parse_str(s)?))
    }
}

impl sqlx::Type<sqlx::MySql> for StringUuid {
    fn type_info() -> sqlx::mysql::MySqlTypeInfo {
        <String as sqlx::Type<sqlx::MySql>>::type_info()
    }

    fn compatible(ty: &sqlx::mysql::MySqlTypeInfo) -> bool {
        <String as sqlx::Type<sqlx::MySql>>::compatible(ty)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::MySql> for StringUuid {
    fn decode(value: sqlx::mysql::MySqlValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::MySql>>::decode(value)?;
        let uuid = Uuid::parse_str(&s)?;
        Ok(StringUuid(uuid))
    }
}

impl<'q> sqlx::Encode<'q, sqlx::MySql> for StringUuid {
    fn encode_by_ref(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        <String as sqlx::Encode<sqlx::MySql>>::encode_by_ref(&self.0.to_string(), buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_uuid_new() {
        let uuid = StringUuid::new_v4();
        assert!(!uuid.is_nil());
    }

    #[test]
    fn test_string_uuid_nil() {
        let uuid = StringUuid::nil();
        assert!(uuid.is_nil());
        assert_eq!(uuid.0, Uuid::nil());
    }

    #[test]
    fn test_string_uuid_is_nil() {
        let nil_uuid = StringUuid::nil();
        let valid_uuid = StringUuid::new_v4();

        assert!(nil_uuid.is_nil());
        assert!(!valid_uuid.is_nil());
    }

    #[test]
    fn test_string_uuid_from_str() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let uuid: StringUuid = uuid_str.parse().unwrap();
        assert_eq!(uuid.to_string(), uuid_str);
    }

    #[test]
    fn test_string_uuid_from_str_invalid() {
        let invalid = "not-a-uuid";
        let result: Result<StringUuid, _> = invalid.parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_string_uuid_conversion() {
        let uuid = Uuid::new_v4();
        let string_uuid: StringUuid = uuid.into();
        let back: Uuid = string_uuid.into();
        assert_eq!(uuid, back);
    }

    #[test]
    fn test_string_uuid_deref() {
        let uuid = Uuid::new_v4();
        let string_uuid = StringUuid(uuid);

        // Test deref - should be able to call Uuid methods directly
        assert_eq!(*string_uuid, uuid);
        assert_eq!(string_uuid.as_bytes(), uuid.as_bytes());
    }

    #[test]
    fn test_string_uuid_display() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let uuid: StringUuid = uuid_str.parse().unwrap();

        // Test Display trait
        assert_eq!(format!("{}", uuid), uuid_str);
    }

    #[test]
    fn test_string_uuid_equality() {
        let uuid1 = StringUuid::new_v4();
        let uuid2 = uuid1;
        let uuid3 = StringUuid::new_v4();

        assert_eq!(uuid1, uuid2);
        assert_ne!(uuid1, uuid3);
    }

    #[test]
    fn test_string_uuid_hash() {
        use std::collections::HashSet;

        let uuid1 = StringUuid::new_v4();
        let uuid2 = StringUuid::new_v4();

        let mut set = HashSet::new();
        set.insert(uuid1);
        set.insert(uuid2);
        set.insert(uuid1); // Duplicate

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_string_uuid_serialization() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let uuid: StringUuid = uuid_str.parse().unwrap();

        // Test serde serialization
        let json = serde_json::to_string(&uuid).unwrap();
        assert_eq!(json, format!("\"{}\"", uuid_str));

        // Test deserialization
        let deserialized: StringUuid = serde_json::from_str(&json).unwrap();
        assert_eq!(uuid, deserialized);
    }

    #[test]
    fn test_string_uuid_copy() {
        let uuid1 = StringUuid::new_v4();
        let uuid2 = uuid1; // Copy

        // Both should be usable (Copy trait)
        assert_eq!(uuid1, uuid2);
        assert!(!uuid1.is_nil());
        assert!(!uuid2.is_nil());
    }
}
