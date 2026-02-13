#[tokio::test]
async fn test_nodejs_require_blocked() {
    use crate::service::action_engine::{ActionEngine, MockActionRepository};
    use crate::domain::{Action, ActionContext, ActionTrigger, StringUuid};
    use std::sync::Arc;
    use chrono::Utc;
    
    let mock_repo = MockActionRepository::new();
    let engine = ActionEngine::new(Arc::new(mock_repo));
    
    // Try to use Node.js require()
    let action = Action {
        id: StringUuid::new(),
        tenant_id: StringUuid::new(),
        name: "Test Node.js require".to_string(),
        description: None,
        trigger_id: ActionTrigger::PostLogin,
        script: r#"
            try {
                const fs = require("fs");
                context.claims = context.claims || {};
                context.claims.leaked_data = "Node.js require worked!";
            } catch (e) {
                context.claims = context.claims || {};
                context.claims.blocked = true;
                context.claims.error = String(e);
            }
            context;
        "#.to_string(),
        enabled: true,
        execution_order: 0,
        timeout_ms: 3000,
        last_executed_at: None,
        execution_count: 0,
        error_count: 0,
        last_error: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    let context = ActionContext {
        user: crate::domain::ActionUserContext {
            id: StringUuid::new(),
            email: "test@example.com".to_string(),
            display_name: Some("Test User".to_string()),
            is_admin: false,
        },
        tenant: crate::domain::ActionTenantContext {
            id: StringUuid::new(),
            slug: "test".to_string(),
            name: "Test Tenant".to_string(),
        },
        claims: std::collections::HashMap::new(),
        trigger: ActionTrigger::PostLogin,
    };
    
    let result = engine.execute_action(&action, &context).await;
    
    // Should succeed because require() should not be defined
    assert!(result.is_ok(), "Node.js require() should be blocked");
    
    if let Ok(updated_context) = result {
        assert!(updated_context.claims.contains_key("blocked"), 
                "Should have blocked flag when require() fails");
        assert!(updated_context.claims.contains_key("error"),
                "Should have error message");
        let error = updated_context.claims.get("error").unwrap();
        assert!(error.contains("ReferenceError") || error.contains("require is not defined"),
                "Error should be ReferenceError: require is not defined");
    }
}
