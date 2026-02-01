# Keycloak 服务层重构设计

## 1. 背景与目标

### 1.1 问题现状

当前 Keycloak 相关代码存在以下问题：

1. **单一大文件**: `src/keycloak/mod.rs` 达 2428 行，混合了多种职责
2. **职责不清**: Admin API 客户端、Seeder 初始化、类型定义混在一起
3. **缺少状态同步**: Auth9 配置变化时不会同步到 Keycloak realm
4. **OIDC 逻辑分散**: 认证流程代码分布在 `api/auth.rs` 和 `keycloak/mod.rs`

### 1.2 具体问题示例

**注册链接泄漏问题**：
- Auth9 的 `branding.allow_registration` 默认为 `false`
- Keycloak realm 的 `registrationAllowed` 被硬编码为 `true`
- 两者不同步，导致 Keycloak Theme 需要同时检查两个值
- 未自定义的 Theme 页面会泄漏注册链接

### 1.3 设计目标

1. **职责分离**: 将 Keycloak 相关功能拆分为独立的服务
2. **状态同步**: 实现 Auth9 ↔ Keycloak 配置自动同步
3. **可测试性**: 每个服务可独立 mock 测试
4. **可扩展性**: 便于未来添加更多同步配置项

## 2. 架构设计

### 2.1 服务划分

```
                        ┌─────────────────────┐
                        │   KeycloakClient    │
                        │   (Admin API 客户端) │
                        │   - HTTP 请求封装    │
                        │   - Token 缓存       │
                        └──────────┬──────────┘
                                   │
                    ┌──────────────┴──────────────┐
                    │                             │
           ┌────────▼────────┐          ┌────────▼────────┐
           │KeycloakOidcService│        │KeycloakSyncService│
           │                   │        │                   │
           │ OIDC 认证流程      │        │ 状态同步          │
           │ - authorize       │        │ - realm 配置      │
           │ - callback        │        │ - 注册开关        │
           │ - token_exchange  │        │ - 密码策略        │
           │ - logout          │        │ - (未来扩展)      │
           │ - userinfo        │        │                   │
           └───────────────────┘        └───────────────────┘
```

### 2.2 组件职责

| 组件 | 职责 | 位置 |
|------|------|------|
| **KeycloakClient** | 底层 Admin API HTTP 客户端 | `src/keycloak/client.rs` |
| **KeycloakOidcService** | OIDC 认证流程封装 | `src/service/keycloak_oidc.rs` |
| **KeycloakSyncService** | Auth9 ↔ Keycloak 状态同步 | `src/service/keycloak_sync.rs` |
| **KeycloakSeeder** | 初始化和数据种子 | `src/keycloak/seeder.rs` |

### 2.3 目标文件结构

```
src/keycloak/
├── mod.rs              # 模块导出
├── client.rs           # KeycloakClient (Admin API HTTP 客户端)
├── types.rs            # 共享类型定义 (KeycloakUser, KeycloakOidcClient 等)
└── seeder.rs           # KeycloakSeeder (初始化逻辑)

src/service/
├── mod.rs              # 添加新服务导出
├── keycloak_oidc.rs    # KeycloakOidcService (OIDC 流程)
├── keycloak_sync.rs    # KeycloakSyncService (状态同步)
├── branding.rs         # 修改: 调用 SyncService
└── ...
```

### 2.4 依赖关系

```
┌─────────────────────────────────────────────────────────────┐
│                        API Layer                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ api/auth.rs │  │api/branding │  │ api/session.rs      │  │
│  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘  │
└─────────┼────────────────┼────────────────────┼─────────────┘
          │                │                    │
          ▼                ▼                    ▼
┌─────────────────────────────────────────────────────────────┐
│                      Service Layer                           │
│  ┌──────────────────┐  ┌──────────────────┐                 │
│  │KeycloakOidcService│  │KeycloakSyncService│                │
│  └────────┬─────────┘  └────────┬─────────┘                 │
│           │                     │                            │
│           │    ┌────────────────┘                            │
│           │    │                                             │
│           ▼    ▼                                             │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              KeycloakClient (共享)                    │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## 3. 详细设计

### 3.1 KeycloakClient (底层客户端)

**职责**: 纯粹的 Admin API HTTP 封装，无业务逻辑

```rust
// src/keycloak/client.rs

pub struct KeycloakClient {
    config: KeycloakConfig,
    http_client: Client,
    token: Arc<RwLock<Option<AdminToken>>>,
}

impl KeycloakClient {
    // Token 管理
    pub async fn get_admin_token(&self) -> Result<String>;

    // 用户管理
    pub async fn create_user(&self, input: &CreateKeycloakUserInput) -> Result<String>;
    pub async fn get_user(&self, user_id: &str) -> Result<KeycloakUser>;
    pub async fn update_user(&self, user_id: &str, input: &KeycloakUserUpdate) -> Result<()>;
    pub async fn delete_user(&self, user_id: &str) -> Result<()>;
    pub async fn search_users_by_email(&self, email: &str) -> Result<Vec<KeycloakUser>>;

    // OIDC 客户端管理
    pub async fn create_oidc_client(&self, input: &CreateOidcClientInput) -> Result<String>;
    pub async fn get_client_secret(&self, client_uuid: &str) -> Result<String>;
    pub async fn update_oidc_client(&self, uuid: &str, input: &UpdateOidcClientInput) -> Result<()>;
    pub async fn delete_oidc_client(&self, uuid: &str) -> Result<()>;

    // 会话管理
    pub async fn get_user_sessions(&self, user_id: &str) -> Result<Vec<KeycloakSession>>;
    pub async fn delete_user_session(&self, session_id: &str) -> Result<()>;
    pub async fn logout_user(&self, user_id: &str) -> Result<()>;

    // 密码管理
    pub async fn reset_user_password(&self, user_id: &str, password: &str, temporary: bool) -> Result<()>;
    pub async fn validate_user_password(&self, user_id: &str, password: &str) -> Result<bool>;

    // Realm 管理 (新增)
    pub async fn get_realm(&self) -> Result<KeycloakRealm>;
    pub async fn update_realm(&self, update: &RealmUpdate) -> Result<()>;

    // 身份提供者管理
    pub async fn list_identity_providers(&self) -> Result<Vec<KeycloakIdentityProvider>>;
    pub async fn create_identity_provider(&self, input: &CreateIdpInput) -> Result<()>;
    pub async fn update_identity_provider(&self, alias: &str, input: &UpdateIdpInput) -> Result<()>;
    pub async fn delete_identity_provider(&self, alias: &str) -> Result<()>;
}
```

### 3.2 KeycloakSyncService (状态同步服务)

**职责**: 管理 Auth9 和 Keycloak 之间的配置同步

```rust
// src/service/keycloak_sync.rs

/// Realm 配置更新 (可扩展)
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RealmSettingsUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registration_allowed: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_password_allowed: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub remember_me: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub login_with_email_allowed: Option<bool>,

    // 未来扩展:
    // pub brute_force_protected: Option<bool>,
    // pub password_policy: Option<String>,
}

/// Keycloak 同步服务
pub struct KeycloakSyncService {
    keycloak: Arc<KeycloakClient>,
}

impl KeycloakSyncService {
    pub fn new(keycloak: Arc<KeycloakClient>) -> Self;

    /// 同步 Realm 配置到 Keycloak
    pub async fn sync_realm_settings(&self, settings: RealmSettingsUpdate) -> Result<()>;

    /// 从 BrandingConfig 提取需要同步的配置
    pub fn extract_realm_settings(config: &BrandingConfig) -> RealmSettingsUpdate;

    /// 获取当前 Keycloak Realm 配置
    pub async fn get_current_realm_settings(&self) -> Result<RealmSettingsUpdate>;
}
```

### 3.3 KeycloakOidcService (OIDC 服务)

**职责**: 封装 OIDC 认证流程，从 `api/auth.rs` 提取业务逻辑

```rust
// src/service/keycloak_oidc.rs

/// OIDC 授权请求参数
pub struct AuthorizeParams {
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub response_type: Option<String>,
}

/// OIDC 回调结果
pub struct CallbackResult {
    pub user_id: Uuid,
    pub email: String,
    pub identity_token: String,
    pub redirect_uri: String,
}

/// Keycloak OIDC 服务
pub struct KeycloakOidcService<U: UserRepository, S: ServiceRepository> {
    keycloak: Arc<KeycloakClient>,
    jwt_manager: Arc<JwtManager>,
    user_repo: Arc<U>,
    service_repo: Arc<S>,
    config: KeycloakConfig,
}

impl<U: UserRepository, S: ServiceRepository> KeycloakOidcService<U, S> {
    /// 构建 Keycloak 授权 URL
    pub fn build_authorize_url(&self, params: &AuthorizeParams) -> Result<String>;

    /// 处理 OIDC 回调
    pub async fn handle_callback(&self, code: &str, state: &str) -> Result<CallbackResult>;

    /// 交换授权码为 Token
    pub async fn exchange_code(&self, code: &str, redirect_uri: &str) -> Result<TokenResponse>;

    /// 刷新 Token
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<TokenResponse>;

    /// 获取用户信息
    pub async fn get_userinfo(&self, access_token: &str) -> Result<UserInfo>;

    /// 构建登出 URL
    pub fn build_logout_url(&self, id_token_hint: Option<&str>, post_logout_uri: Option<&str>) -> String;

    /// 验证 redirect_uri
    pub async fn validate_redirect_uri(&self, client_id: &str, redirect_uri: &str) -> Result<bool>;
}
```

### 3.4 BrandingService 修改

```rust
// src/service/branding.rs (修改)

pub struct BrandingService<R: SystemSettingsRepository> {
    repo: Arc<R>,
    sync_service: Option<Arc<KeycloakSyncService>>,  // 新增
}

impl<R: SystemSettingsRepository> BrandingService<R> {
    pub fn new(repo: Arc<R>, sync_service: Option<Arc<KeycloakSyncService>>) -> Self;

    pub async fn update_branding(&self, config: BrandingConfig) -> Result<BrandingConfig> {
        // 1. 验证配置
        self.validate_branding(&config)?;

        // 2. 保存到数据库
        // ... 现有逻辑 ...

        // 3. 同步到 Keycloak (新增)
        if let Some(sync) = &self.sync_service {
            let realm_settings = KeycloakSyncService::extract_realm_settings(&config);
            sync.sync_realm_settings(realm_settings).await?;
        }

        Ok(config)
    }
}
```

## 4. 实施计划

### Phase 1: 拆分 keycloak/mod.rs (预计 2-3 小时)

**目标**: 将大文件拆分为职责清晰的小模块

**任务清单**:

- [ ] 1.1 创建 `src/keycloak/types.rs`
  - 移动所有类型定义: `KeycloakUser`, `KeycloakOidcClient`, `KeycloakSession` 等
  - 移动输入/输出类型: `CreateKeycloakUserInput`, `KeycloakUserUpdate` 等

- [ ] 1.2 创建 `src/keycloak/client.rs`
  - 移动 `KeycloakClient` 结构体和实现
  - 移动 `AdminToken` 相关代码
  - 添加 `get_realm()` 和 `update_realm()` 方法

- [ ] 1.3 创建 `src/keycloak/seeder.rs`
  - 移动 `KeycloakSeeder` 结构体和实现
  - 移动初始化相关常量

- [ ] 1.4 更新 `src/keycloak/mod.rs`
  - 改为模块导出文件
  - 重新导出所有公开类型

- [ ] 1.5 修复所有编译错误
  - 更新 import 路径
  - 确保所有现有功能正常

**测试**:
- [ ] 运行 `cargo build` 确保编译通过
- [ ] 运行 `cargo test` 确保现有测试通过
- [ ] 运行 `tests/keycloak_unit_test.rs` 确保 Keycloak 测试通过

---

### Phase 2: 实现 KeycloakSyncService (预计 3-4 小时)

**目标**: 实现状态同步服务，解决注册链接问题

**任务清单**:

- [ ] 2.1 添加 Realm 管理方法到 KeycloakClient
  ```rust
  pub async fn get_realm(&self) -> Result<KeycloakRealm>;
  pub async fn update_realm(&self, update: &RealmUpdate) -> Result<()>;
  ```

- [ ] 2.2 创建 `src/service/keycloak_sync.rs`
  - 实现 `KeycloakSyncService`
  - 实现 `sync_realm_settings()` 方法
  - 实现 `extract_realm_settings()` 辅助方法

- [ ] 2.3 修改 `src/service/branding.rs`
  - 添加 `KeycloakSyncService` 依赖
  - 在 `update_branding()` 中调用同步

- [ ] 2.4 修改 AppState
  - 添加 `KeycloakSyncService` 到 AppState
  - 更新依赖注入

- [ ] 2.5 修改 KeycloakSeeder
  - 将 `registration_allowed` 默认值改为 `false`

- [ ] 2.6 简化 Keycloak Theme
  - 移除 `branding.allow_registration` 检查
  - 只依赖 `realm.registrationAllowed`

**测试**:
- [ ] 编写 `KeycloakSyncService` 单元测试
  - 使用 `MockKeycloakClient` 或 `wiremock`
  - 测试同步成功场景
  - 测试同步失败场景（网络错误、权限错误）
- [ ] 编写 `BrandingService` 集成测试
  - 验证更新 branding 时触发同步
- [ ] 手动测试 E2E 流程
  - Portal 更新 allow_registration → Keycloak realm 同步更新

---

### Phase 3: 实现 KeycloakOidcService (预计 4-5 小时)

**目标**: 将 OIDC 流程逻辑从 API 层提取到 Service 层

**任务清单**:

- [ ] 3.1 创建 `src/service/keycloak_oidc.rs`
  - 定义服务结构和接口

- [ ] 3.2 从 `api/auth.rs` 提取 OIDC 逻辑
  - `build_authorize_url()` - 构建授权 URL
  - `exchange_code_for_tokens()` - 授权码交换
  - `fetch_userinfo()` - 获取用户信息
  - `build_logout_url()` - 构建登出 URL
  - `validate_redirect_uri()` - 验证重定向 URI

- [ ] 3.3 实现 `handle_callback()`
  - 整合回调处理逻辑
  - 用户查找/创建
  - Identity Token 生成

- [ ] 3.4 修改 `api/auth.rs`
  - 注入 `KeycloakOidcService`
  - API handler 只做参数处理和响应构建
  - 业务逻辑委托给 Service

- [ ] 3.5 更新 AppState
  - 添加 `KeycloakOidcService` 到 AppState

**测试**:
- [ ] 编写 `KeycloakOidcService` 单元测试
  - Mock `KeycloakClient` 和 Repository
  - 测试 `build_authorize_url()` 参数构建
  - 测试 `handle_callback()` 各种场景
  - 测试 `validate_redirect_uri()` 验证逻辑
- [ ] 更新 `tests/api/http/auth_http_test.rs`
  - 确保现有 API 测试通过
- [ ] 运行完整 E2E 测试
  - `npm run test:e2e:full` 验证登录流程

---

### Phase 4: 清理和文档 (预计 1-2 小时)

**目标**: 代码清理、文档更新、最终验证

**任务清单**:

- [ ] 4.1 代码清理
  - 移除 `api/auth.rs` 中的重复逻辑
  - 统一错误处理
  - 添加必要的日志

- [ ] 4.2 更新文档
  - 更新 `CLAUDE.md` 中的模块说明
  - 更新 `docs/architecture.md` 架构图

- [ ] 4.3 添加 rustdoc 注释
  - `KeycloakClient` 公开方法
  - `KeycloakOidcService` 公开方法
  - `KeycloakSyncService` 公开方法

- [ ] 4.4 最终验证
  - 运行所有测试: `cargo test`
  - 运行 clippy: `cargo clippy`
  - 运行格式化: `cargo fmt`
  - 本地 E2E 测试完整流程

**测试**:
- [ ] `cargo test` 全部通过
- [ ] `cargo clippy` 无警告
- [ ] 本地完整登录流程测试
- [ ] Portal branding 设置同步测试

## 5. 测试策略

### 5.1 单元测试

| 模块 | 测试文件 | Mock 依赖 |
|------|----------|-----------|
| KeycloakClient | `src/keycloak/client.rs` (内置) | `wiremock` HTTP mock |
| KeycloakSyncService | `src/service/keycloak_sync.rs` (内置) | `MockKeycloakClient` |
| KeycloakOidcService | `src/service/keycloak_oidc.rs` (内置) | `MockKeycloakClient`, `MockUserRepository` |
| BrandingService | `src/service/branding.rs` (已有) | `MockSystemSettingsRepository`, `MockKeycloakSyncService` |

### 5.2 集成测试

| 测试文件 | 测试内容 |
|----------|----------|
| `tests/keycloak_unit_test.rs` | KeycloakClient API 调用 (wiremock) |
| `tests/api/http/auth_http_test.rs` | Auth API 端点 |
| `tests/api/http/branding_http_test.rs` | Branding API + 同步 |

### 5.3 E2E 测试

| 测试 | 命令 | 验证内容 |
|------|------|----------|
| 前端隔离 | `npm run test:e2e` | UI 渲染、导航 |
| 全栈集成 | `npm run test:e2e:full` | 完整登录流程、Keycloak 集成 |

### 5.4 Mock 设计

```rust
// KeycloakClient Mock (用于 Service 测试)
#[cfg(test)]
pub struct MockKeycloakClient {
    // 可配置的返回值
}

#[cfg(test)]
impl MockKeycloakClient {
    pub fn new() -> Self;
    pub fn expect_update_realm(&mut self) -> &mut Self;
    pub fn returning<F>(&mut self, f: F) -> &mut Self;
}

// 或使用 mockall
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait KeycloakClientTrait: Send + Sync {
    async fn update_realm(&self, update: &RealmUpdate) -> Result<()>;
    // ...
}
```

## 6. 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 重构导致现有功能回归 | 高 | 每个 Phase 后运行完整测试套件 |
| Keycloak API 兼容性 | 中 | 使用 wiremock 模拟真实响应 |
| 状态同步失败 | 中 | 添加重试机制和错误日志 |
| 性能影响 | 低 | 同步操作异步执行，不阻塞主流程 |

## 7. 时间估算

| Phase | 预计时间 | 依赖 |
|-------|----------|------|
| Phase 1: 拆分文件 | 2-3 小时 | 无 |
| Phase 2: SyncService | 3-4 小时 | Phase 1 |
| Phase 3: OidcService | 4-5 小时 | Phase 1 |
| Phase 4: 清理文档 | 1-2 小时 | Phase 2, 3 |
| **总计** | **10-14 小时** | |

## 8. 验收标准

### 8.1 功能验收

- [ ] Auth9 更新 `allow_registration` 时，Keycloak realm 同步更新
- [ ] Keycloak Theme 只检查 `realm.registrationAllowed`
- [ ] 所有 OIDC 流程正常工作（登录、登出、Token 交换）
- [ ] 未自定义的 Theme 页面不再泄漏注册链接

### 8.2 代码质量

- [ ] 所有测试通过 (`cargo test`)
- [ ] 无 clippy 警告 (`cargo clippy`)
- [ ] 代码格式化 (`cargo fmt`)
- [ ] 关键方法有 rustdoc 注释

### 8.3 文档完整

- [ ] 架构文档更新
- [ ] CLAUDE.md 模块说明更新
- [ ] 公开 API rustdoc 注释完整
