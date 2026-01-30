# Auth9 项目规则

## 测试策略

### 单元测试 (默认)
- 运行快速（< 1秒/测试）
- 使用 `mockall` 对 Repository traits 进行 mock
- 使用 `wiremock` 对 HTTP 外部依赖 (Keycloak) 进行 mock
- 覆盖所有业务逻辑
- 运行命令: `cargo test`

### Mock 使用规范
- Repository 层: 使用 `#[cfg_attr(test, mockall::automock)]` 注解 trait
- HTTP 依赖: 使用 WireMock 模拟外部 API
- Cache: 使用 `NoOpCacheManager` 或 `MockCacheOperations`

### 禁止事项
- 不使用 testcontainers
- 不依赖真实数据库或 Redis 运行测试
- 不使用 faker（直接构造测试数据）

## 项目结构

```
auth9-core/
├── src/
│   ├── api/          # HTTP API handlers
│   ├── cache/        # Redis cache layer (CacheManager, NoOpCacheManager)
│   ├── config/       # Configuration types
│   ├── domain/       # Domain models
│   ├── error/        # Error types
│   ├── grpc/         # gRPC service implementations
│   ├── jwt/          # JWT token management
│   ├── keycloak/     # Keycloak integration
│   ├── repository/   # Data access layer (with mock support)
│   ├── server/       # Server initialization
│   └── service/      # Business logic (with unit tests)
└── tests/
    ├── common/       # Shared test utilities
    └── grpc_*.rs     # gRPC service tests
```

## 测试文件位置

- **Service 层单元测试**: 放在 `src/service/*.rs` 的 `#[cfg(test)]` 模块中
- **Repository trait mock**: 放在 `src/repository/*.rs` 的 trait 定义上
- **gRPC 集成测试**: 放在 `tests/grpc_*.rs` 中

## 编码规范

### Repository 层
```rust
// 使用 mockall 自动生成 mock
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait TenantRepository: Send + Sync {
    async fn create(&self, input: &CreateTenantInput) -> Result<Tenant>;
    // ...
}
```

### Service 层测试
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::tenant::MockTenantRepository;

    #[tokio::test]
    async fn test_create_tenant_success() {
        let mut mock = MockTenantRepository::new();
        mock.expect_find_by_slug().returning(|_| Ok(None));
        mock.expect_create().returning(|input| Ok(Tenant { ... }));

        let service = TenantService::new(Arc::new(mock), None);
        let result = service.create(input).await;
        assert!(result.is_ok());
    }
}
```

### gRPC 测试
```rust
// 使用 NoOpCacheManager 代替真实 Redis
fn create_test_cache() -> NoOpCacheManager {
    NoOpCacheManager::new()
}

#[tokio::test]
async fn test_exchange_token() {
    let cache = create_test_cache();
    // ...
}
```
