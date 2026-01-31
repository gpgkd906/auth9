# Auth9 测试改进计划

## 概述

本文档描述了如何提升 auth9-core 的测试覆盖率，重点针对：
- **API Handlers 层**: 当前 ~15%，目标 80%
- **gRPC token_exchange**: 当前 8.8%，目标 85%

## 测试原则 (必须遵守)

所有测试必须保持 **快速执行** (~1-2 秒完成全部测试)，**无外部依赖**：

| 组件 | 测试方法 | 禁止使用 |
|------|----------|---------|
| Repository 层 | `mockall` mock traits | ❌ 真实数据库 |
| Service 层 | mock repositories | ❌ testcontainers |
| API/gRPC 层 | `axum::test` + mock services | ❌ 真实服务器 |
| Keycloak | `wiremock` HTTP mocking | ❌ 真实 Keycloak |
| Redis 缓存 | `NoOpCacheManager` | ❌ 真实 Redis |
| 测试数据 | 直接构造 struct | ❌ faker 库 |

## 当前状态

| 模块 | 覆盖率 | 目标 | 差距 |
|------|--------|------|------|
| src/api/auth.rs | 13.2% | 80% | -66.8% |
| src/api/role.rs | 0% | 80% | -80% |
| src/api/tenant.rs | 0% | 80% | -80% |
| src/api/user.rs | 0% | 80% | -80% |
| src/api/service.rs | 18.8% | 80% | -61.2% |
| src/grpc/token_exchange.rs | 8.8% | 85% | -76.2% |

---

## Part 1: API Handlers 测试计划

### 1.1 测试架构

使用 `axum::test` 进行 handler 单元测试，不需要启动真实服务器：

```rust
// tests/api/mod.rs
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use auth9_core::api::create_router;

pub async fn test_app() -> Router {
    // 使用 mock repositories
    let tenant_repo = Arc::new(MockTenantRepository::new());
    let user_repo = Arc::new(MockUserRepository::new());
    // ... 其他 mocks

    create_router(tenant_repo, user_repo, /* ... */)
}
```

### 1.2 测试文件结构

```
auth9-core/tests/
├── api/
│   ├── mod.rs                    # 测试辅助函数和 mock 设置
│   ├── tenant_api_test.rs        # 租户 API 测试
│   ├── user_api_test.rs          # 用户 API 测试
│   ├── role_api_test.rs          # 角色 API 测试
│   ├── service_api_test.rs       # 服务 API 测试
│   ├── auth_api_test.rs          # 认证 API 测试
│   └── audit_api_test.rs         # 审计日志 API 测试
├── grpc/
│   └── token_exchange_test.rs    # gRPC 测试 (扩展)
└── common/
    └── mod.rs                    # 共享测试配置
```

### 1.3 租户 API 测试 (`tenant_api_test.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // ===== LIST TENANTS =====

    #[tokio::test]
    async fn test_list_tenants_success() {
        // Setup mock
        let mut mock = MockTenantRepository::new();
        mock.expect_list()
            .returning(|_, _| Ok((vec![create_test_tenant()], 1)));

        // Create app
        let app = create_test_app(mock);

        // Make request
        let response = app
            .oneshot(Request::get("/api/v1/tenants").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        // Verify response body
    }

    #[tokio::test]
    async fn test_list_tenants_pagination() {
        // 测试分页参数 page=2, per_page=10
    }

    #[tokio::test]
    async fn test_list_tenants_empty() {
        // 测试空列表返回
    }

    // ===== GET TENANT =====

    #[tokio::test]
    async fn test_get_tenant_success() {
        // 测试根据 ID 获取租户
    }

    #[tokio::test]
    async fn test_get_tenant_not_found() {
        // 测试租户不存在的情况
    }

    // ===== CREATE TENANT =====

    #[tokio::test]
    async fn test_create_tenant_success() {
        // 测试创建租户
    }

    #[tokio::test]
    async fn test_create_tenant_duplicate_slug() {
        // 测试 slug 重复
    }

    #[tokio::test]
    async fn test_create_tenant_invalid_input() {
        // 测试无效输入 (空名称等)
    }

    // ===== UPDATE TENANT =====

    #[tokio::test]
    async fn test_update_tenant_success() {
        // 测试更新租户
    }

    #[tokio::test]
    async fn test_update_tenant_not_found() {
        // 测试更新不存在的租户
    }

    // ===== DELETE TENANT =====

    #[tokio::test]
    async fn test_delete_tenant_success() {
        // 测试删除租户
    }

    #[tokio::test]
    async fn test_delete_tenant_not_found() {
        // 测试删除不存在的租户
    }
}
```

### 1.4 用户 API 测试 (`user_api_test.rs`)

| 测试用例 | 描述 |
|---------|------|
| `test_list_users_success` | 成功列出用户 |
| `test_list_users_by_tenant` | 按租户筛选用户 |
| `test_get_user_success` | 获取单个用户 |
| `test_get_user_not_found` | 用户不存在 |
| `test_create_user_success` | 创建用户（含 Keycloak mock） |
| `test_create_user_duplicate_email` | 邮箱已存在 |
| `test_update_user_success` | 更新用户 |
| `test_get_user_tenants` | 获取用户所属租户 |
| `test_add_user_to_tenant` | 添加用户到租户 |
| `test_remove_user_from_tenant` | 从租户移除用户 |

### 1.5 认证 API 测试 (`auth_api_test.rs`)

| 测试用例 | 描述 |
|---------|------|
| `test_authorize_redirect` | 授权重定向到 Keycloak |
| `test_authorize_with_state` | 带状态参数的授权 |
| `test_callback_success` | 回调处理成功 |
| `test_callback_invalid_code` | 无效授权码 |
| `test_callback_missing_params` | 缺少必要参数 |
| `test_logout_success` | 登出成功 |
| `test_token_refresh` | Token 刷新 |

### 1.6 角色/权限 API 测试 (`role_api_test.rs`)

| 测试用例 | 描述 |
|---------|------|
| `test_list_roles_by_service` | 按服务列出角色 |
| `test_create_role_success` | 创建角色 |
| `test_create_role_duplicate_name` | 角色名重复 |
| `test_update_role_success` | 更新角色 |
| `test_delete_role_success` | 删除角色 |
| `test_assign_roles_to_user` | 分配角色给用户 |
| `test_unassign_role_from_user` | 取消用户角色 |
| `test_get_user_roles_in_tenant` | 获取用户在租户的角色 |

---

## Part 2: gRPC Token Exchange 测试计划

### 2.1 当前测试文件扩展

文件位置: `tests/grpc_token_exchange_test.rs`

### 2.2 测试场景列表

```rust
mod token_exchange_tests {
    // ===== 正常流程 =====

    #[tokio::test]
    async fn test_exchange_with_valid_identity_token() {
        // 使用有效的身份令牌交换访问令牌
        // 验证返回的 TAT 包含正确的 claims
    }

    #[tokio::test]
    async fn test_exchange_includes_roles_and_permissions() {
        // 验证交换后的令牌包含用户角色和权限
    }

    #[tokio::test]
    async fn test_exchange_with_specific_tenant() {
        // 指定租户 ID 进行交换
    }

    // ===== 错误处理 =====

    #[tokio::test]
    async fn test_exchange_with_expired_token() {
        // 使用过期的身份令牌
        // 预期: InvalidToken 错误
    }

    #[tokio::test]
    async fn test_exchange_with_invalid_signature() {
        // 使用签名无效的令牌
        // 预期: InvalidToken 错误
    }

    #[tokio::test]
    async fn test_exchange_with_missing_tenant() {
        // 用户不属于指定租户
        // 预期: TenantNotFound 或 Unauthorized 错误
    }

    #[tokio::test]
    async fn test_exchange_with_disabled_user() {
        // 用户已被禁用
        // 预期: Unauthorized 错误
    }

    #[tokio::test]
    async fn test_exchange_with_malformed_token() {
        // 令牌格式错误
        // 预期: InvalidToken 错误
    }

    // ===== 缓存行为 (使用 MockCacheManager) =====

    #[tokio::test]
    async fn test_token_caching_enabled() {
        // 使用 MockCacheManager (内存实现，非真实 Redis)
        // 验证令牌被缓存到内存 HashMap
    }

    #[tokio::test]
    async fn test_cached_token_returned_on_repeat_request() {
        // MockCacheManager 验证重复请求返回缓存的令牌
    }

    #[tokio::test]
    async fn test_cache_invalidation_on_role_change() {
        // 角色变更后调用 cache.invalidate()
    }

    // ===== 边界条件 =====

    #[tokio::test]
    async fn test_exchange_with_user_having_no_roles() {
        // 用户没有任何角色
        // 验证返回空角色列表而非错误
    }

    #[tokio::test]
    async fn test_exchange_with_multiple_tenants() {
        // 用户属于多个租户，未指定租户 ID
        // 验证行为（选择默认或返回错误）
    }

    #[tokio::test]
    async fn test_exchange_with_inherited_roles() {
        // 测试角色继承
    }
}
```

### 2.3 测试数据设置 (直接构造，不使用 faker)

```rust
// tests/common/fixtures.rs
// 注意: 所有测试数据直接构造，不使用 faker 库

pub fn create_test_tenant() -> Tenant {
    Tenant {
        id: "tenant-test-001".to_string(),
        name: "Test Tenant".to_string(),
        slug: "test-tenant".to_string(),
        logo_url: None,
        settings: serde_json::json!({}),
        status: TenantStatus::Active,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

pub fn create_test_user() -> User {
    User {
        id: "user-test-001".to_string(),
        keycloak_id: Some("kc-user-001".to_string()),
        email: "test@example.com".to_string(),
        display_name: Some("Test User".to_string()),
        avatar_url: None,
        mfa_enabled: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

pub fn create_test_identity_token(
    user_id: &str,
    email: &str,
    expires_in: Duration,
) -> String {
    // 使用测试密钥签发 JWT (jsonwebtoken crate)
}

pub fn create_expired_token() -> String {
    create_test_identity_token("user-1", "test@example.com", Duration::seconds(-1))
}

pub fn create_invalid_signature_token() -> String {
    // 使用错误的密钥签名
}
```

---

## Part 3: 实施顺序

### Phase 1: 基础设施 (预计工作量: 2-3 小时)

1. 创建 `tests/api/mod.rs` 测试辅助模块
2. 设置共享 mock 工厂函数
3. 实现 `create_test_app()` 辅助函数

### Phase 2: 租户和用户 API (预计工作量: 3-4 小时)

1. `tenant_api_test.rs` - 12 个测试用例
2. `user_api_test.rs` - 10 个测试用例

### Phase 3: 角色和认证 API (预计工作量: 3-4 小时)

1. `role_api_test.rs` - 8 个测试用例
2. `auth_api_test.rs` - 7 个测试用例

### Phase 4: gRPC Token Exchange (预计工作量: 2-3 小时)

1. 扩展现有 `grpc_token_exchange_test.rs`
2. 添加 15+ 个新测试用例

---

## Part 4: 预期成果

完成后的覆盖率目标:

| 模块 | 当前 | 目标 | 新增测试数 |
|------|------|------|-----------|
| API Handlers | 15% | 80%+ | ~40 |
| gRPC Handlers | 8.8% | 85%+ | ~15 |
| **总体** | 31.91% | **55%+** | **~55** |

---

## 附录 A: 依赖确认

本计划 **不会** 引入以下昂贵的测试依赖：

```toml
# Cargo.toml [dev-dependencies] 中不会添加:
# ❌ testcontainers = "..."      # Docker 容器
# ❌ fake = "..."                 # faker 数据生成
# ❌ sqlx-test = "..."            # 数据库测试
# ❌ redis-test = "..."           # Redis 测试

# 仅使用现有的轻量级测试依赖:
# ✅ mockall                      # Trait mocking
# ✅ wiremock                     # HTTP mocking
# ✅ tokio-test                   # Async 测试工具
```

---

## 附录 B: Mock 模式参考

```rust
// 标准 mock 设置模式
#[tokio::test]
async fn test_example() {
    // 1. 创建 mock
    let mut mock_repo = MockTenantRepository::new();

    // 2. 设置期望
    mock_repo.expect_find_by_id()
        .with(eq("tenant-123"))
        .times(1)
        .returning(|_| Ok(Some(Tenant {
            id: "tenant-123".to_string(),
            name: "Test Tenant".to_string(),
            // ...
        })));

    // 3. 创建服务/app
    let service = TenantService::new(Arc::new(mock_repo), None);

    // 4. 执行测试
    let result = service.get("tenant-123").await;

    // 5. 断言
    assert!(result.is_ok());
    assert_eq!(result.unwrap().name, "Test Tenant");
}
```
