# Auth9-Core 领域化迁移开发规范

## 1. 目标
本规范用于后续在 `auth9-core` 中持续推进领域化重构，确保新增改造遵循统一边界，避免回退到按技术层耦合。

## 2. 目录模板
新增或迁移领域时，使用以下结构：

```text
auth9-core/src/domains/<domain>/
├── mod.rs
├── context.rs
├── routes.rs
├── api/
│   ├── mod.rs
│   └── *.rs
├── service/
│   ├── mod.rs
│   └── *.rs
└── services.rs
```

职责约定：
- `routes.rs`：只负责路由声明与中间件拼装。
- `api/*.rs`：HTTP handler 实现。
- `service/*.rs`：领域业务服务实现。
- `services.rs`：对外暴露领域 service 类型与 facade。
- `context.rs`：声明该领域对 `AppState` 的最小依赖能力。

## 3. 迁移步骤
1. 将实现从 `src/api/*.rs` / `src/service/*.rs` 迁移到目标领域目录。
2. 在原路径保留 shim：`pub use crate::domains::<domain>::...`。
3. 更新 `domains/<domain>/api/mod.rs` 与 `service/mod.rs` 的导出。
4. 让 `domains/<domain>/routes.rs` 仅依赖本领域 `api` facade。
5. 验证：`cargo check`、`cargo test --test api_test`、`./scripts/check-domain-boundaries.sh`。

## 4. 跨域调用规则
- 允许：在领域 `api` 中调用本领域 `service`。
- 尽量避免：领域 A 的 handler 直接依赖领域 B 的具体 service 类型。
- 推荐：通过 `state` trait（`context.rs`）暴露必要能力，降低编译耦合。
- 禁止：在 `routes.rs` 中直接依赖 `repository` 实现细节。

## 5. 兼容与收敛策略
- 重构期保留 `src/api`、`src/service` shim，保证外部编译路径兼容。
- 新增功能优先落在 `src/domains/<domain>`，不再新增真实实现到旧目录。
- 当全量调用方完成切换后，可计划下一阶段删除 shim（单独评审）。

## 6. 守卫与 CI
- 本地守卫脚本：`./scripts/check-domain-boundaries.sh`
- CI 必须执行该脚本，防止 `server` 与 `routes` 边界回退。

## 7. Review 检查项
提交 PR 前自检：
- [ ] 新增/修改路由是否位于 `domains/<domain>/routes.rs`
- [ ] 旧目录是否仅保留 shim
- [ ] 是否新增了跨域硬编码依赖
- [ ] `cargo check` 是否通过
- [ ] `api_test` 与边界守卫是否通过
