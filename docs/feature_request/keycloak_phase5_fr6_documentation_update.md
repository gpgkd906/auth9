# Phase 5 FR6: 文档更新

**类型**: 文档
**严重程度**: Medium
**影响范围**: CLAUDE.md, docs/, README.md
**前置依赖**:
- `keycloak_phase5_fr5_infrastructure_cleanup.md`
**被依赖**: 无

---

## 背景

FR1-FR5 完成了代码和基础设施的全量 Keycloak 退役。项目文档仍引用 Keycloak 作为架构组件，需要同步更新。

---

## 期望行为

### R1: CLAUDE.md 更新

- 架构表：移除 Keycloak 行，更新 "Auth Engine" 为 "auth9-oidc (内置)"
- Docker 命令：移除 Keycloak 相关启动说明
- 环境变量：移除 `KEYCLOAK_*` 和 `IDENTITY_BACKEND` 引用
- 测试说明：移除 Keycloak wiremock 相关描述
- 代码组织：移除 `keycloak/` 目录说明

### R2: docs/architecture.md 更新

- 更新系统架构图：移除 Keycloak 组件
- 更新数据流：认证流直接走 auth9-core 内置 OIDC engine
- 更新组件依赖关系

### R3: 归档/删除 Keycloak 相关文档

| 文件 | 操作 |
|------|------|
| `docs/keycloak-theme.md` | 删除 |
| `docs/keycloak-service-refactor.md` | 删除 |

### R4: README.md 更新

- 更新项目描述：不再提及 Keycloak 作为依赖
- 更新快速开始：Docker Compose 命令不含 Keycloak
- 更新架构概览

### R5: QA/Security/UIUX 文档同步

- 扫描 `docs/qa/`、`docs/security/`、`docs/uiux/` 中引用 Keycloak 的步骤
- 更新测试预期：不再有 Keycloak redirect、Keycloak login page 等
- 更新安全断言：认证链路不经过外部 Keycloak

### R6: 替换计划文档归档

- `docs/feature_request/keycloak_replacement_program.md` — 更新状态为 **全部完成**，标记 Phase 5 CLOSED
- Phase 1-4 的 FR 文档已在各 Phase 关闭时删除（正常流程）
- Phase 5 FR1-FR6 文档在全部验证通过后删除

---

## 非目标

- 不修改代码（FR1-FR5 已完成）
- 不修改基础设施配置（FR5 已完成）

---

## 验证方法

```bash
# 文档中不应有 Keycloak 作为运行时依赖的描述
rg -n "keycloak" CLAUDE.md docs/architecture.md README.md
# 期望：0 结果（或仅出现在历史说明/迁移记录中）

# 已删除的文档不存在
ls docs/keycloak-theme.md 2>&1              # 期望：不存在
ls docs/keycloak-service-refactor.md 2>&1   # 期望：不存在
```
