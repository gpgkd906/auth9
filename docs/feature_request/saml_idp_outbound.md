# SAML IdP 出站签发（Auth9 作为 SAML Identity Provider）

**类型**: 新功能
**严重程度**: High
**影响范围**: auth9-core (Backend), auth9-portal (Frontend), Keycloak (配置)
**前置依赖**: 无（现有 SAML SP 入站功能已完成）

---

## 背景

Auth9 当前已支持 **SAML SP（入站）**——将第三方 SAML IdP 接入 Auth9 作为身份源（通过 Keycloak Identity Broker）。但缺少 **SAML IdP（出站）** 能力：将 Auth9 管理的用户身份以 SAML Assertion 的形式签发给外部 Service Provider（如企业内部应用、SaaS 服务）。

### 为什么需要这个功能

1. **企业 SSO 标准**: 许多传统企业应用（Salesforce、ServiceNow、AWS Console、Confluence 等）仅支持 SAML SP 模式接入 IdP，不支持 OIDC
2. **隐藏底层引擎**: 当前如果直接暴露 Keycloak SAML 端点，外部 SP 会看到 Keycloak 的 URL、Metadata 和签名证书，这暴露了 Auth9 的内部实现细节
3. **统一管理面**: 管理员应通过 Auth9 Portal 统一管理所有 SAML SP 注册，而非进入 Keycloak Admin Console

### 架构策略：包装 Keycloak SAML 能力 + KC_HOSTNAME 统一 URL

Keycloak 原生支持 SAML 协议的 Client（`protocol: "saml"`）。Auth9 的策略是 **包装而非重写**：

```
外部 SP  ←→  Keycloak (KC_HOSTNAME = Auth9 公开域名)  ←→  Auth9 管理 API
                ↑                                            ↑
          SAML SSO/SLO/Metadata                    CRUD SAML Application
          URL 自动使用公开域名                       通过 Keycloak Admin API 操作
```

- Auth9 通过 Keycloak Admin API 创建 `protocol: "saml"` 的 Client
- Keycloak 的 `KC_HOSTNAME` 已配置为公开域名，生成的 SAML Metadata/Assertion 中所有 URL 自动使用该域名
- Auth9 提供 Metadata 代理端点，从 Keycloak Installation Provider API 获取 XML 并透传（URL 已正确）
- SAML SSO/SLO 流量由 Keycloak 直接处理，**无需 Auth9 代理**——Nginx sidecar（K8s）或端口映射（Docker）已将流量转发到 Keycloak
- 外部 SP 只看到 Auth9 的公开域名

### 现有基础设施验证

本功能无需新增任何代理/网关配置。现有的 `KC_HOSTNAME` 配置已覆盖所有环境：

| 环境 | 配置位置 | KC_HOSTNAME 值 | SAML 端点 URL |
|------|---------|---------------|--------------|
| Docker 本地 | `docker-compose.yml:259` | `http://localhost:8081` | `http://localhost:8081/realms/auth9/protocol/saml` |
| K8s 生产 | `deploy/deploy.sh` → `apply_keycloak_configmap()` | 交互式输入的 `KEYCLOAK_PUBLIC_URL` | `https://{公开域名}/realms/auth9/protocol/saml` |

- **Docker**: `KC_HOSTNAME: http://localhost:8081` + `KC_HOSTNAME_STRICT: false` 确保本地开发可用
- **K8s**: `deploy/deploy.sh:980` 从 `CONFIGMAP_VALUES[KEYCLOAK_PUBLIC_URL]` 读取公开 URL，写入 `KC_HOSTNAME`；Nginx sidecar 网关已转发 `/realms/` 路径并屏蔽 `/admin`、`/metrics`

### 与现有功能的关系

| 维度 | SAML SP 入站（已有） | SAML IdP 出站（本 FR） |
|------|---------------------|----------------------|
| 角色 | Auth9 = Service Provider | Auth9 = Identity Provider |
| 方向 | 外部 IdP → Auth9 用户 | Auth9 用户 → 外部 SP |
| 数据流 | 消费 SAML Assertion | 签发 SAML Assertion |
| Keycloak 机制 | Identity Broker | SAML Client (`protocol: "saml"`) |
| 管理对象 | Identity Provider / Enterprise SSO Connector | SAML Application（新概念） |
| 现有模型 | `IdentityProvider`, `EnterpriseSsoConnector` | 新增 `SamlApplication` |

---

## 期望行为

### R1: 数据模型 — SAML Application

新增 `saml_applications` 表，用于管理外部 SP 的注册信息：

```sql
CREATE TABLE saml_applications (
    id CHAR(36) NOT NULL PRIMARY KEY,
    tenant_id CHAR(36) NOT NULL,
    name VARCHAR(255) NOT NULL,
    -- SP 信息
    entity_id VARCHAR(512) NOT NULL COMMENT 'SP 的 Entity ID / Audience',
    acs_url VARCHAR(1024) NOT NULL COMMENT 'Assertion Consumer Service URL',
    slo_url VARCHAR(1024) NULL COMMENT 'Single Logout URL (可选)',
    -- SAML 配置
    name_id_format VARCHAR(128) NOT NULL DEFAULT 'urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress',
    sign_assertions BOOLEAN NOT NULL DEFAULT TRUE,
    sign_responses BOOLEAN NOT NULL DEFAULT TRUE,
    encrypt_assertions BOOLEAN NOT NULL DEFAULT FALSE,
    sp_certificate TEXT NULL COMMENT 'SP 的签名/加密证书 (PEM)，用于验证 AuthnRequest 签名和加密 Assertion',
    -- 属性映射
    attribute_mappings JSON NOT NULL DEFAULT '[]' COMMENT '用户属性 → SAML Attribute 映射',
    -- Keycloak 关联
    keycloak_client_id VARCHAR(255) NOT NULL COMMENT 'Keycloak 中对应的 SAML Client ID',
    -- 状态
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    -- 索引
    UNIQUE INDEX idx_saml_app_tenant_entity (tenant_id, entity_id),
    INDEX idx_saml_app_tenant (tenant_id),
    UNIQUE INDEX idx_saml_app_kc_client (keycloak_client_id)
);
```

**属性映射 JSON 结构**:
```json
[
  {
    "source": "email",
    "saml_attribute": "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress",
    "friendly_name": "email"
  },
  {
    "source": "display_name",
    "saml_attribute": "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/name",
    "friendly_name": "displayName"
  },
  {
    "source": "tenant_roles",
    "saml_attribute": "http://schemas.auth9.com/claims/roles",
    "friendly_name": "roles"
  }
]
```

**可映射的 source 字段**:
| Source | 说明 |
|--------|------|
| `email` | 用户邮箱 |
| `display_name` | 用户显示名 |
| `first_name` | 名 |
| `last_name` | 姓 |
| `user_id` | Auth9 User ID |
| `tenant_roles` | 用户在该 Tenant 下的角色列表 |
| `tenant_permissions` | 用户在该 Tenant 下的权限列表 |

**涉及文件**:
- `auth9-core/migrations/` — 新增迁移文件
- `auth9-core/src/models/saml_application.rs` — 新增模型
- `auth9-core/src/repository/saml_application.rs` — 新增 Repository trait + 实现

### R2: Keycloak SAML Client 管理

扩展 `KeycloakClient`，新增 SAML Client 的 CRUD 方法。Keycloak 的 Client API 支持通过 `protocol: "saml"` 创建 SAML 类型的 Client。

**新增 Keycloak 方法**:

```rust
impl KeycloakClient {
    /// 创建 SAML Client
    pub async fn create_saml_client(&self, client: &KeycloakSamlClient) -> Result<()>;

    /// 更新 SAML Client
    pub async fn update_saml_client(&self, client_uuid: &str, client: &KeycloakSamlClient) -> Result<()>;

    /// 删除 SAML Client
    pub async fn delete_saml_client(&self, client_uuid: &str) -> Result<()>;

    /// 获取 SAML Client
    pub async fn get_saml_client(&self, client_uuid: &str) -> Result<KeycloakSamlClient>;

    /// 获取 SAML Client 的 Installation 配置（SP Metadata XML / IdP Metadata XML）
    pub async fn get_saml_client_installation(
        &self,
        client_uuid: &str,
        provider_id: &str, // "saml-idp-descriptor" 获取 IdP metadata
    ) -> Result<String>;
}
```

**KeycloakSamlClient 结构**（映射 Keycloak Client API）:
```rust
pub struct KeycloakSamlClient {
    pub id: Option<String>,
    pub client_id: String,           // = SP Entity ID
    pub name: Option<String>,
    pub enabled: bool,
    pub protocol: String,            // 固定 "saml"
    pub base_url: Option<String>,
    pub redirect_uris: Vec<String>,  // 包含 ACS URL
    pub attributes: HashMap<String, String>,
    // SAML 特有 attributes:
    // "saml.assertion.signature" -> "true"
    // "saml.server.signature" -> "true"
    // "saml_name_id_format" -> "email"
    // "saml.signing.certificate" -> SP 证书
    // "saml.encrypt" -> "true/false"
    // "saml_single_logout_service_url_redirect" -> SLO URL
}
```

**Keycloak API 端点**:
- `POST /admin/realms/{realm}/clients` — 创建 Client（`protocol: "saml"`）
- `PUT /admin/realms/{realm}/clients/{id}` — 更新 Client
- `DELETE /admin/realms/{realm}/clients/{id}` — 删除 Client
- `GET /admin/realms/{realm}/clients/{id}/installation/providers/{provider_id}` — 获取安装配置

**涉及文件**:
- `auth9-core/src/keycloak/client.rs` — 新增方法
- `auth9-core/src/keycloak/types.rs` — 新增 `KeycloakSamlClient` 类型

### R3: Service 层 — SamlApplicationService

```rust
pub struct SamlApplicationService {
    repo: Arc<dyn SamlApplicationRepository>,
    keycloak: Arc<KeycloakClient>,
}

impl SamlApplicationService {
    /// 创建 SAML Application
    /// 1. 验证输入（entity_id 唯一、ACS URL 格式）
    /// 2. 在 Keycloak 创建 SAML Client
    /// 3. 配置属性映射为 Keycloak Protocol Mappers
    /// 4. 存储到 saml_applications 表
    pub async fn create(&self, tenant_id: Uuid, input: CreateSamlApplicationInput)
        -> Result<SamlApplication>;

    /// 更新 SAML Application
    /// 同步更新 Keycloak Client 配置和 Protocol Mappers
    pub async fn update(&self, tenant_id: Uuid, app_id: Uuid, input: UpdateSamlApplicationInput)
        -> Result<SamlApplication>;

    /// 删除 SAML Application
    /// 1. 删除 Keycloak SAML Client
    /// 2. 删除 saml_applications 记录
    pub async fn delete(&self, tenant_id: Uuid, app_id: Uuid) -> Result<()>;

    /// 列出 Tenant 下所有 SAML Application
    pub async fn list(&self, tenant_id: Uuid) -> Result<Vec<SamlApplication>>;

    /// 获取单个 SAML Application
    pub async fn get(&self, tenant_id: Uuid, app_id: Uuid) -> Result<SamlApplication>;

    /// 获取 IdP Metadata XML
    /// 从 Keycloak Installation Provider API 获取，URL 已由 KC_HOSTNAME 自动设为公开域名
    pub async fn get_idp_metadata(&self, tenant_id: Uuid, app_id: Uuid)
        -> Result<String>;

    /// 获取 IdP 签名证书（PEM 格式）
    pub async fn get_signing_certificate(&self) -> Result<String>;
}
```

**属性映射 → Keycloak Protocol Mapper**:

Auth9 的 `attribute_mappings` 需转换为 Keycloak SAML Protocol Mapper：

```json
// Keycloak Protocol Mapper 示例
{
  "name": "email-mapper",
  "protocol": "saml",
  "protocolMapper": "saml-user-attribute-idp-mapper",
  "config": {
    "user.attribute": "email",
    "friendly.name": "email",
    "attribute.name": "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress",
    "attribute.nameformat": "URI Reference"
  }
}
```

对于 `tenant_roles` 和 `tenant_permissions` 等 Auth9 特有属性，需使用 Keycloak Script Mapper 或 Hardcoded Attribute Mapper 配合 Auth9 的 Token Exchange 机制。

**涉及文件**:
- `auth9-core/src/domains/tenant_access/service/saml_application.rs` — 新增 Service
- `auth9-core/src/domains/tenant_access/mod.rs` — 注册 Service

### R4: REST API 端点

新增 Tenant 级别的 SAML Application 管理 API：

```
GET    /api/v1/tenants/{tenant_id}/saml-apps                     # 列出所有 SAML Application
POST   /api/v1/tenants/{tenant_id}/saml-apps                     # 创建 SAML Application
GET    /api/v1/tenants/{tenant_id}/saml-apps/{app_id}            # 获取单个
PUT    /api/v1/tenants/{tenant_id}/saml-apps/{app_id}            # 更新
DELETE /api/v1/tenants/{tenant_id}/saml-apps/{app_id}            # 删除
GET    /api/v1/tenants/{tenant_id}/saml-apps/{app_id}/metadata   # 获取 IdP Metadata XML
GET    /api/v1/tenants/{tenant_id}/saml-apps/{app_id}/certificate # 下载签名证书
```

**创建请求**:
```json
POST /api/v1/tenants/{tenant_id}/saml-apps
{
  "name": "Salesforce SSO",
  "entity_id": "https://salesforce.example.com",
  "acs_url": "https://salesforce.example.com/saml/acs",
  "slo_url": "https://salesforce.example.com/saml/slo",
  "name_id_format": "email",
  "sign_assertions": true,
  "encrypt_assertions": false,
  "sp_certificate": "-----BEGIN CERTIFICATE-----\n...\n-----END CERTIFICATE-----",
  "attribute_mappings": [
    {
      "source": "email",
      "saml_attribute": "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress",
      "friendly_name": "email"
    },
    {
      "source": "tenant_roles",
      "saml_attribute": "http://schemas.auth9.com/claims/roles",
      "friendly_name": "roles"
    }
  ]
}
```

**创建响应**:
```json
{
  "id": "uuid",
  "tenant_id": "uuid",
  "name": "Salesforce SSO",
  "entity_id": "https://salesforce.example.com",
  "acs_url": "https://salesforce.example.com/saml/acs",
  "slo_url": "https://salesforce.example.com/saml/slo",
  "name_id_format": "email",
  "sign_assertions": true,
  "encrypt_assertions": false,
  "attribute_mappings": [...],
  "enabled": true,
  "metadata_url": "https://auth.example.com/api/v1/tenants/{tenant_id}/saml-apps/{id}/metadata",
  "sso_url": "https://auth.example.com/realms/auth9/protocol/saml",
  "created_at": "...",
  "updated_at": "..."
}
```

**Metadata 端点**（公开，不需认证）:
```
GET /api/v1/tenants/{tenant_id}/saml-apps/{app_id}/metadata
Content-Type: application/xml

<?xml version="1.0" encoding="UTF-8"?>
<EntityDescriptor entityID="https://auth.example.com"
    xmlns="urn:oasis:names:tc:SAML:2.0:metadata">
  <IDPSSODescriptor ...>
    <SingleSignOnService
        Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Redirect"
        Location="https://auth.example.com/realms/auth9/protocol/saml"/>
    <SingleLogoutService ... />
    <KeyDescriptor use="signing">
      <ds:KeyInfo>
        <ds:X509Data><ds:X509Certificate>...</ds:X509Certificate></ds:X509Data>
      </ds:KeyInfo>
    </KeyDescriptor>
  </IDPSSODescriptor>
</EntityDescriptor>
```

> **关键**: Metadata XML 中的 URL 由 Keycloak 的 `KC_HOSTNAME` 配置决定。Auth9 的 Metadata 代理端点只需透传 Keycloak Installation Provider API 返回的 XML，无需做 URL 重写。

**涉及文件**:
- `auth9-core/src/domains/tenant_access/api/saml_application.rs` — 新增 Handler
- `auth9-core/src/server/mod.rs` — 注册路由（metadata 端点为公开路由）

### R5: SAML SSO/SLO 端点 — 零额外配置（利用现有 KC_HOSTNAME）

外部 SP 发起 SAML AuthnRequest 时，目标 URL 需指向 Auth9 的公开域名而非 Keycloak 内部地址。**这已由现有基础设施完全解决，无需新增任何代理或配置。**

#### 工作原理

Keycloak 26+ 的 `KC_HOSTNAME` 接受完整 URL，Keycloak 会在所有协议端点（OIDC、SAML）的输出中自动使用该 URL 作为 Issuer 和 Location：

```
KC_HOSTNAME = "https://idp.example.com"
→ SAML Metadata 中 SSO Location = "https://idp.example.com/realms/auth9/protocol/saml"
→ SAML Assertion 中 Issuer = "https://idp.example.com/realms/auth9"
→ SAML SLO Location = "https://idp.example.com/realms/auth9/protocol/saml"
```

#### 各环境配置现状

**Docker 本地开发**（`docker-compose.yml:259-261`）:
```yaml
KC_HOSTNAME: http://localhost:8081
KC_HOSTNAME_STRICT: "false"      # 允许 HTTP（开发环境）
KC_HOSTNAME_STRICT_HTTPS: "false"
```
- Keycloak 监听容器内 `:8080`，Docker 端口映射 `8081:8080`
- 外部 SP 配置 `http://localhost:8081/realms/auth9/protocol/saml` 即可完成 SSO
- Auth9 Theme 已覆盖登录页 UI

**K8s 生产环境**（`deploy/deploy.sh:976-998`）:
```bash
# deploy.sh 中的 apply_keycloak_configmap() 函数
local keycloak_public_url="${CONFIGMAP_VALUES[KEYCLOAK_PUBLIC_URL]:-https://idp.auth9.example.com}"
# ...
KC_HOSTNAME: "$keycloak_public_url"
```
- `KEYCLOAK_PUBLIC_URL` 通过交互式部署脚本配置（通常为 cloudflared 隧道 URL）
- Nginx sidecar 网关（`deploy/k8s/keycloak/nginx-gw-configmap.yaml`）：
  - 转发 `/realms/` 路径到 Keycloak（包括 SAML 端点）
  - 屏蔽 `/admin` 和 `/metrics`（返回 403）
  - 设置 `X-Forwarded-Proto`、`X-Forwarded-For` 头

#### 本 FR 无需改动的文件

- `docker-compose.yml` — `KC_HOSTNAME` 已配置
- `deploy/deploy.sh` — `apply_keycloak_configmap()` 已动态设置 `KC_HOSTNAME`
- `deploy/k8s/keycloak/nginx-gw-configmap.yaml` — 已转发 SAML 端点流量
- `auth9-core/src/keycloak/seeder.rs` — 无需设置 Realm `frontendUrl`（`KC_HOSTNAME` 是全局配置，优先级更高）

#### Keycloak Theme 的角色

Keycloak Theme **不能覆盖 SAML 协议端点**（SSO、SLO、Metadata 是 Protocol Provider SPI，不是 FreeMarker 模板）。但 Theme 在 SAML IdP 流程中仍然重要：

| 阶段 | 处理者 | 说明 |
|------|--------|------|
| SP 发送 AuthnRequest | Keycloak SAML Protocol SPI | 解析 XML、验证签名 |
| **用户登录页** | **Auth9 Keycloak Theme** | 用户看到的是 Auth9 品牌的登录页 |
| 签发 SAML Assertion | Keycloak SAML Protocol SPI | 签名、加密、属性映射 |
| Assertion POST 到 SP ACS | 浏览器自动提交 | Keycloak 生成的 HTML auto-submit form |

即：SAML 协议层由 Keycloak 内核处理，用户交互层由 Auth9 Theme 处理，两者各司其职。

### R6: Portal 管理 UI

在 Tenant 详情页新增 "SAML Applications" Tab：

**列表页**:
- 显示所有已注册的 SAML Application（名称、Entity ID、状态、创建时间）
- 快速启用/禁用开关
- 创建新 SAML Application 按钮

**创建/编辑表单**:
```
┌─────────────────────────────────────────────┐
│ Add SAML Application                        │
├─────────────────────────────────────────────┤
│ Name:           [Salesforce SSO          ]  │
│ Entity ID:      [https://sf.example.com  ]  │
│ ACS URL:        [https://sf.example.com/ ]  │
│ SLO URL:        [                        ]  │
│ NameID Format:  [email ▼]                   │
│                                             │
│ ☑ Sign Assertions   ☑ Sign Responses       │
│ ☐ Encrypt Assertions                       │
│                                             │
│ SP Certificate (PEM):                       │
│ ┌─────────────────────────────────────────┐ │
│ │ -----BEGIN CERTIFICATE-----             │ │
│ │ ...                                     │ │
│ └─────────────────────────────────────────┘ │
│                                             │
│ Attribute Mappings:                         │
│ ┌───────────┬─────────────────────┬──────┐ │
│ │ Source    │ SAML Attribute      │ Name │ │
│ ├───────────┼─────────────────────┼──────┤ │
│ │ email ▼  │ [claims/email     ] │[email]│ │
│ │ [+ Add Mapping]                        │ │
│ └────────────────────────────────────────┘ │
│                                             │
│            [Cancel]  [Save]                 │
└─────────────────────────────────────────────┘
```

**详情页**:
- 显示配置摘要
- **IdP Metadata URL** — 一键复制，供外部 SP 配置使用
- **下载 Metadata XML** 按钮
- **下载签名证书** 按钮
- **Setup Instructions** 折叠面板：针对常见 SP（Salesforce、AWS、Google Workspace）的配置步骤提示

**涉及文件**:
- `auth9-portal/app/routes/dashboard.tenants.$tenantId.saml-apps.tsx` — 新增页面
- `auth9-portal/app/services/api/saml-application.ts` — 新增 API Client
- `auth9-portal/app/routes/dashboard.tenants.$tenantId.tsx` — 添加 Tab 导航

### R7: 单元测试覆盖

- **Repository 层**: `SamlApplicationRepository` CRUD 的 mock 测试
- **Service 层**:
  - 创建流程（Keycloak Client 创建 + DB 存储）
  - 更新流程（属性映射变更 → Protocol Mapper 同步）
  - 删除流程（级联删除 Keycloak Client + DB 记录）
  - Entity ID 唯一性校验
  - Metadata 获取（验证透传 Keycloak Installation Provider API 的 XML）
- **API 层**: Handler 请求/响应测试（使用 `TestAppState` + mock）
- **Keycloak 层**: `wiremock` 模拟 SAML Client CRUD 和 Installation Provider API
- **属性映射**: Source → Keycloak Protocol Mapper 转换逻辑

---

## 安全考量

### SAML 签名与加密
1. **Assertion 签名**: 默认开启，使用 Keycloak Realm 的签名密钥（RSA-SHA256）
2. **Response 签名**: 默认开启，防止中间人篡改
3. **Assertion 加密**: 可选，需要 SP 提供加密证书（AES-128-CBC + RSA-OAEP）
4. **AuthnRequest 签名验证**: 如果 SP 提供了证书，应验证 AuthnRequest 的签名

### URL 隐藏（由 KC_HOSTNAME 保障）
- `KC_HOSTNAME` 确保 Metadata XML 中所有 Location URL 使用公开域名
- Keycloak 内部地址（`http://keycloak:8080`）不会出现在协议输出中
- K8s Nginx sidecar 屏蔽 `/admin` 和 `/metrics`，防止管理接口暴露
- Auth9 错误页面（Theme `Error.tsx`）不泄露 Keycloak 内部信息

### 属性映射安全
- `tenant_roles` / `tenant_permissions` 映射须确保只返回用户在当前 Tenant 下的角色/权限
- 属性值需 XML 转义，防止 SAML Assertion 注入

### 证书管理
- SP 证书的 PEM 格式验证（创建/更新时）
- IdP 签名证书轮换时需通知所有已注册的 SP 更新 Metadata
- 建议在 Portal 中显示证书过期时间并提前告警

---

## 验证方法

### 代码验证

```bash
# 搜索 SAML Application 相关实现
grep -r "saml_application\|SamlApplication" auth9-core/src/ auth9-portal/app/

# 运行后端测试
cd auth9-core && cargo test saml_application

# 运行前端测试
cd auth9-portal && npm run test
```

### 手动验证

1. 通过 Portal 创建一个 SAML Application（使用 SAML Test SP 工具，如 samltool.io）
2. 下载 IdP Metadata XML，确认所有 URL 指向 Auth9 域名
3. 在 Test SP 中配置 Auth9 的 IdP Metadata
4. 发起 SP-Initiated SSO：Test SP → Auth9 登录页 → 登录成功 → 回到 Test SP 并显示 Assertion 内容
5. 验证 Assertion 中包含配置的属性映射
6. 验证 NameID 格式正确
7. 测试 SLO（如已配置）
8. 测试禁用后 SSO 失败

### 集成测试工具

- **SAML Tracer** (Browser Extension): 抓取 SAML Request/Response
- **samltool.io**: 在线 SAML SP 模拟器
- **OneLogin SAML Test Connector**: 可用于端到端验证

---

## 实现顺序

建议按以下顺序分阶段实施：

### Phase 1: 基础 CRUD + 端到端 SSO（MVP）
1. 数据库迁移 + Model（R1）
2. Keycloak SAML Client 方法（R2）
3. Service 层（R3 基础 CRUD + Metadata）
4. REST API（R4）
5. 单元测试（R7）
6. 端到端验证：R5 无需额外工作（`KC_HOSTNAME` 已就位），MVP 完成即可进行 SAML SSO 测试

### Phase 2: Portal UI
7. Portal 管理 UI（R6）

### Phase 3: 高级功能
8. Assertion 加密支持
9. 属性映射中的 `tenant_roles` / `tenant_permissions`
10. 证书轮换告警
11. SP-Initiated SLO

---

## NameID Format 简写映射

Portal 表单中使用简写，Service 层转换为完整 URN：

| 简写 | 完整 URN |
|------|---------|
| `email` | `urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress` |
| `persistent` | `urn:oasis:names:tc:SAML:2.0:nameid-format:persistent` |
| `transient` | `urn:oasis:names:tc:SAML:2.0:nameid-format:transient` |
| `unspecified` | `urn:oasis:names:tc:SAML:1.1:nameid-format:unspecified` |

---

## 参考

- Keycloak SAML Client 文档: https://www.keycloak.org/docs/latest/server_admin/#saml-clients
- Keycloak Client Installation Provider API: `GET /admin/realms/{realm}/clients/{id}/installation/providers/saml-idp-descriptor`
- SAML 2.0 规范: https://docs.oasis-open.org/security/saml/v2.0/
- 现有 SAML SP 入站实现: `src/domains/identity/service/identity_provider.rs`, `src/domains/tenant_access/api/tenant_sso.rs`
- 现有 Service/Client 模型: `src/models/service.rs`（参考 OIDC Client 的管理模式）
- Docker KC_HOSTNAME 配置: `docker-compose.yml:259`
- K8s KC_HOSTNAME 动态配置: `deploy/deploy.sh:976-998`（`apply_keycloak_configmap()` 函数）
- K8s Nginx sidecar 网关: `deploy/k8s/keycloak/nginx-gw-configmap.yaml`

---

## Implementation Log

- **Date**: 2026-03-16
- **Phase 1 Fulfilled (Backend MVP)**:
  - ✅ R1: Data model — `saml_applications` migration + `SamlApplication` model with `AttributeMapping`, `NameIdFormat`
  - ✅ R2: Keycloak SAML Client methods — `create_saml_client`, `update_saml_client`, `delete_saml_client`, `get_saml_client_installation` + `KeycloakSamlClient`/`KeycloakProtocolMapper` types
  - ✅ R3: `SamlApplicationService` — CRUD + metadata proxy + attribute mapping → Keycloak Protocol Mapper conversion
  - ✅ R4: REST API — 6 endpoints (list/create/get/update/delete + metadata), routes registered (metadata is public, CRUD is protected)
  - ✅ R5: No work needed (KC_HOSTNAME already configured)
  - ✅ R7: 28 unit tests (model validation, repository mock, service logic, Keycloak builder, protocol mapper conversion)
- **Remaining (Phase 2 & 3)**:
  - R6: Portal management UI (SAML Applications tab in tenant detail)
  - Dedicated certificate download endpoint (`/certificate`)
  - Assertion encryption support (Phase 3)
  - `tenant_roles`/`tenant_permissions` Script Mapper integration (Phase 3)
  - Certificate rotation alerts (Phase 3)
  - SP-Initiated SLO (Phase 3)
