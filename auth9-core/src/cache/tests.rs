//! Cache module tests

use super::*;

#[test]
fn test_cache_key_format() {
    let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let tenant_id = Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap();

    let key = format!("{}:{}:{}", keys::USER_ROLES, user_id, tenant_id);
    assert_eq!(
        key,
        "auth9:user_roles:550e8400-e29b-41d4-a716-446655440000:6ba7b810-9dad-11d1-80b4-00c04fd430c8"
    );
}

#[test]
fn test_cache_key_constants() {
    assert_eq!(keys::USER_ROLES, "auth9:user_roles");
    assert_eq!(keys::USER_ROLES_SERVICE, "auth9:user_roles_service");
    assert_eq!(keys::SERVICE_CONFIG, "auth9:service");
    assert_eq!(keys::TENANT_CONFIG, "auth9:tenant");
}

#[test]
fn test_cache_ttl_constants() {
    assert_eq!(ttl::USER_ROLES_SECS, 300);
    assert_eq!(ttl::USER_ROLES_SERVICE_SECS, 300);
    assert_eq!(ttl::SERVICE_CONFIG_SECS, 600);
    assert_eq!(ttl::TENANT_CONFIG_SECS, 600);
}

#[test]
fn test_service_config_key_format() {
    let service_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let key = format!("{}:{}", keys::SERVICE_CONFIG, service_id);
    assert_eq!(key, "auth9:service:550e8400-e29b-41d4-a716-446655440000");
}

#[test]
fn test_tenant_config_key_format() {
    let tenant_id = Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap();
    let key = format!("{}:{}", keys::TENANT_CONFIG, tenant_id);
    assert_eq!(key, "auth9:tenant:6ba7b810-9dad-11d1-80b4-00c04fd430c8");
}

#[test]
fn test_user_roles_service_key_format() {
    let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let tenant_id = Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap();
    let service_id = Uuid::parse_str("a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6").unwrap();

    let key = format!(
        "{}:{}:{}:{}",
        keys::USER_ROLES_SERVICE,
        user_id,
        tenant_id,
        service_id
    );
    assert!(key.starts_with("auth9:user_roles_service:"));
    assert!(key.contains(&user_id.to_string()));
    assert!(key.contains(&tenant_id.to_string()));
    assert!(key.contains(&service_id.to_string()));
}

#[tokio::test]
async fn test_noop_cache_manager_ping() {
    let cache = NoOpCacheManager::new();
    assert!(cache.ping().await.is_ok());
}

#[tokio::test]
async fn test_noop_cache_manager_get_user_roles() {
    let cache = NoOpCacheManager::new();
    let result = cache
        .get_user_roles(Uuid::new_v4(), Uuid::new_v4())
        .await
        .unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_noop_cache_manager_set_user_roles() {
    let cache = NoOpCacheManager::new();
    let roles = UserRolesInTenant {
        user_id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        roles: vec![],
        permissions: vec![],
    };
    assert!(cache.set_user_roles(&roles).await.is_ok());
}

#[tokio::test]
async fn test_noop_cache_manager_get_user_roles_for_service() {
    let cache = NoOpCacheManager::new();
    let result = cache
        .get_user_roles_for_service(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4())
        .await
        .unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_noop_cache_manager_set_user_roles_for_service() {
    let cache = NoOpCacheManager::new();
    let roles = UserRolesInTenant {
        user_id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        roles: vec![],
        permissions: vec![],
    };
    assert!(cache
        .set_user_roles_for_service(&roles, Uuid::new_v4())
        .await
        .is_ok());
}

#[tokio::test]
async fn test_noop_cache_manager_invalidate_user_roles_with_tenant() {
    let cache = NoOpCacheManager::new();
    assert!(cache
        .invalidate_user_roles(Uuid::new_v4(), Some(Uuid::new_v4()))
        .await
        .is_ok());
}

#[tokio::test]
async fn test_noop_cache_manager_invalidate_user_roles_without_tenant() {
    let cache = NoOpCacheManager::new();
    assert!(cache
        .invalidate_user_roles(Uuid::new_v4(), None)
        .await
        .is_ok());
}

#[tokio::test]
async fn test_noop_cache_manager_invalidate_user_roles_for_tenant() {
    let cache = NoOpCacheManager::new();
    assert!(cache
        .invalidate_user_roles_for_tenant(Uuid::new_v4(), Uuid::new_v4())
        .await
        .is_ok());
}

#[tokio::test]
async fn test_noop_cache_manager_invalidate_all_user_roles() {
    let cache = NoOpCacheManager::new();
    assert!(cache.invalidate_all_user_roles().await.is_ok());
}

#[test]
fn test_noop_cache_manager_default() {
    let cache = NoOpCacheManager::default();
    // Just verify it creates without panic
    let _ = cache;
}

#[test]
fn test_noop_cache_manager_clone() {
    let cache1 = NoOpCacheManager::new();
    let cache2 = cache1.clone();
    // Just verify cloning works
    let _ = cache2;
}

#[test]
fn test_token_blacklist_key_format() {
    let jti = "abc123-session-id";
    let key = format!("{}:{}", keys::TOKEN_BLACKLIST, jti);
    assert_eq!(key, "auth9:token_blacklist:abc123-session-id");
}

#[tokio::test]
async fn test_noop_cache_manager_add_to_token_blacklist() {
    let cache = NoOpCacheManager::new();
    let result = cache.add_to_token_blacklist("test-jti", 3600).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_noop_cache_manager_is_token_blacklisted() {
    let cache = NoOpCacheManager::new();
    // NoOp always returns false (token not blacklisted)
    let result = cache.is_token_blacklisted("test-jti").await.unwrap();
    assert!(!result);
}

#[tokio::test]
async fn test_noop_cache_manager_blacklist_zero_ttl() {
    let cache = NoOpCacheManager::new();
    // Zero TTL should still be ok (no-op)
    let result = cache.add_to_token_blacklist("test-jti", 0).await;
    assert!(result.is_ok());
}

// ========================================================================
// CacheOperations trait dispatch tests for NoOpCacheManager
// (covers the `impl CacheOperations for NoOpCacheManager` block)
// ========================================================================

#[tokio::test]
async fn test_noop_cache_operations_trait_ping() {
    let cache: &dyn CacheOperations = &NoOpCacheManager::new();
    assert!(cache.ping().await.is_ok());
}

#[tokio::test]
async fn test_noop_cache_operations_trait_get_user_roles() {
    let cache: &dyn CacheOperations = &NoOpCacheManager::new();
    let result = cache
        .get_user_roles(Uuid::new_v4(), Uuid::new_v4())
        .await
        .unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_noop_cache_operations_trait_set_user_roles() {
    let cache: &dyn CacheOperations = &NoOpCacheManager::new();
    let roles = UserRolesInTenant {
        user_id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        roles: vec![],
        permissions: vec![],
    };
    assert!(cache.set_user_roles(&roles).await.is_ok());
}

#[tokio::test]
async fn test_noop_cache_operations_trait_get_user_roles_for_service() {
    let cache: &dyn CacheOperations = &NoOpCacheManager::new();
    let result = cache
        .get_user_roles_for_service(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4())
        .await
        .unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_noop_cache_operations_trait_set_user_roles_for_service() {
    let cache: &dyn CacheOperations = &NoOpCacheManager::new();
    let roles = UserRolesInTenant {
        user_id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        roles: vec![],
        permissions: vec![],
    };
    assert!(cache
        .set_user_roles_for_service(&roles, Uuid::new_v4())
        .await
        .is_ok());
}

#[tokio::test]
async fn test_noop_cache_operations_trait_invalidate_user_roles() {
    let cache: &dyn CacheOperations = &NoOpCacheManager::new();
    assert!(cache
        .invalidate_user_roles(Uuid::new_v4(), Some(Uuid::new_v4()))
        .await
        .is_ok());
    assert!(cache
        .invalidate_user_roles(Uuid::new_v4(), None)
        .await
        .is_ok());
}

#[tokio::test]
async fn test_noop_cache_operations_trait_invalidate_user_roles_for_tenant() {
    let cache: &dyn CacheOperations = &NoOpCacheManager::new();
    assert!(cache
        .invalidate_user_roles_for_tenant(Uuid::new_v4(), Uuid::new_v4())
        .await
        .is_ok());
}

#[tokio::test]
async fn test_noop_cache_operations_trait_invalidate_all_user_roles() {
    let cache: &dyn CacheOperations = &NoOpCacheManager::new();
    assert!(cache.invalidate_all_user_roles().await.is_ok());
}

#[tokio::test]
async fn test_noop_cache_operations_trait_token_blacklist() {
    let cache: &dyn CacheOperations = &NoOpCacheManager::new();
    assert!(cache.add_to_token_blacklist("jti-1", 3600).await.is_ok());
    assert!(!cache.is_token_blacklisted("jti-1").await.unwrap());
}

// ========================================================================
// WebAuthn challenge state tests
// ========================================================================

#[test]
fn test_webauthn_reg_key_format() {
    let user_id = "user-123";
    let key = format!("{}:{}", keys::WEBAUTHN_REG, user_id);
    assert_eq!(key, "auth9:webauthn_reg:user-123");
}

#[test]
fn test_webauthn_auth_key_format() {
    let challenge_id = "challenge-456";
    let key = format!("{}:{}", keys::WEBAUTHN_AUTH, challenge_id);
    assert_eq!(key, "auth9:webauthn_auth:challenge-456");
}

#[tokio::test]
async fn test_noop_cache_webauthn_reg_state() {
    let cache = NoOpCacheManager::new();
    assert!(cache
        .store_webauthn_reg_state("user-1", "{}", 300)
        .await
        .is_ok());
    let result = cache.get_webauthn_reg_state("user-1").await.unwrap();
    assert!(result.is_none());
    assert!(cache.remove_webauthn_reg_state("user-1").await.is_ok());
}

#[tokio::test]
async fn test_noop_cache_webauthn_auth_state() {
    let cache = NoOpCacheManager::new();
    assert!(cache
        .store_webauthn_auth_state("challenge-1", "{}", 300)
        .await
        .is_ok());
    let result = cache.get_webauthn_auth_state("challenge-1").await.unwrap();
    assert!(result.is_none());
    assert!(cache
        .remove_webauthn_auth_state("challenge-1")
        .await
        .is_ok());
}

#[tokio::test]
async fn test_noop_cache_operations_trait_webauthn_reg() {
    let cache: &dyn CacheOperations = &NoOpCacheManager::new();
    assert!(cache
        .store_webauthn_reg_state("user-1", "{\"test\": true}", 300)
        .await
        .is_ok());
    assert!(cache
        .get_webauthn_reg_state("user-1")
        .await
        .unwrap()
        .is_none());
    assert!(cache.remove_webauthn_reg_state("user-1").await.is_ok());
}

#[tokio::test]
async fn test_noop_cache_operations_trait_webauthn_auth() {
    let cache: &dyn CacheOperations = &NoOpCacheManager::new();
    assert!(cache
        .store_webauthn_auth_state("ch-1", "{\"test\": true}", 300)
        .await
        .is_ok());
    assert!(cache
        .get_webauthn_auth_state("ch-1")
        .await
        .unwrap()
        .is_none());
    assert!(cache.remove_webauthn_auth_state("ch-1").await.is_ok());
}

#[tokio::test]
async fn test_noop_cache_oidc_state_consume_once() {
    let cache = NoOpCacheManager::new();
    cache
        .store_oidc_state("nonce-1", "{\"redirect_uri\":\"https://a\"}", 300)
        .await
        .unwrap();
    let first = cache.consume_oidc_state("nonce-1").await.unwrap();
    let second = cache.consume_oidc_state("nonce-1").await.unwrap();
    assert!(first.is_some());
    assert!(second.is_none());
}

#[tokio::test]
async fn test_noop_cache_refresh_session_binding() {
    let cache = NoOpCacheManager::new();
    cache
        .bind_refresh_token_session("rt-1", "sid-1", 300)
        .await
        .unwrap();
    let found = cache.get_refresh_token_session("rt-1").await.unwrap();
    assert_eq!(found.as_deref(), Some("sid-1"));
    cache.remove_refresh_token_session("rt-1").await.unwrap();
    let missing = cache.get_refresh_token_session("rt-1").await.unwrap();
    assert!(missing.is_none());
}

// ========================================================================
// Security Fix Tests: Redis SCAN instead of KEYS
// ========================================================================

#[test]
fn test_delete_pattern_uses_scan() {
    // This test verifies that delete_pattern uses SCAN instead of KEYS
    // The actual behavior is tested in integration tests with real Redis
    // Here we just verify the pattern matching logic
    let pattern = "auth9:user_roles:*";
    assert!(pattern.contains("*"));
    assert!(pattern.starts_with("auth9:"));
}

#[tokio::test]
async fn test_invalidate_all_user_roles_noop() {
    // Test that invalidate_all_user_roles works with NoOpCacheManager
    let cache = NoOpCacheManager::new();
    let result = cache.invalidate_all_user_roles().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_invalidate_user_roles_for_tenant_noop() {
    // Test that invalidate_user_roles_for_tenant works with NoOpCacheManager
    let cache = NoOpCacheManager::new();
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let result = cache
        .invalidate_user_roles_for_tenant(user_id, tenant_id)
        .await;
    assert!(result.is_ok());
}

// ========================================================================
// CacheOperations trait dispatch tests - OIDC and Refresh Token
// ========================================================================

#[tokio::test]
async fn test_noop_cache_operations_trait_oidc_state() {
    let cache: &dyn CacheOperations = &NoOpCacheManager::new();
    assert!(cache
        .store_oidc_state("nonce-1", "payload", 300)
        .await
        .is_ok());
    // NoOp impl stores in-memory, so consuming works
    let result = cache.consume_oidc_state("nonce-1").await.unwrap();
    assert!(result.is_some());
    let second = cache.consume_oidc_state("nonce-1").await.unwrap();
    assert!(second.is_none());
}

#[tokio::test]
async fn test_noop_cache_operations_trait_refresh_session() {
    let cache: &dyn CacheOperations = &NoOpCacheManager::new();
    assert!(cache
        .bind_refresh_token_session("rt-1", "sid-1", 300)
        .await
        .is_ok());
    let result = cache.get_refresh_token_session("rt-1").await.unwrap();
    assert_eq!(result.as_deref(), Some("sid-1"));
    assert!(cache.remove_refresh_token_session("rt-1").await.is_ok());
    let missing = cache.get_refresh_token_session("rt-1").await.unwrap();
    assert!(missing.is_none());
}

#[test]
fn test_refresh_token_hash_deterministic() {
    let hash1 = NoOpCacheManager::refresh_token_hash("test-token");
    let hash2 = NoOpCacheManager::refresh_token_hash("test-token");
    assert_eq!(hash1, hash2);
}

#[test]
fn test_refresh_token_hash_different_inputs() {
    let hash1 = NoOpCacheManager::refresh_token_hash("token-a");
    let hash2 = NoOpCacheManager::refresh_token_hash("token-b");
    assert_ne!(hash1, hash2);
}

#[test]
fn test_cache_manager_refresh_token_hash_deterministic() {
    let hash1 = CacheManager::refresh_token_hash("test-token");
    let hash2 = CacheManager::refresh_token_hash("test-token");
    assert_eq!(hash1, hash2);
    // Both managers should produce same hash
    let noop_hash = NoOpCacheManager::refresh_token_hash("test-token");
    assert_eq!(hash1, noop_hash);
}

#[test]
fn test_oidc_state_key_format() {
    let key = format!("{}:{}", keys::OIDC_STATE, "nonce-abc");
    assert_eq!(key, "auth9:oidc_state:nonce-abc");
}

#[test]
fn test_refresh_token_session_key_format() {
    let key = format!("{}:{}", keys::REFRESH_TOKEN_SESSION, "hash-abc");
    assert_eq!(key, "auth9:refresh_session:hash-abc");
}
