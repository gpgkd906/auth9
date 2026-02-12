# 技术负债 #001: Action Test Endpoint - axum/tonic 版本冲突

**创建日期**: 2026-02-12
**状态**: 🔴 Active
**优先级**: Medium
**影响范围**: Action 测试功能
**预计修复时间**: 1-2 天（等待上游依赖更新）

---

## 问题描述

Action Test Endpoint (`POST /api/v1/tenants/{tenant_id}/actions/{action_id}/test`) 当前无法完全实现，因为存在 axum 和 tonic 之间的版本冲突。

### 技术细节

- **auth9-core** 使用 **axum 0.8.8**（最新稳定版）
- **tonic 0.12.3** 依赖 **axum 0.7.9**
- 这导致项目中存在两个不同版本的 axum
- `Handler<T, S>` trait 在两个版本中不兼容
- 编译器报错：trait bound 不满足

### 错误示例

```rust
error[E0277]: the trait bound `fn(State<S>, ..., ..., ...) -> ... {test_action::<...>}: Handler<_, _>` is not satisfied
note: there are multiple different versions of crate `axum` in the dependency graph
```

### 依赖树

```
auth9-core
├── axum 0.8.8 ✅ (直接依赖)
└── tonic 0.12.3
    └── axum 0.7.9 ❌ (间接依赖，冲突)
```

---

## 当前解决方案 (Workaround)

### 实现方式

`test_action` handler 已实现但返回限制说明：

```rust
pub async fn test_action<S: HasServices>(
    State(state): State<S>,
    _auth: AuthUser,
    Path((tenant_id, action_id)): Path<(StringUuid, StringUuid)>,
    Json(_req): Json<TestActionRequest>,
) -> Result<Json<SuccessResponse<TestActionResponse>>, AppError> {
    // 验证 Action 存在
    let action_service = state.action_service();
    let _action = action_service.get(action_id, tenant_id).await?;

    // 返回说明性响应
    let response = TestActionResponse {
        success: false,
        error_message: Some(
            "Test endpoint temporarily unavailable due to axum/tonic version conflict. \
             To test this action: (1) Enable it and trigger through actual login, \
             (2) Check execution logs after triggering, or (3) Use Portal UI test button when available."
        ),
        console_logs: vec![
            "This endpoint will be fully functional after resolving dependency conflicts".to_string(),
        ],
        duration_ms: 0,
        modified_context: None,
    };

    Ok(Json(SuccessResponse::new(response)))
}
```

### 功能影响

#### ✅ 不受影响的功能
- Action CRUD（创建、读取、更新、删除）
- Action 执行（在实际认证流程中）
- Action 日志查询
- Action 统计查询
- 批量操作
- TypeScript SDK 的所有其他功能

#### ❌ 受影响的功能
- **Action 测试端点**：无法通过 API 直接测试 Action 脚本
- **SDK test() 方法**：返回限制说明而非实际执行结果

### 替代测试方法

用户可以通过以下方式测试 Actions：

1. **启用并触发实际流程**
   ```bash
   curl -X PATCH http://localhost:8080/api/v1/tenants/{tenant_id}/actions/{action_id} \
     -H "Authorization: Bearer $TOKEN" \
     -d '{"enabled": true}'
   # 然后执行登录等实际操作
   ```

2. **查看执行日志**
   ```bash
   curl http://localhost:8080/api/v1/tenants/{tenant_id}/actions/{action_id}/logs \
     -H "Authorization: Bearer $TOKEN"
   ```

3. **解码 JWT Token**（验证 claims 修改）
   ```bash
   echo $TOKEN | cut -d. -f2 | base64 -d | jq '.claims'
   ```

---

## 长期解决方案

### 方案 1：升级 tonic（推荐）⭐

**描述**：等待或贡献 tonic 支持 axum 0.8+

**步骤**：
1. 监控 tonic 仓库：https://github.com/hyperium/tonic/issues
2. 搜索相关 issue："axum 0.8", "axum upgrade"
3. 一旦 tonic 发布兼容版本（可能是 0.13 或 0.14）：
   ```toml
   # Cargo.toml
   tonic = { version = "0.XX", features = ["transport"] }
   tonic-reflection = "0.XX"
   ```
4. 更新 `build.rs`（可能需要 API 调整）
5. 恢复完整的 `test_action` 实现：
   ```rust
   let response = action_service.test(action_id, tenant_id, req.context).await?;
   Ok(Json(SuccessResponse::new(response)))
   ```

**优点**：
- ✅ 保持使用最新的 axum 版本
- ✅ 解决根本问题
- ✅ 未来兼容性好

**缺点**：
- ⏳ 需要等待上游更新
- ⚠️ 可能需要适配新的 API

**预计时间**：1-3 个月（取决于 tonic 发布周期）

---

### 方案 2：降级 axum（不推荐）

**描述**：将 axum 降级到 0.7.x

**步骤**：
```toml
# Cargo.toml
axum = { version = "0.7", features = ["macros", "multipart"] }
```

**优点**：
- ✅ 立即解决冲突
- ✅ 可以完整实现 test endpoint

**缺点**：
- ❌ 失去 axum 0.8 的新特性和改进
- ❌ 可能需要修改大量使用 axum 0.8 API 的代码
- ❌ 向后兼容，不利于长期维护

**预计时间**：2-3 天（代码迁移 + 测试）

**不推荐原因**：axum 0.8 引入了重要的性能改进和更好的类型安全

---

### 方案 3：隔离测试服务

**描述**：创建独立的测试微服务

**架构**：
```
┌─────────────────────────┐
│  auth9-test-service     │
│  (axum 0.8, no tonic)   │
│  - Action 脚本验证      │
│  - 沙箱执行             │
│  - 测试端点             │
└───────────┬─────────────┘
            │ HTTP
            ↓
┌─────────────────────────┐
│  auth9-core             │
│  (axum 0.8 + tonic 0.12)│
│  - 实际 Action 执行     │
│  - gRPC 服务            │
│  - 主业务逻辑           │
└─────────────────────────┘
```

**优点**：
- ✅ 完全隔离依赖冲突
- ✅ 测试服务可以独立演进
- ✅ 更好的关注点分离

**缺点**：
- ❌ 增加架构复杂度
- ❌ 需要额外的部署和维护
- ❌ 代码重复（ActionEngine 需要在两个服务中）

**预计时间**：1-2 周（新服务开发 + 部署）

**适用场景**：如果 tonic 长期不支持 axum 0.8

---

### 方案 4：使用 Lua 替代 TypeScript（激进）

**描述**：将 Action 脚本语言从 TypeScript (Deno Core) 改为 Lua (mlua)

**原因**：
- mlua 不依赖 axum，不会有版本冲突
- Lua 生态成熟，性能优秀
- 内存占用更小（<5MB vs 50-100MB）

**缺点**：
- ❌ 需要重写 ActionEngine
- ❌ 用户需要学习 Lua 语法
- ❌ TypeScript 的开发体验和生态更好
- ❌ 与 AI Agent 集成不如 TypeScript 友好

**预计时间**：2-3 周（完全重写）

**不推荐原因**：TypeScript 是更好的 Actions 脚本语言选择

---

## 推荐方案与时间线

### 立即行动（当前）✅
- [x] 实现 workaround（已完成）
- [x] 添加详细错误消息
- [x] 文档化替代测试方法
- [x] 创建技术负债追踪

### 短期（1-3 个月）⭐ 推荐
- [ ] 监控 tonic 仓库更新
- [ ] 一旦 tonic 支持 axum 0.8：
  - [ ] 升级 tonic 版本
  - [ ] 更新 build.rs（如需要）
  - [ ] 恢复完整 test_action 实现
  - [ ] 运行完整测试套件
  - [ ] 关闭此技术负债

### 中期（3-6 个月）
如果 tonic 仍不支持 axum 0.8，考虑：
- [ ] 评估方案 3（隔离测试服务）
- [ ] 或向 tonic 贡献 PR

### 长期
- [ ] 监控依赖版本兼容性
- [ ] 建立依赖更新流程
- [ ] 自动化依赖冲突检测

---

## 相关资源

### 文档
- Actions 系统计划：`docs/plans/actions-system.md`
- QA 测试文档：`docs/qa/action/`
- 技术负债说明：本文档

### 代码位置
- Handler 实现：`src/api/action.rs:157-189`
- Service 实现：`src/service/action.rs:224-250`
- 路由注册：`src/server/mod.rs:1345`

### 上游依赖
- tonic: https://github.com/hyperium/tonic
- axum: https://github.com/tokio-rs/axum
- 相关 issue: (搜索 "axum 0.8" 在 tonic 仓库)

---

## 影响评估

### 对用户的影响
- **开发体验**: 🟡 Medium - 需要使用替代测试方法
- **生产功能**: 🟢 None - 实际 Action 执行完全正常
- **AI Agent 集成**: 🟡 Medium - SDK test() 方法受限，但可以使用日志查询

### 对开发的影响
- **新功能开发**: 🟢 None - 不影响其他功能开发
- **测试**: 🟡 Medium - 需要使用集成测试而非单元测试
- **部署**: 🟢 None - 不影响部署流程

### 技术债务成本
- **维护成本**: 🟢 Low - workaround 简单稳定
- **未来风险**: 🟡 Medium - 如果 tonic 长期不支持，需要考虑其他方案
- **学习曲线**: 🟢 Low - 文档清晰，替代方案简单

---

## 验收标准

此技术负债在以下条件满足时可关闭：

- [ ] tonic 升级到支持 axum 0.8 的版本
- [ ] `test_action` handler 可以成功调用 `action_service.test()`
- [ ] 编译无错误和警告
- [ ] 单元测试通过
- [ ] API 测试验证功能正常：
  ```bash
  # 应该返回实际的测试结果而非错误消息
  curl -X POST http://localhost:8080/api/v1/tenants/{tenant_id}/actions/{action_id}/test \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"context": {...}}' | jq '.data.success'
  # 预期: true (如果脚本成功) 或 false (如果脚本失败)
  # 而非错误消息
  ```
- [ ] SDK test() 方法返回实际执行结果
- [ ] 更新文档移除限制说明
- [ ] QA 测试通过

---

## 历史记录

| 日期 | 状态 | 变更 | 负责人 |
|------|------|------|--------|
| 2026-02-12 | 🔴 Active | 初始创建，问题识别并添加 workaround | Claude Code |
| - | - | - | - |

---

## 相关技术负债

- 无（首个技术负债）

---

**下次审查日期**: 2026-03-12 (1 个月后)
**负责人**: Backend Team
**联系方式**: 见项目 README
