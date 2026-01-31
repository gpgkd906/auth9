# Auth9 实施计划

## 项目概述

| 项目 | Auth9 - Identity & RBAC Powerhouse |
|------|-------------------------------------|
| 目标 | 构建自托管的身份认证服务，替代 Auth0 |
| 后端 | Rust (axum + tonic + sqlx) |
| 前端 | Remix + TypeScript + Vite |
| 数据库 | TiDB (MySQL 兼容) |
| 缓存 | Redis |
| 镜像仓库 | ghcr.io/gpgkd906 |

## 开发阶段

### Phase 1: 基础框架 (MVP)

#### 1.1 auth9-core Rust 项目骨架

- [x] 创建 Cargo.toml 配置依赖
- [x] 配置模块 (`src/config/`)
- [x] 错误处理模块 (`src/error/`)
- [x] 领域模型 (`src/domain/`)
  - Tenant, User, Service, RBAC 模型
  - 验证规则 (validator)
- [x] 数据访问层 (`src/repository/`)
  - TenantRepository
  - UserRepository
  - ServiceRepository
  - RbacRepository
  - AuditRepository
- [x] 业务逻辑层 (`src/service/`)
- [x] JWT 模块 (`src/jwt/`)
- [x] 缓存模块 (`src/cache/`)
- [x] Keycloak 客户端 (`src/keycloak/`)
- [x] gRPC Proto 定义
- [x] 测试框架搭建

#### 1.2 TiDB Schema 设计

- [x] tenants 表
- [x] users 表
- [x] tenant_users 表
- [x] services 表
- [x] permissions 表
- [x] roles 表
- [x] role_permissions 表
- [x] user_tenant_roles 表
- [x] audit_logs 表

#### 1.3 auth9-portal Remix 项目

- [x] Remix + Vite 项目配置
- [x] Vitest 测试配置
- [x] Tailwind CSS 苹果风格主题
- [x] 基础 UI 组件
  - Button
  - Input
  - Card
  - Avatar
- [x] 页面路由
  - Landing Page
  - Login Page
  - Dashboard Layout
  - Dashboard Index
- [x] API 服务层

#### 1.4 Docker 配置

- [x] auth9-core Dockerfile (多阶段构建)
- [x] auth9-portal Dockerfile
- [x] docker-compose.yml (完整环境)
- [x] docker-compose.dev.yml (仅依赖)

### Phase 2: 核心认证

#### 2.1 Keycloak Admin API 集成

- [x] Admin Token 获取与缓存
- [x] 用户 CRUD 操作
- [x] OIDC Client 管理

#### 2.2 OIDC 登录流程

- [x] Authorization 端点
- [x] Callback 处理
- [x] Token 端点
- [x] Logout 端点
- [x] UserInfo 端点
- [x] OpenID Configuration 发现

### Phase 3: RBAC 引擎

#### 3.1 租户管理

- [x] CRUD API 实现
- [x] 租户配置管理
- [x] 租户状态管理

#### 3.2 服务注册

- [x] 服务 CRUD
- [x] Client Secret 生成与轮换
- [x] Redirect URI 管理

#### 3.3 动态 RBAC

- [x] 权限定义
- [x] 角色管理
- [x] 角色-权限映射
- [x] 用户-租户-角色分配

#### 3.4 gRPC Token Exchange

- [x] Proto 定义
- [x] ExchangeToken 实现
- [x] ValidateToken 实现
- [x] GetUserRoles 实现
- [x] IntrospectToken 实现

### Phase 3.5: 前端交互完善 ✅

#### 3.5.1 UI 组件库扩展

- [x] Dialog 组件 (Create/Edit 表单)
- [x] DropdownMenu 组件 (行操作菜单)
- [x] Select 组件
- [x] Label 组件
- [x] Checkbox 组件

#### 3.5.2 实体 CRUD 交互

- [x] Tenants: Create/Edit/Delete 交互实现
- [x] Services: Create/Edit/Delete 交互实现
- [x] Roles: Create/Edit/Delete 交互实现
- [x] Users: Create/Edit/Assign Tenant/Assign Roles 交互实现
- [x] Settings: 全局设置表单实现

#### 3.5.3 用户管理增强 (补完)

- [x] 用户创建功能 - Users 页面添加 "Create User" 按钮和对话框
- [x] 角色移除 API - `DELETE /api/v1/users/:user_id/tenants/:tenant_id/roles/:role_id`
- [x] 前端集成角色移除功能 - Manage Roles 对话框支持取消勾选即时移除

### Phase 4: 高级功能

#### 4.1 审计日志

- [x] 审计日志记录
- [x] 审计日志查询 API

#### 4.2 缓存优化

- [x] 用户角色缓存
- [x] 服务配置缓存
- [x] 租户配置缓存
- [x] 缓存失效策略

### Phase 5: 生产就绪

#### 5.1 CI/CD 配置

- [x] GitHub Actions CI (ci.yml)
  - Rust: fmt, clippy, 单元测试, 集成测试
  - Node: lint, typecheck, 测试, 构建
  - Docker 构建测试
- [x] GitHub Actions CD (cd.yml)
  - 构建多架构镜像 (amd64, arm64)
  - 推送到 ghcr.io/gpgkd906
  - 生成部署 Summary

#### 5.2 Kubernetes 部署

- [x] Namespace 配置
- [x] ConfigMap
- [x] Secrets 模板
- [x] ServiceAccount
- [x] auth9-core Deployment
- [x] auth9-core Service
- [x] auth9-core HPA
- [x] auth9-portal Deployment
- [x] auth9-portal Service
- [x] auth9-portal HPA
- [x] Ingress (HTTP + gRPC)
- [x] deploy.sh 部署脚本

## 目录结构

```
auth9/
├── auth9-core/              # Rust 后端
│   ├── src/
│   │   ├── api/             # REST API
│   │   ├── cache/           # Redis 缓存
│   │   ├── config/          # 配置
│   │   ├── domain/          # 领域模型
│   │   ├── error/           # 错误处理
│   │   ├── grpc/            # gRPC 服务
│   │   ├── jwt/             # JWT 处理
│   │   ├── keycloak/        # Keycloak 客户端
│   │   ├── repository/      # 数据访问
│   │   ├── server/          # 服务器
│   │   └── service/         # 业务逻辑
│   ├── tests/               # 集成测试
│   ├── migrations/          # 数据库迁移
│   ├── proto/               # gRPC proto
│   ├── Cargo.toml
│   └── Dockerfile
├── auth9-portal/            # Remix 前端
│   ├── app/
│   │   ├── components/ui/   # UI 组件
│   │   ├── routes/          # 页面路由
│   │   ├── services/        # API 客户端
│   │   └── lib/             # 工具函数
│   ├── tests/               # 测试
│   ├── package.json
│   ├── vite.config.ts
│   ├── vitest.config.ts
│   └── Dockerfile
├── deploy/
│   ├── k8s/
│   │   ├── namespace.yaml
│   │   ├── configmap.yaml
│   │   ├── secrets.yaml.example
│   │   ├── serviceaccount.yaml
│   │   ├── auth9-core/
│   │   ├── auth9-portal/
│   │   └── ingress.yaml
│   └── deploy.sh
├── .github/
│   └── workflows/
│       ├── ci.yml
│       └── cd.yml
├── docs/
│   ├── requirement.md
│   ├── architecture.md
│   └── implementation-plan.md
├── docker-compose.yml
├── docker-compose.dev.yml
├── .gitignore
├── .env.example
└── README.md
```

## 镜像信息

### 镜像仓库

```
ghcr.io/gpgkd906/auth9-core
ghcr.io/gpgkd906/auth9-portal
```

### 标签策略

- `latest` - 最新的 main 分支构建
- `{short-sha}` - 特定 commit 的构建 (如 `abc1234`)

### 获取镜像

```bash
# 最新版本
docker pull ghcr.io/gpgkd906/auth9-core:latest
docker pull ghcr.io/gpgkd906/auth9-portal:latest

# 特定版本
docker pull ghcr.io/gpgkd906/auth9-core:abc1234
docker pull ghcr.io/gpgkd906/auth9-portal:abc1234
```

## 部署指南

### 本地开发

```bash
# 启动依赖服务 (TiDB, Redis, Keycloak)
docker-compose -f docker-compose.yml -f docker-compose.dev.yml up -d

# 运行 auth9-core
cd auth9-core
cp .env.example .env
cargo run

# 运行 auth9-portal
cd auth9-portal
cp .env.example .env
npm install
npm run dev
```

### Docker Compose 部署

```bash
# 启动完整环境
docker-compose up -d

# 查看日志
docker-compose logs -f auth9-core
docker-compose logs -f auth9-portal
```

### Kubernetes 部署

```bash
# 1. 创建 Secrets (必须先执行)
kubectl create secret generic auth9-secrets \
  --from-literal=DATABASE_URL='mysql://user:pass@tidb:4000/auth9' \
  --from-literal=REDIS_URL='redis://redis:6379' \
  --from-literal=JWT_SECRET='your-jwt-secret' \
  --from-literal=KEYCLOAK_URL='http://keycloak:8080' \
  --from-literal=KEYCLOAK_ADMIN_CLIENT_SECRET='secret' \
  --from-literal=SESSION_SECRET='your-session-secret' \
  -n auth9

# 2. 部署
./deploy/deploy.sh

# 3. 查看状态
kubectl get pods -n auth9
kubectl get svc -n auth9
kubectl get ingress -n auth9
```

## 测试指南

### auth9-core 测试

```bash
cd auth9-core

# 单元测试
cargo test --lib

# 集成测试 (需要 Docker)
cargo test --test '*'

# 带覆盖率
cargo llvm-cov --html
```

### auth9-portal 测试

```bash
cd auth9-portal

# 单元测试
npm run test

# 类型检查
npm run typecheck

# 代码检查
npm run lint
```

## CI/CD 流程

### CI (Pull Request)

```mermaid
flowchart LR
    PR[Pull Request] --> Test1[test-core]
    PR --> Test2[test-portal]
    Test1 --> Docker[docker-build-test]
    Test2 --> Docker
```

### CD (Push to main)

```mermaid
flowchart LR
    Push[Push to main] --> Build1[build-core]
    Push --> Build2[build-portal]
    Build1 --> Summary[summary]
    Build2 --> Summary
    Summary --> GHCR[ghcr.io/gpgkd906]
```

## 环境变量

### auth9-core

| 变量 | 描述 | 默认值 |
|------|------|--------|
| `HTTP_HOST` | HTTP 监听地址 | `0.0.0.0` |
| `HTTP_PORT` | HTTP 端口 | `8080` |
| `GRPC_HOST` | gRPC 监听地址 | `0.0.0.0` |
| `GRPC_PORT` | gRPC 端口 | `50051` |
| `DATABASE_URL` | 数据库连接字符串 | 必填 |
| `REDIS_URL` | Redis 连接字符串 | `redis://localhost:6379` |
| `JWT_SECRET` | JWT 签名密钥 | 必填 |
| `JWT_ISSUER` | JWT 签发者 | `https://auth9.example.com` |
| `KEYCLOAK_URL` | Keycloak 地址 | `http://localhost:8081` |
| `KEYCLOAK_REALM` | Keycloak Realm | `auth9` |

### auth9-portal

| 变量 | 描述 | 默认值 |
|------|------|--------|
| `AUTH9_CORE_URL` | auth9-core 地址 | `http://localhost:8080` |
| `SESSION_SECRET` | Session 密钥 | 必填 |
| `NODE_ENV` | Node 环境 | `development` |

## 性能指标

| 指标 | 目标 | 说明 |
|------|------|------|
| Token Exchange 延迟 | < 20ms | 使用 Redis 缓存 |
| 鉴权 QPS | > 1000 | 水平扩展支持 |
| 可用性 | 99.9% | 多副本 + HPA |

## 后续规划

1. **auth9-sdk**: Rust/Go/TypeScript 多语言 SDK
2. **MFA 增强**: WebAuthn, TOTP
3. **社交登录**: Google, GitHub, WeChat
4. **Webhook**: 事件通知机制
5. **监控**: Prometheus + Grafana Dashboard
