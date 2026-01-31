# Auth9 API 访问控制分类清单

> **文档版本**: v1.0  
> **创建日期**: 2026-01-31  
> **状态**: 待开发组审查  
> **审查重点**: 确认公开/认证端点分类、gRPC 安全加固方案

---

## 📋 执行摘要

当前 Auth9 项目包含 **69 个 REST API 端点** 和 **4 个 gRPC 方法**。

**关键发现**:
- ✅ REST API: 11 个公开端点、58 个需认证端点
- ❌ **gRPC API: 全部 4 个方法无任何认证保护** (P0 安全风险)
- ⚠️ REST API 缺少统一认证中间件，依赖手动提取 JWT

**建议优先级**:
1. **P0 (紧急)**: 为 gRPC 添加 mTLS 或 API Key 认证
2. **P0 (紧急)**: 实现 REST API 统一认证中间件
3. **P1 (高)**: 实现 Rate Limiting 和权限级别验证
4. **P2 (中)**: CORS 白名单、审计日志增强

---

## 🌐 REST API 分类

### ✅ PUBLIC - 无需认证的端点 (11 个)

#### 1. 健康检查 (2 个)

| 端点 | 方法 | 用途 | 公开原因 |
|------|------|------|----------|
| `/health` | GET | 基础健康检查 | K8s liveness probe 必须 |
| `/ready` | GET | 就绪检查 (DB/Redis) | K8s readiness probe 必须 |

**建议**: 保持公开，但考虑限制访问频率 (1000 req/min/IP)

---

#### 2. OIDC 标准端点 (4 个)

| 端点 | 方法 | 用途 | 公开原因 |
|------|------|------|----------|
| `/.well-known/openid-configuration` | GET | OIDC 元数据发现 | **RFC 8414 标准要求公开** |
| `/.well-known/jwks.json` | GET | JWT 公钥集 (JWK Set) | 客户端验证 JWT 签名必须 |
| `/api/v1/auth/authorize` | GET | OIDC 授权入口 | 用户登录流程起点 |
| `/api/v1/auth/callback` | GET | OIDC 回调处理 | Keycloak 重定向回调 |

**安全措施**:
- `authorize`: 验证 `client_id` 和 `redirect_uri` 白名单
- `callback`: 验证 `state` 参数防 CSRF

---

#### 3. 认证相关端点 (3 个)

| 端点 | 方法 | 认证方式 | 说明 |
|------|------|----------|------|
| `/api/v1/auth/token` | POST | Client Secret | 使用 `client_id` + `client_secret` 换取 Token |
| `/api/v1/auth/logout` | GET | Session Cookie | 携带 session 注销登录 |
| `/api/v1/auth/userinfo` | GET | Bearer Token | **需要有效 JWT**，但端点本身无认证层 |

**注意**: 
- `token` 端点通过 **Client Secret** 验证，但仍属于"公开"（无需用户认证）
- `userinfo` 实际需要 JWT，应归类为"半公开"

---

#### 4. 特殊公开端点 (2 个)

| 端点 | 方法 | 用途 | 公开原因 |
|------|------|------|----------|
| `/api/v1/public/branding` | GET | 获取品牌配置 (logo/颜色) | 🎨 **Keycloak 登录页主题需要** |
| `/api/v1/invitations/accept` | POST | 接受邀请 | 📧 邮件链接访问，使用一次性加密 token |

**安全措施**:
- `branding`: 只返回视觉配置，不含敏感信息
- `invitations/accept`: 
  - Token 格式: `argon2` 哈希存储
  - 过期时间: 可配置 (默认 7 天)
  - 一次性使用: 接受后立即失效

---

### 🔒 AUTHENTICATED - 需要 JWT 认证的端点 (58 个)

#### 1. 租户管理 (5 个)

| 端点 | 方法 | 最低权限要求 | 说明 |
|------|------|-------------|------|
| `GET /api/v1/tenants` | GET | `platform_admin` | 列出所有租户 |
| `POST /api/v1/tenants` | POST | `platform_admin` | 创建租户 |
| `GET /api/v1/tenants/:id` | GET | `tenant_member` | 查看自己租户的详情 |
| `PUT /api/v1/tenants/:id` | PUT | `tenant_owner` | 更新租户配置 (名称/logo/设置) |
| `DELETE /api/v1/tenants/:id` | DELETE | `platform_admin` | 删除租户 (软删除/suspend) |

**权限逻辑**:
```rust
// 伪代码
fn check_tenant_access(jwt: &Claims, tenant_id: Uuid, action: Action) -> Result<()> {
    if jwt.is_platform_admin() {
        return Ok(());
    }
    
    if !jwt.tenant_ids.contains(&tenant_id) {
        return Err(Forbidden("Not a member of this tenant"));
    }
    
    match action {
        Action::Read => Ok(()),
        Action::Update => {
            if jwt.is_tenant_owner(tenant_id) {
                Ok(())
            } else {
                Err(Forbidden("Owner role required"))
            }
        }
    }
}
```

---

#### 2. 用户管理 (11 个)

| 端点 | 方法 | 最低权限要求 | 特殊规则 |
|------|------|-------------|----------|
| `GET /api/v1/users` | GET | `tenant_admin` | 租户隔离 |
| `POST /api/v1/users` | POST | `tenant_admin` | - |
| `GET /api/v1/users/:id` | GET | `tenant_member` | ✅ 可查看自己 (`jwt.sub == id`) |
| `PUT /api/v1/users/:id` | PUT | `tenant_member` | ✅ 可修改自己 |
| `DELETE /api/v1/users/:id` | DELETE | `tenant_admin` | ❌ 不能删除自己 |
| `POST /api/v1/users/:id/mfa` | POST | `tenant_member` | ✅ 可启用自己的 MFA |
| `DELETE /api/v1/users/:id/mfa` | DELETE | `tenant_member` | ⚠️ 管理员禁用他人 MFA 需二次验证 |
| `GET /api/v1/users/:id/tenants` | GET | `tenant_member` | ✅ 可查看自己 |
| `POST /api/v1/users/:id/tenants` | POST | `tenant_admin` | 添加用户到租户 |
| `DELETE /api/v1/users/:user_id/tenants/:tenant_id` | DELETE | `tenant_admin` | 从租户移除用户 |
| `GET /api/v1/tenants/:tenant_id/users` | GET | `tenant_member` | 列出租户成员 |

**自我访问规则**:
```rust
fn allow_self_access(jwt: &Claims, user_id: Uuid) -> bool {
    jwt.sub == user_id.to_string()
}
```

---

#### 3. 服务/客户端管理 (9 个)

| 端点 | 方法 | 最低权限要求 | 风险等级 |
|------|------|-------------|----------|
| `GET /api/v1/services` | GET | `tenant_member` | 🟢 低 |
| `POST /api/v1/services` | POST | `tenant_admin` | 🟡 中 |
| `GET /api/v1/services/:id` | GET | `tenant_member` | 🟢 低 |
| `PUT /api/v1/services/:id` | PUT | `tenant_admin` | 🟡 中 |
| `DELETE /api/v1/services/:id` | DELETE | `tenant_admin` | 🟠 高 |
| `GET /api/v1/services/:id/clients` | GET | `tenant_member` | 🟢 低 |
| `POST /api/v1/services/:id/clients` | POST | `tenant_admin` | 🟡 中 |
| `DELETE /api/v1/services/:service_id/clients/:client_id` | DELETE | `tenant_admin` | 🟠 高 |
| `POST /api/v1/services/:service_id/clients/:client_id/regenerate-secret` | POST | `tenant_admin` | 🔴 **极高** |

**高风险操作审计**:
- `regenerate-secret`: 
  - 会导致旧 secret 立即失效
  - 必须记录审计日志 (操作人、时间、client_id)
  - 建议: 二次验证 (输入旧 secret 或 OTP)

---

#### 4. 权限点管理 (3 个)

| 端点 | 方法 | 最低权限要求 |
|------|------|-------------|
| `POST /api/v1/permissions` | POST | `service_owner` |
| `DELETE /api/v1/permissions/:id` | DELETE | `service_owner` |
| `GET /api/v1/services/:service_id/permissions` | GET | `tenant_member` |

**权限设计**:
- Permission 属于 Service
- 只有 Service 创建者可以管理其权限点

---

#### 5. 角色管理 (8 个)

| 端点 | 方法 | 最低权限要求 |
|------|------|-------------|
| `POST /api/v1/roles` | POST | `tenant_admin` |
| `GET /api/v1/roles/:id` | GET | `tenant_member` |
| `PUT /api/v1/roles/:id` | PUT | `tenant_admin` |
| `DELETE /api/v1/roles/:id` | DELETE | `tenant_admin` |
| `GET /api/v1/services/:service_id/roles` | GET | `tenant_member` |
| `POST /api/v1/roles/:role_id/permissions` | POST | `tenant_admin` |
| `DELETE /api/v1/roles/:role_id/permissions/:permission_id` | DELETE | `tenant_admin` |

---

#### 6. RBAC 分配 (4 个)

| 端点 | 方法 | 最低权限要求 | 说明 |
|------|------|-------------|------|
| `POST /api/v1/rbac/assign` | POST | `tenant_admin` | 批量分配角色给用户 |
| `GET /api/v1/users/:user_id/tenants/:tenant_id/roles` | GET | `tenant_member` | 查询用户角色 (含继承) |
| `GET /api/v1/users/:user_id/tenants/:tenant_id/assigned-roles` | GET | `tenant_member` | 查询直接分配的角色 |
| `DELETE /api/v1/users/:user_id/tenants/:tenant_id/roles/:role_id` | DELETE | `tenant_admin` | 取消角色分配 |

---

#### 7. 审计日志 (1 个)

| 端点 | 方法 | 最低权限要求 | 建议改进 |
|------|------|-------------|----------|
| `GET /api/v1/audit-logs` | GET | `platform_admin` 或 `audit_viewer` | ⚠️ 添加租户过滤参数 |

**当前问题**:
- 返回全局审计日志，无租户隔离
- 建议: 
  - 平台管理员: 查看所有日志
  - 租户管理员: 仅查看本租户日志 (`?tenant_id=xxx`)
  - 审计员: 只读权限

---

#### 8. 系统设置 (6 个) - 超级管理员专属

| 端点 | 方法 | 权限要求 | 风险等级 |
|------|------|---------|----------|
| `GET /api/v1/system/email` | GET | `super_admin` | 🟡 中 (返回脱敏配置) |
| `PUT /api/v1/system/email` | PUT | `super_admin` | 🔴 **极高** (包含 SMTP 密码) |
| `POST /api/v1/system/email/test` | POST | `super_admin` | 🟢 低 |
| `POST /api/v1/system/email/send-test` | POST | `super_admin` | 🟢 低 |
| `GET /api/v1/system/branding` | GET | `admin` | 🟢 低 |
| `PUT /api/v1/system/branding` | PUT | `admin` | 🟡 中 |

**敏感信息处理**:
```rust
// GET 响应时脱敏
{
  "type": "smtp",
  "host": "smtp.example.com",
  "username": "user@example.com",
  "password": "***"  // 脱敏
}

// PUT 请求时加密存储
fn update_email_settings(config: EmailConfig) {
    let encrypted_password = aes_gcm_encrypt(config.password);
    db.save(encrypted_password);
}
```

---

#### 9. 邮件模板管理 (6 个)

| 端点 | 方法 | 权限要求 |
|------|------|---------|
| `GET /api/v1/system/email-templates` | GET | `admin` |
| `GET /api/v1/system/email-templates/:type` | GET | `admin` |
| `PUT /api/v1/system/email-templates/:type` | PUT | `admin` |
| `DELETE /api/v1/system/email-templates/:type` | DELETE | `admin` |
| `POST /api/v1/system/email-templates/:type/preview` | POST | `admin` |
| `POST /api/v1/system/email-templates/:type/send-test` | POST | `admin` |

**模板类型**:
- `invitation` - 邀请邮件
- `password_reset` - 密码重置
- `email_mfa` - MFA 验证码
- `welcome` - 欢迎邮件
- `email_verification` - 邮箱验证
- `password_changed` - 密码已更改通知
- `security_alert` - 安全警报

---

#### 10. 邀请管理 (5 个)

| 端点 | 方法 | 权限要求 |
|------|------|---------|
| `GET /api/v1/tenants/:tenant_id/invitations` | GET | `tenant_admin` |
| `POST /api/v1/tenants/:tenant_id/invitations` | POST | `tenant_admin` |
| `GET /api/v1/invitations/:id` | GET | `tenant_admin` |
| `DELETE /api/v1/invitations/:id` | DELETE | `tenant_admin` |
| `POST /api/v1/invitations/:id/revoke` | POST | `tenant_admin` |
| `POST /api/v1/invitations/:id/resend` | POST | `tenant_admin` |

**注意**: 
- `POST /api/v1/invitations/accept` 是公开端点 (已在前面列出)
- 创建邀请时从 JWT 提取 `invited_by`

---

## 🔌 gRPC API 分类

### ❌ 当前状态: 全部无认证 (P0 安全风险)

| RPC 方法 | 端口 | 用途 | 当前认证 | 风险等级 |
|---------|------|------|----------|----------|
| `ExchangeToken` | 50051 | Identity Token → Tenant Access Token | ❌ 无 | 🔴 **极高** |
| `ValidateToken` | 50051 | 验证 Access Token 有效性 | ❌ 无 | 🟡 中 |
| `GetUserRoles` | 50051 | 查询用户角色和权限 | ❌ 无 | 🟠 高 |
| `IntrospectToken` | 50051 | Token 内省 (调试用) | ❌ 无 | 🟠 高 |

---

### 🚨 风险详细分析

#### 1. ExchangeToken - 🔴 极高风险

**当前实现**:
```rust
async fn exchange_token(
    &self,
    request: Request<ExchangeTokenRequest>,
) -> Result<Response<ExchangeTokenResponse>, Status> {
    let req = request.into_inner();
    
    // ❌ 无调用方认证
    // ❌ 任何内网服务可调用
    
    // 仅验证用户的 identity_token
    let claims = self.jwt_manager.verify_identity_token(&req.identity_token)?;
    
    // 生成 Tenant Access Token
    let access_token = self.jwt_manager.create_tenant_access_token(...)?;
    
    Ok(Response::new(ExchangeTokenResponse { access_token, ... }))
}
```

**攻击场景**:
1. 恶意内网服务伪造 `identity_token` (如果知道 JWT secret)
2. 重放攻击: 窃取合法 `identity_token` 后重复调用
3. 权限提升: 请求不属于自己的 `tenant_id` 和 `service_id`

**影响范围**:
- 可冒充任意用户获取 Tenant Access Token
- 绕过所有业务层权限检查
- 数据泄露、越权操作

---

#### 2. ValidateToken - 🟡 中风险

**风险**:
- 可用于 Token 扫描攻击
- 枚举有效 Token 列表
- 无调用频率限制

**建议**: Rate Limiting (1000 req/min/client)

---

#### 3. GetUserRoles - 🟠 高风险

**风险**:
- 枚举用户权限信息
- 隐私泄露
- 辅助权限提升攻击

**建议**: 
- 验证调用方身份
- 检查调用方是否有权查询目标用户

---

#### 4. IntrospectToken - 🟠 高风险

**风险**:
- 暴露 Token 内部结构 (roles, permissions)
- 辅助攻击者理解权限模型
- 生产环境应禁用或严格限制

**建议**: 
- 仅在开发/调试环境开放
- 生产环境通过 Feature Flag 禁用

---

## 🛡️ gRPC 认证方案对比

### 方案 1: mTLS (推荐)

**优点**:
- ⭐⭐⭐⭐⭐ 安全性最高 (双向证书验证)
- ⭐⭐⭐⭐ 性能优秀 (TLS 加速硬件)
- ✅ Kubernetes 原生支持 (cert-manager)
- ✅ 自动证书轮换

**缺点**:
- ⭐⭐⭐ 配置复杂度较高
- 需要 CA 证书管理

**实现示例**:
```rust
use tonic::transport::{Server, ServerTlsConfig, Identity};

// 加载证书
let server_cert = std::fs::read("server-cert.pem")?;
let server_key = std::fs::read("server-key.pem")?;
let ca_cert = std::fs::read("ca-cert.pem")?;

let server_identity = Identity::from_pem(server_cert, server_key);

// 配置 mTLS
let tls_config = ServerTlsConfig::new()
    .identity(server_identity)
    .client_ca_root(Certificate::from_pem(ca_cert));  // 验证客户端证书

Server::builder()
    .tls_config(tls_config)?
    .add_service(TokenExchangeServer::new(service))
    .serve(addr)
    .await?;
```

**Kubernetes 配置**:
```yaml
# 使用 cert-manager 自动签发证书
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: auth9-core-grpc-tls
spec:
  secretName: auth9-core-grpc-tls
  issuer:
    name: internal-ca
  dnsNames:
    - auth9-core.auth9.svc.cluster.local
  usages:
    - server auth
    - client auth
```

---

### 方案 2: API Key (Interceptor)

**优点**:
- ⭐⭐⭐⭐⭐ 实现简单
- ⭐⭐⭐⭐⭐ 性能最佳
- ✅ 快速集成

**缺点**:
- ⭐⭐⭐ 安全性中等
- ⚠️ 需要安全存储 API Key
- ⚠️ Key 轮换需手动处理

**实现示例**:
```rust
use tonic::{Request, Status, service::Interceptor};

#[derive(Clone)]
struct ApiKeyInterceptor {
    valid_keys: Arc<HashSet<String>>,
}

impl Interceptor for ApiKeyInterceptor {
    fn call(&mut self, request: Request<()>) -> Result<Request<()>, Status> {
        let api_key = request
            .metadata()
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| Status::unauthenticated("Missing API Key"))?;
        
        if !self.valid_keys.contains(api_key) {
            return Err(Status::unauthenticated("Invalid API Key"));
        }
        
        Ok(request)
    }
}

// 使用
Server::builder()
    .add_service(
        TokenExchangeServer::with_interceptor(service, interceptor)
    )
    .serve(addr)
    .await?;
```

**客户端调用**:
```rust
let channel = Channel::from_static("http://auth9-core:50051")
    .connect()
    .await?;

let mut client = TokenExchangeClient::with_interceptor(
    channel,
    |mut req: Request<()>| {
        req.metadata_mut().insert(
            "x-api-key",
            "secret-api-key-12345".parse().unwrap(),
        );
        Ok(req)
    },
);
```

---

### 方案 3: JWT Token

**优点**:
- ⭐⭐⭐⭐ 安全性较好
- ✅ 可携带调用方身份信息
- ✅ 支持过期时间

**缺点**:
- ⭐⭐⭐ 性能较低 (每次验证签名)
- ⚠️ 需要 Token 刷新机制
- ⚠️ 增加客户端复杂度

**实现示例**:
```rust
impl Interceptor for JwtInterceptor {
    fn call(&mut self, request: Request<()>) -> Result<Request<()>, Status> {
        let token = request
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .ok_or_else(|| Status::unauthenticated("Missing token"))?;
        
        // 验证 JWT
        let claims = self.jwt_manager
            .verify_service_token(token)
            .map_err(|_| Status::unauthenticated("Invalid token"))?;
        
        // 可选: 将 claims 注入到 request extensions
        Ok(request)
    }
}
```

---

### 🎯 推荐选择

| 部署环境 | 推荐方案 | 理由 |
|---------|---------|------|
| **Kubernetes (生产)** | mTLS | 安全性最高，cert-manager 自动化管理 |
| **Docker Compose (开发)** | API Key | 实现简单，快速启动 |
| **混合云** | JWT Token | 跨网络灵活性好 |

---

## 🛡️ 安全加固优先级

### 🚨 P0 - 立即修复 (1-2 天)

#### 1. gRPC 添加认证

**任务**: 
- [ ] 选择认证方案 (推荐 mTLS for K8s)
- [ ] 实现 Interceptor/TLS 配置
- [ ] 更新客户端调用代码
- [ ] 编写集成测试

**影响范围**: 
- `auth9-core/src/grpc/`
- `auth9-core/src/server/mod.rs`
- 所有调用 gRPC 的 Business Services

---

#### 2. REST API 统一认证中间件

**任务**:
- [ ] 实现 `JwtAuthMiddleware`
- [ ] 定义公开端点白名单
- [ ] 为所有需认证端点添加 middleware
- [ ] 实现权限级别检查 (`platform_admin`, `tenant_admin`, etc.)

**实现示例**:
```rust
// src/middleware/auth.rs
pub async fn jwt_auth_middleware<B>(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, AppError> {
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::Unauthorized("Missing token".into()))?;
    
    // 验证 JWT
    let claims = state.jwt_manager
        .verify_identity_token(auth_header)
        .or_else(|_| state.jwt_manager.verify_tenant_access_token(auth_header, None))
        .map_err(|_| AppError::Unauthorized("Invalid token".into()))?;
    
    // 注入 claims 到 request extensions
    request.extensions_mut().insert(claims);
    
    Ok(next.run(request).await)
}

// 应用到路由
Router::new()
    .route("/api/v1/tenants", get(api::tenant::list))
    .layer(middleware::from_fn_with_state(state.clone(), jwt_auth_middleware))
```

---

#### 3. Rate Limiting

**任务**:
- [ ] 引入 `tower-governor` crate
- [ ] 为关键端点添加限流
- [ ] 配置不同端点的限流策略

**限流策略**:

| 端点类型 | 限制 | 键 |
|---------|------|-----|
| 登录相关 (`/api/v1/auth/*`) | 10 req/min | IP |
| Token Exchange (gRPC) | 100 req/min | client_id |
| 管理 API | 60 req/min | user_id |
| 公开端点 | 1000 req/min | IP |

**实现**:
```rust
use tower_governor::{GovernorLayer, GovernorConfigBuilder};

let governor_conf = Box::new(
    GovernorConfigBuilder::default()
        .per_millisecond(100)  // 10 req/s
        .burst_size(30)
        .finish()
        .unwrap(),
);

Router::new()
    .route("/api/v1/auth/token", post(api::auth::token))
    .layer(GovernorLayer { config: Box::leak(governor_conf) })
```

---

### 🔒 P1 - 高优先级 (3-5 天)

#### 4. 权限级别实现

**任务**:
- [ ] 定义权限枚举
- [ ] 实现权限检查 trait
- [ ] 为每个端点添加权限注解
- [ ] 实现租户隔离验证

**权限模型**:
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Permission {
    // 平台级
    PlatformAdmin,      // 跨租户管理
    AuditViewer,        // 全局审计日志查看
    
    // 租户级
    TenantOwner,        // 租户所有者 (可删除租户)
    TenantAdmin,        // 租户管理员 (管理用户/角色)
    TenantMember,       // 普通成员 (只读)
    
    // 服务级
    ServiceOwner,       // 服务创建者
}

// 权限检查
impl Claims {
    pub fn has_permission(&self, perm: Permission, tenant_id: Option<Uuid>) -> bool {
        match perm {
            Permission::PlatformAdmin => {
                self.roles.contains(&"platform_admin".to_string())
            }
            Permission::TenantAdmin => {
                if let Some(tid) = tenant_id {
                    self.tenant_id == tid.to_string() 
                        && (self.roles.contains(&"tenant_admin") 
                            || self.roles.contains(&"tenant_owner"))
                } else {
                    false
                }
            }
            // ...
        }
    }
}
```

---

#### 5. CORS 白名单

**当前配置** (不安全):
```rust
let cors = CorsLayer::new()
    .allow_origin(Any)  // ⚠️ 允许所有域名
    .allow_methods(Any)
    .allow_headers(Any);
```

**改进配置**:
```rust
use tower_http::cors::AllowOrigin;

let allowed_origins = vec![
    "https://portal.auth9.example.com".parse().unwrap(),
    "https://app.example.com".parse().unwrap(),
];

let cors = CorsLayer::new()
    .allow_origin(AllowOrigin::list(allowed_origins))
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
    .allow_headers([AUTHORIZATION, CONTENT_TYPE])
    .allow_credentials(true);
```

---

#### 6. gRPC 审计日志

**任务**:
- [ ] 为 gRPC Interceptor 添加审计日志
- [ ] 记录调用方身份 (client_id, certificate CN)
- [ ] 记录请求参数 (脱敏 token)

**实现**:
```rust
impl Interceptor for AuditInterceptor {
    fn call(&mut self, request: Request<()>) -> Result<Request<()>, Status> {
        let client_id = extract_client_id(&request)?;
        let method = request.uri().path();
        
        // 记录审计日志
        self.audit_logger.log(AuditLog {
            actor_id: client_id,
            action: format!("grpc.{}", method),
            timestamp: Utc::now(),
            ip_address: extract_ip(&request),
        });
        
        Ok(request)
    }
}
```

---

### 📝 P2 - 中优先级 (1 周)

#### 7. Request Body Size Limit

```rust
Router::new()
    .route("/api/v1/tenants", post(api::tenant::create))
    .layer(DefaultBodyLimit::max(1 * 1024 * 1024))  // 1MB
```

---

#### 8. 敏感操作二次验证

**场景**:
- 删除租户
- 重新生成 Client Secret
- 禁用他人 MFA

**实现**:
```rust
#[derive(Deserialize)]
pub struct DeleteTenantRequest {
    pub tenant_slug: String,  // 需输入租户 slug 确认
    pub otp: Option<String>,  // 可选 OTP
}

pub async fn delete_tenant(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<DeleteTenantRequest>,
) -> Result<impl IntoResponse> {
    let tenant = state.tenant_service.get(id).await?;
    
    // 二次确认
    if tenant.slug != input.tenant_slug {
        return Err(AppError::BadRequest("Slug mismatch".into()));
    }
    
    // OTP 验证 (如果启用)
    if let Some(otp) = input.otp {
        verify_otp(&state, &otp)?;
    }
    
    state.tenant_service.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

---

#### 9. Token 黑名单

**场景**: 用户登出后 Token 仍在有效期内

**实现**:
```rust
// 登出时加入黑名单
pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse> {
    if let Some(token) = extract_token(&headers) {
        let claims = state.jwt_manager.verify_identity_token(token)?;
        let ttl = claims.exp - Utc::now().timestamp();
        
        // 加入 Redis 黑名单
        state.cache_manager
            .set(&format!("blacklist:{}", token), "1", ttl as u64)
            .await?;
    }
    
    Ok(StatusCode::NO_CONTENT)
}

// 验证时检查黑名单
pub fn verify_token_with_blacklist(
    token: &str,
    cache: &CacheManager,
) -> Result<Claims> {
    let claims = verify_jwt(token)?;
    
    // 检查黑名单
    if cache.exists(&format!("blacklist:{}", token)).await? {
        return Err(AppError::Unauthorized("Token revoked".into()));
    }
    
    Ok(claims)
}
```

---

## ✅ 审查检查清单

### 端点分类审查

- [ ] **公开端点数量 (11 个)** 是否合理？
- [ ] `/.well-known/*` 端点必须公开 (OIDC 标准)
- [ ] `/api/v1/public/branding` 公开是否可接受？(Keycloak 主题需要)
- [ ] `/api/v1/invitations/accept` 公开是否可接受？(邮件链接访问)
- [ ] 是否需要为 `/api/v1/auth/userinfo` 单独添加认证检查？

### gRPC 安全审查

- [ ] **gRPC 全部需要认证** 是否同意？
- [ ] 选择哪种认证方案？(推荐: mTLS for K8s, API Key for Dev)
- [ ] `IntrospectToken` 是否应该在生产环境禁用？
- [ ] gRPC 是否需要独立的审计日志？

### 权限模型审查

- [ ] 权限级别 (`PlatformAdmin`, `TenantOwner`, `TenantAdmin`, `TenantMember`) 是否足够？
- [ ] 是否需要细粒度权限？(如 `user:read`, `user:write`)
- [ ] 审计日志是否应该限制为 `platform_admin` + `audit_viewer`？
- [ ] 是否需要 **租户隔离验证**？(防止跨租户访问)

### 特殊场景审查

- [ ] 用户可以修改自己的信息 (`jwt.sub == user_id`) - 是否同意？
- [ ] 用户可以查看自己所属的租户列表 - 是否同意？
- [ ] 管理员禁用他人 MFA 是否需要二次验证？
- [ ] 重新生成 Client Secret 是否需要额外审计？

### 实现优先级审查

- [ ] P0 (gRPC 认证 + REST 中间件 + Rate Limiting) 是否合理？
- [ ] P1 (权限级别 + CORS + gRPC 审计) 是否合理？
- [ ] P2 (Request Limit + 二次验证 + Token 黑名单) 是否合理？

---

## 📚 相关文档

- [OIDC 标准 RFC 8414](https://datatracker.ietf.org/doc/html/rfc8414)
- [gRPC 认证指南](https://grpc.io/docs/guides/auth/)
- [Tonic TLS 配置](https://github.com/hyperium/tonic/blob/master/examples/src/tls/server.rs)
- [Auth9 架构设计](./architecture.md)
- [Auth9 API 文档](./rest-api.md)

---

## 📝 变更记录

| 版本 | 日期 | 作者 | 变更说明 |
|------|------|------|----------|
| v1.0 | 2026-01-31 | AI Assistant | 初始版本，完整端点分类和安全建议 |

---

**审查负责人**: _______________  
**审查日期**: _______________  
**批准状态**: [ ] 待审查 [ ] 已批准 [ ] 需修改