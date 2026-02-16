# Auth9-Core 领域化重构任务状态

更新时间：2026-02-16

## 已完成
- [x] 领域骨架与路由按域组合（`src/domains/*` + `server` 聚合）
- [x] `DomainRouterState` 与各领域 `context` 约束
- [x] Identity 全量迁移（API/Service）
- [x] tenant_access 全量迁移（API/Service）
- [x] authorization 全量迁移（API/Service）
- [x] platform 全量迁移（API/Service）
- [x] integration 全量迁移（API/Service）
- [x] security_observability 全量迁移（API/Service）
- [x] 旧 `api/service` 文件收敛为 shim 兼容层
- [x] 边界守卫脚本落地并接入 CI
- [x] 架构文档更新
- [x] 迁移规范文档输出
- [x] 回归验证：`cargo check`、`cargo test --test api_test`、边界守卫

## 未完成
- 无（当前计划 100%）
