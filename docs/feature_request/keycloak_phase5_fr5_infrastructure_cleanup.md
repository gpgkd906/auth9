# Phase 5 FR5: 基础设施与 Portal 清理

**类型**: 基础设施清理
**严重程度**: Medium
**影响范围**: docker-compose, auth9-portal, auth9-keycloak-theme, auth9-keycloak-events, deploy/, scripts/
**前置依赖**:
- `keycloak_phase5_fr4_refactor_config.md`
**被依赖**:
- `keycloak_phase5_fr6_documentation_update.md`

---

## 背景

FR3-FR4 完成了 auth9-core 内部的 Keycloak 代码和配置清理。但 Keycloak 残留仍存在于：
- docker-compose.yml（Keycloak 容器、Theme Builder、Events SPI Builder）
- auth9-portal（LOGIN_MODE "oidc" 模式）
- 独立项目目录（auth9-keycloak-theme/、auth9-keycloak-events/）
- K8s deploy 配置

---

## 期望行为

### R1: Docker Compose 清理

文件：`docker-compose.yml`（及 `docker-compose.dev.yml`, `docker-compose.observability.yml` 等 overlay 文件）

| 操作 | 具体内容 |
|------|---------|
| 删除 service | `keycloak` |
| 删除 service | `auth9-theme-builder` |
| 删除 service | `auth9-keycloak-events-builder` |
| 删除 volume | `keycloak-theme` |
| 删除 depends_on | auth9-core 和 auth9-core-production 中的 `keycloak` 依赖 |
| 删除环境变量 | auth9-core 中的所有 `KEYCLOAK_*` 变量 |
| 删除环境变量 | `IDENTITY_BACKEND` |
| 更新注释 | 移除 "Keycloak (Auth Engine)" 相关注释 |

### R2: Portal 清理

#### LOGIN_MODE 简化

文件：`auth9-portal/app/routes/login.tsx`（及相关文件）

- 删除 `LOGIN_MODE` 对 `"oidc"` 和 `"percentage"` 模式的支持
- `"hosted"` 成为唯一模式（可直接移除 LOGIN_MODE 判断逻辑）
- 删除 `redirectToOidcLogin()` 函数
- 删除 `shouldUseHostedLogin()` helper
- 删除 `LOGIN_MODE` 和 `LOGIN_ROLLOUT_PCT` 环境变量
- 从 docker-compose 的 auth9-portal service 中删除这些环境变量

#### E2E 测试清理

- `tests/e2e-integration/setup/keycloak-admin.ts` — 如果只用于 Keycloak test data setup，删除
- `tests/e2e-integration/global-setup.ts` — 移除 Keycloak 相关的 test user 创建
- `tests/e2e-integration/setup/test-config.ts` — 移除 Keycloak URL 配置

### R3: 删除外部项目目录

| 目录 | 内容 | 操作 |
|------|------|------|
| `auth9-keycloak-theme/` | Keycloak 登录主题（Keycloakify） | 整个目录删除 |
| `auth9-keycloak-events/` | Keycloak Events SPI（Java） | 整个目录删除 |

### R4: 删除 Keycloak Seeder 残留

文件：`auth9-core/src/migration/mod.rs`（如果 FR3 未完全清理）

- 移除 `KeycloakSeeder` 的任何残留 import 或调用
- 移除启动时的 Keycloak realm/client 同步逻辑

### R5: K8s Deploy 清理

目录：`deploy/`

- 删除 Keycloak Deployment / StatefulSet / Service 清单
- 删除 Keycloak ConfigMap / Secret
- 更新 auth9-core Deployment 中的 `KEYCLOAK_*` 环境变量引用
- 删除 `IDENTITY_BACKEND` 环境变量
- 更新 Ingress / NetworkPolicy 中的 Keycloak 路由规则

### R6: Scripts 清理

目录：`scripts/`

- 检查并更新所有引用 Keycloak 的脚本
- `scripts/reset-docker.sh` — 移除 Keycloak 容器重置逻辑

---

## 非目标

- 不修改 auth9-core Rust 代码（FR3-FR4 已完成）
- 不更新文档（FR6 负责）

---

## 验证方法

```bash
# Docker Compose 无 Keycloak 依赖
docker-compose config | grep -i keycloak
# 期望：0 结果

# 启动正常
docker-compose down -v && docker-compose up -d
# 期望：无 keycloak 容器，auth9-core 正常启动

# Portal 测试
cd auth9-portal && npm run test
cd auth9-portal && npm run test:e2e

# 删除的目录不存在
ls auth9-keycloak-theme/ 2>&1  # 期望：不存在
ls auth9-keycloak-events/ 2>&1  # 期望：不存在

# 全局扫描
rg -n "keycloak" docker-compose.yml deploy/ scripts/
# 期望：0 结果（或仅存在于注释/历史文档中）
```
