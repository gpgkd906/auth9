# Keycloak 替换计划总览（Auth9 OIDC Engine Program）

**类型**: 架构演进
**严重程度**: High
**影响范围**: auth9-core (Backend), auth9-portal (Frontend), 新增 `auth9-oidc` 服务, deploy, docs
**前置依赖**: 无
**被依赖**:
- `keycloak_phase1_identity_engine_abstraction.md`
- `keycloak_phase2_hosted_login_and_session_frontend.md`
- `keycloak_phase3_local_credentials_and_mfa.md`
- `keycloak_phase4_external_identity_broker.md`
- `keycloak_phase5_cutover_and_keycloak_retirement.md`

---

## 背景

生产环境观察到，凡是需要跳转到 Keycloak 或读取 Keycloak API 的页面，整体体验都会慢于纯 Auth9 页面。根因不应简单归结为 Java/HotSpot，而应视为一组叠加开销：

1. 浏览器跨站跳转到独立 Keycloak 页面
2. Keycloak 登录主题运行时再请求 Auth9 branding API
3. Core 在授权码交换、refresh、userinfo、session 管理上仍需调用 Keycloak
4. 用户、Client、Enterprise SSO Connector、事件流、MFA 状态等关键身份对象仍由 Keycloak 托管

当前 Auth9 已经部分具备“外部是 Auth9，内部才是 Keycloak”的结构：

- OIDC discovery/JWKS 由 Auth9 暴露
- `/api/v1/auth/authorize`、`/api/v1/auth/token`、`/api/v1/auth/userinfo` 已由 Auth9 编排
- Portal 与业务系统优先对接 Auth9，而不是直接对接 Keycloak

这意味着 Auth9 可以逐步把 Keycloak 从“对外身份平台”收缩为“内部身份引擎”，再进一步完全替换。

---

## 目标

建立一条分阶段、可回滚、可双写的替换路径，使 Auth9 最终能够：

1. 不依赖 Keycloak 托管登录页
2. 不依赖 Keycloak 的 token / userinfo / browser session 核心链路
3. 不依赖 Keycloak 托管本地账号、密码、MFA、邮件动作
4. 不依赖 Keycloak 托管企业联邦与社交登录
5. 最终移除运行时 Keycloak 依赖，仅保留必要的迁移工具和兼容层

---

## 非目标

本计划不要求在一个版本内“完全重写 Keycloak 的全部能力”。以下内容应按阶段评估，而不是在前期一次性完成：

- 完整通用 IAM 平台能力
- Keycloak Admin Console 的所有管理功能一比一兼容
- 所有协议一次性迁移
- 所有历史事件、会话、credential 元数据无损即时迁移

---

## 分阶段路线

### Phase 1: 身份引擎抽象层

目标：把 `auth9-core` 对 Keycloak 的直接依赖收敛到统一接口，允许后续新增 `auth9-oidc` 实现。

见 `keycloak_phase1_identity_engine_abstraction.md`

### Phase 2: Auth9 Hosted Login + 本地 Browser Session

目标：消除用户浏览器对 Keycloak 托管页面的依赖，优先解决“跳到 Keycloak 页面慢”的体验问题。

见 `keycloak_phase2_hosted_login_and_session_frontend.md`

### Phase 3: 本地账号、密码、MFA、邮件动作

目标：Auth9 自己托管第一方账号认证闭环，不再依赖 Keycloak 保存本地密码、TOTP/WebAuthn 状态和 required actions。

见 `keycloak_phase3_local_credentials_and_mfa.md`

### Phase 4: 外部身份代理层（Social / Enterprise SSO）

目标：接管社交登录、企业 OIDC/SAML 连接器、账号链接与联邦映射。

见 `keycloak_phase4_external_identity_broker.md`

### Phase 5: Keycloak 全量退役（封闭开发）

目标：补全 auth9_oidc adapter → 移除所有 Keycloak 代码和依赖 → 清理基础设施 → 更新文档。

**方案变更**：原设计为渐进灰度切换（双栈、影子比对、回滚），改为封闭开发全量切换。

见 `keycloak_phase5_cutover_and_keycloak_retirement.md`

---

## 里程碑验收标准

### M1: Auth9 可在不修改业务接入方的前提下切换身份后端实现

- `auth9-core` 不直接依赖 `KeycloakClient` 业务语义
- 所有身份后端经统一 trait 调用
- 新增 `auth9-oidc` 实现可在 dev/test 环境跑通最小链路

### M2: 浏览器端不再跳转到 Keycloak 页面

- Portal 登录、登出、注册、忘记密码、MFA 验证页面均由 Auth9 托管
- 登录成功后的主链路不再依赖 Keycloak page render

### M3: 第一方账号不再依赖 Keycloak

- Auth9 自主管理密码哈希、重置 token、MFA factors、session
- `userinfo`、token issuance、refresh 均来自 Auth9 自身

### M4: 外部身份接入不再依赖 Keycloak

- Social login、Enterprise OIDC、Enterprise SAML broker 均可由 Auth9 自管
- 联邦身份链接、断链、映射规则由 Auth9 存储和执行

### M5: 可下线 Keycloak

- 生产环境不再需要 Keycloak Deployment / DB / Theme / Event SPI
- 所有监控、审计、运维脚本已切到 `auth9-oidc`

---

## 风险

1. **低估协议与状态复杂度**: OIDC/SAML/WebAuthn/MFA 都不是普通 CRUD
2. **切换期双写一致性**: 用户、session、federated identity 可能出现漂移
3. **安全回归**: 自研认证闭环极易引入 token、nonce、PKCE、session fixation、MFA bypass 问题
4. **范围失控**: 若不做阶段边界控制，项目会退化成“重写一个小型 Keycloak”

---

## 实施原则

1. 先做抽象和观测，再做替换
2. 先消灭用户可感知性能瓶颈，再消灭后台依赖
3. 每阶段必须支持 feature flag、双栈运行和回滚
4. 每阶段必须产出独立 QA 文档和回归清单
5. 不追求“一步到位”，但每一步都必须可单独上线

---

## 验证方法

### 架构验证

```bash
rg -n "KeycloakClient|keycloak_client\\(" auth9-core/src
rg -n "IdentityEngine|AuthBackend|FederationBroker" auth9-core auth9-oidc
```

### 运行时验证

```bash
# 关键目标：逐阶段减少对 Keycloak 运行时调用
rg -n "realms/.*/protocol/openid-connect|/admin/realms/" auth9-core/src auth9-oidc/src
```

### 用户体验验证

1. 访问登录页时不发生跨站跳转到 Keycloak
2. 登录后 token / userinfo / refresh 无需 Keycloak API
3. 关闭 Keycloak 后，已迁移阶段功能仍可用

---

## 实现顺序

1. `keycloak_phase1_identity_engine_abstraction.md`
2. `keycloak_phase2_hosted_login_and_session_frontend.md`
3. `keycloak_phase3_local_credentials_and_mfa.md`
4. `keycloak_phase4_external_identity_broker.md`
5. `keycloak_phase5_cutover_and_keycloak_retirement.md`

