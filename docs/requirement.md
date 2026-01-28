Identity Service 要件定义书 (V1.0)
1. 项目概述
1.1 目标
构建一个集中的身份抽象层（Identity Abstraction Layer），屏蔽 Keycloak 原生 UI 和复杂的配置逻辑，为全栈微服务提供统一的 SSO、多租户管理、动态 RBAC 以及服务治理能力。

1.2 核心理念
Headless Keycloak: Keycloak 仅负责核心协议（OIDC）、MFA 和基础账号存储。

管理面与数据面合一: Identity Service 承载管理 UI 及其对应的 API 服务。

Token 瘦身: 通过 Token Exchange 策略，将庞大的租户/角色信息从初始登录 Token 中剥离，按需交换。

2. 系统架构 (System Architecture)
2.1 部署模型
环境: 部署于 Kubernetes 集群。

组件:

Identity UI: 自研前端，提供租户、用户及服务管理界面。

Identity Backend: 处理业务逻辑，通过 Admin REST API 与 Keycloak 通信。

Keycloak: 认证引擎，配置为不暴露 /admin 公网访问。

Local Database: 存储租户元数据、服务配置及业务特定的权限映射。

3. 功能要件 (Functional Requirements)
3.1 租户管理 (Tenant Management)
生命周期: 支持租户的创建 (Create)、读取 (Read)、更新 (Update)、禁用 (Disable)。

属性定义: 租户 ID、名称、Logo、配额、专属域名配置。

租户隔离: 支持配置租户级别的登录策略（如是否强制 MFA）。

3.2 用户管理 (User Management)
统一账号: 用户在 Identity Service 注册，后台自动同步至 Keycloak 统一 Realm。

租户归属: 建立用户与租户的多对多关系。

个人中心: 支持用户修改个人资料、绑定 MFA 设备（调用 Keycloak API）。

3.3 服务/客户端治理 (Service & Client Governance)
动态注册: 允许注册新服务，自动在 Keycloak 生成 OIDC Client。

安全凭证: 管理各服务的 client_id 和 client_secret。

终端管理: 配置各服务的 Base URL、Redirect URIs（回调地址）、Logout URIs。

权限声明: 各服务在注册时需声明其拥有的功能权限点（如 user:write, report:export）。

3.4 动态 RBAC 引擎
横向角色定义: 允许不同服务自定义角色集合，互不干扰。

灵活授权: 租户管理员可将（服务 + 角色）分配给所属用户。

角色继承: (可选) 支持基础角色继承。

3.5 认证与凭证交换 (Auth & Token Exchange)
SSO 流程: 未认证请求跳转至 Identity Service（桥接 Keycloak），完成登录后回跳。

凭证交换:

用户持 Identity Token（主令牌）访问特定服务。

服务/网关请求 Identity Service 交换 Tenant Access Token。

Identity Service 验证身份，注入当前租户的角色信息，下发最终 Token。

4. 技术要件 (Technical Requirements)
4.1 接口与通信
对外接口 (OIDC): 兼容标准 OIDC 协议（Login, Logout, UserInfo）。

内部通信 (gRPC): 身份服务与其他业务服务之间使用 gRPC 进行角色同步和 Token 校验/交换。

元数据发现: 暴露 .well-known/openid-configuration。

4.2 数据结构 (JWT Payload)
Identity Token: 包含 sub, email, iss, iat, exp。

Tenant Access Token: 在上述基础上增加 tenant_id 和该租户下的 roles (基于 Resource Access 规范)。

4.3 安全设计
Secrets 管理: 敏感配置（Keycloak Admin Secret 等）存储于 K8s Secrets。

Admin API 保护: Identity Service 与 Keycloak 的交互应在 K8s 内部网络完成，不走公网。

Token 校验: 所有 Access Token 必须包含 aud (Audience) 校验。

5. 非功能要件 (Non-Functional Requirements)
高可用性 (HA): Identity Service 必须支持水平扩展，多副本部署。

性能:

Token 交换接口延迟应在 20ms 以内（建议使用缓存）。

支持每秒处理 1000+ 次鉴权请求。

一致性: Identity Service 本地库与 Keycloak 之间的数据必须保持最终一致性，需具备异常重试机制。

审计日志: 记录所有管理面的操作（谁在何时修改了哪个租户的角色）。

6. 核心流程图 (Sequence Diagram)
用户访问业务服务 -> 发现无 Token。

重定向 -> Keycloak 认证页面。

认证成功 -> Keycloak 带 Code 回跳业务服务。

换取 Token -> 业务服务拿到 Identity Token。

Token Exchange -> 业务服务通过 gRPC 请求 Identity Service。

返回结果 -> 业务服务获得包含 Tenant_A: [Editor] 角色的 Access Token。

给架构师的实施建议：
SPI 扩展: 如果需要极致的实时性，建议在 Keycloak 中实现一个 EventListener SPI，通过消息队列（如 Redis Stream 或 NATS）实时通知 Identity Service 用户状态变更。

网关集成: 建议将 Token Exchange 的逻辑下沉到 k8s Ingress Controller 或 API Gateway 层，让业务服务对认证完全无感。

====
项目概览

Project: auth9 (The Identity & RBAC Powerhouse)
1. auth9-portal (Management UI)
定位：替代 Keycloak 那套过时的 UI。

职责：提供精美的 Dashboard 供租户管理员配置用户、角色和审计日志。

2. auth9-core (The Brain)
定位：业务逻辑后端（gRPC + REST）。

职责：

作为 Keycloak 的 “Sidecar” 或包装器，执行 Admin API 调用。

管理本地数据库（存储租户属性、服务注册表、RBAC 映射）。

Token Exchange 处理器：接收主令牌，根据本地 RBAC 逻辑，签署发放针对具体租户的 auth9-access-token。

3. auth9-sdk (Optional)
定位：给其他服务的“极简接入包”。

职责：封装 gRPC 调用逻辑，实现本地 JWT 校验和 Token 交换的自动化。