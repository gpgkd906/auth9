#[cfg(test)]
mod tests {
    use auth9_core::domain::UpdateUserInput;
    use serde_json::json;

    #[test]
    fn test_update_user_with_mfa_enabled_bool() {
        let json = json!({
            "display_name": "Test",
            "mfa_enabled": true
        });

        let input: Result<UpdateUserInput, _> = serde_json::from_value(json);
        match input {
            Ok(_) => println!("✓ mfa_enabled as bool (true) is accepted by serde"),
            Err(e) => println!("✗ mfa_enabled as bool (true) error: {}", e),
        }
    }

    #[test]
    fn test_update_user_with_mfa_enabled_string() {
        let json = json!({
            "display_name": "Test",
            "mfa_enabled": "true"
        });

        let input: Result<UpdateUserInput, _> = serde_json::from_value(json);
        match input {
            Ok(_) => println!("✓ mfa_enabled as string (\"true\") is accepted by serde"),
            Err(e) => println!("✗ mfa_enabled as string (\"true\") error: {}", e),
        }
    }

    #[test]
    fn test_update_user_only_mfa_enabled() {
        let json = json!({"mfa_enabled": true});

        let input: Result<UpdateUserInput, _> = serde_json::from_value(json);
        match input {
            Ok(_) => println!("✓ Only mfa_enabled field is accepted"),
            Err(e) => println!("✗ Only mfa_enabled field error: {}", e),
        }
    }
}
