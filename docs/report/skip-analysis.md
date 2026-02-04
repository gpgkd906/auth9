# QA 测试 SKIP 项分析报告

**生成时间**: 2026-02-03
**SKIP 总数**: 100 项

---

## 1. SKIP 原因分类汇总

| 分类 | 数量 | 占比 |
|------|------|------|
| Keycloak 依赖 | 22 | 22% |
| 数据/环境不足 | 18 | 18% |
| 功能未实现 | 16 | 16% |
| 保留测试数据 | 9 | 9% |
| 特殊设备/环境需求 | 14 | 14% |
| 依赖失败的前置测试 | 12 | 12% |
| 边界条件/已覆盖 | 9 | 9% |

---

## 2. 详细分析与解决方案

### 2.1 Keycloak 依赖 (22项)

**问题描述**: 多项测试依赖 Keycloak 的特定配置或功能，包括 SMTP、MFA、社交登录等。

| 模块 | 场景 | 原因 | 解决方案 |
|------|------|------|----------|
| auth/01-oidc-login | 首次登录同步 | 需要新 Keycloak 用户 | 在 Keycloak 管理界面创建测试用户 |
| auth/01-oidc-login | 带 MFA 登录 | 需要 MFA 用户 | 为测试用户启用 TOTP |
| auth/01-oidc-login | MFA 验证失败 | 需要 MFA 用户 | 同上 |
| auth/03-password | 密码重置 (1-3) | SMTP 未配置 | 配置 Keycloak SMTP (使用 Mailpit) |
| auth/03-password | 修改密码 | Keycloak 账户页面 | 访问 Keycloak 账户控制台测试 |
| auth/03-password | 密码强度验证 | Keycloak 策略 | 在 Realm 设置中配置密码策略后测试 |
| auth/04-social | 社交登录 (1-3) | 需要 IdP 配置 | 配置 Google/GitHub OAuth 凭据 |
| auth/05-boundary | 超长用户名 | Keycloak 限制 | 使用 Keycloak API 测试限制 |
| auth/05-boundary | 特殊字符密码 | Keycloak 策略 | 配置密码策略后测试 |
| settings/01-branding | 启用/禁用注册 | Keycloak realm 设置 | 通过 Keycloak Admin API 配置 |
| user/02-advanced | 密码重置 | SMTP 未配置 | 同 auth/03-password |
| identity-provider/02 | 禁用 IdP 后登录 | 需要真实 IdP | 配置完整的 OAuth 集成 |
| identity-provider/02 | 无效配置验证 | 需要真实 IdP | 同上 |
| identity-provider/02 | 更新后验证 | 编辑功能异常 | 先修复 identity-provider 编辑 bug |

**建议优先级**: ⭐⭐⭐⭐⭐ (高)

**统一解决方案**:
1. **配置 Keycloak SMTP** → 指向 Mailpit (localhost:1025)
2. **创建 MFA 测试用户** → 在 Keycloak 中创建并绑定 TOTP
3. **配置 OAuth 集成** → 申请 Google/GitHub OAuth 凭据用于测试

---

### 2.2 数据/环境不足 (18项)

**问题描述**: 测试环境中缺乏足够的测试数据，如租户、用户、角色等。

| 模块 | 场景 | 原因 | 解决方案 |
|------|------|------|----------|
| auth/02-token-exchange | Token Exchange (1-5) | 需要租户用户数据 | 创建租户用户并分配角色 |
| tenant/02-list-settings | 租户列表分页 | 只有 1 个租户 | 创建 10+ 测试租户 |
| tenant/02-list-settings | 租户设置更新 | 需要 MFA 设置功能 | 实现租户级 MFA 配置 |
| user/02-advanced | 用户搜索 | 用户数量不足 | 创建 10+ 测试用户 |
| user/02-advanced | 用户禁用/启用 | 需要更多用户 | 同上 |
| rbac/02-role | 删除有子角色的角色 | 需要多层继承结构 | 创建角色继承链 A→B→C |
| rbac/03-assignment | 给用户分配角色 | 需要租户用户 | 先创建租户用户关联 |
| rbac/03-assignment | 移除用户角色 | 需要租户用户 | 同上 |
| rbac/04-advanced | 权限继承验证 | 需要多层角色 | 创建有继承关系的角色链 |
| rbac/04-advanced | 跨服务权限隔离 | 需要多服务 | 创建多个服务及其权限 |
| invitation/01 | 邀请已存在成员 | 需要现有成员 | 确保租户有成员后测试 |
| invitation/03 | 邀请列表分页 | 邀请数量不足 | 创建 10+ 邀请 |
| session/01 | 撤销所有其他会话 | 需要多个其他会话 | 多浏览器/设备登录 |
| session/03 | 暴力破解告警 | 需要失败登录事件 | 依赖 Keycloak 事件集成 |
| session/03 | 新设备登录告警 | 需要新设备环境 | 使用不同 User-Agent 登录 |
| session/03 | 异地登录告警 | 需要 VPN 模拟 | 使用代理服务器模拟异地 IP |

**建议优先级**: ⭐⭐⭐⭐ (中高)

**统一解决方案**:
创建 **测试数据初始化脚本** (`scripts/seed-qa-data.sh`):
```bash
# 创建多个租户
for i in {1..10}; do
  curl -X POST /api/tenants -d '{"name": "Test Tenant '$i'", "slug": "test-tenant-'$i'"}'
done

# 创建多个用户
for i in {1..10}; do
  curl -X POST /api/users -d '{"email": "user'$i'@test.com", "name": "Test User '$i'"}'
done

# 创建角色继承链
curl -X POST /api/roles -d '{"name": "grandparent", ...}'
curl -X POST /api/roles -d '{"name": "parent", "parent_role_id": "..."}'
curl -X POST /api/roles -d '{"name": "child", "parent_role_id": "..."}'
```

---

### 2.3 功能未实现 (16项)

**问题描述**: 某些测试场景需要的功能尚未在系统中实现。

| 模块 | 场景 | 缺失功能 | 优先级 | 工作量 |
|------|------|----------|--------|--------|
| settings/01-branding | 重置为默认 | 品牌设置重置按钮 | P3 | 小 |
| user/02-advanced | 批量用户操作 | 批量选择/操作 UI | P2 | 中 |
| user/02-advanced | 用户导出 | CSV/Excel 导出 | P3 | 中 |
| invitation/03 | 批量管理邀请 | 批量撤销/重发 | P3 | 中 |
| audit/01 | 筛选审计日志 | 时间/类型筛选 | P2 | 小 |
| audit/01 | 审计日志详情 | 详情弹窗/页面 | P2 | 中 |
| audit/01 | 导出审计日志 | CSV/JSON 导出 | P3 | 中 |
| audit/01 | 日志时间范围 | 时间范围选择器 | P2 | 小 |
| analytics/02 | 事件详情 | 事件详情页面 | P3 | 中 |

**建议优先级**: ⭐⭐⭐ (中)

**建议**: 这些是产品功能增强，可添加到产品 backlog。其中审计日志筛选和时间范围是较常见需求，建议优先实现。

---

### 2.4 保留测试数据 (9项)

**问题描述**: 为避免破坏测试环境，部分删除/修改操作被跳过。

| 模块 | 场景 | 原因 |
|------|------|------|
| tenant/01-crud | 删除无关联租户 | 保留测试数据 |
| tenant/01-crud | 删除有关联租户 | 需要级联删除测试 |
| settings/02-email | 切换提供商类型 | 避免改变现有配置 |
| settings/03-email | 发送测试邮件 | 避免发送测试邮件 |
| rbac/03-assignment | 移除角色权限 | 保留测试数据 |
| service/01 | 删除服务 | 保留系统服务 |
| invitation/03 | 撤销邀请 | 保留测试数据 |

**解决方案**:
- 在 **专用 QA 测试环境** 中进行这些测试
- 或在测试前后 **备份/恢复数据库**
- 使用 `npm run test:e2e:full:reset` 重置环境后测试

---

### 2.5 特殊设备/环境需求 (14项)

**问题描述**: 某些测试需要特殊硬件、多设备或特定网络环境。

| 模块 | 场景 | 需求 | 解决方案 |
|------|------|------|----------|
| auth/05-boundary | 并发登录 | 多客户端 | 使用 Playwright 多上下文 |
| passkeys/01 | 注册新 Passkey | WebAuthn 设备 | 使用虚拟认证器 (Chrome DevTools) |
| passkeys/01 | 使用 Passkey 登录 | WebAuthn 设备 | 同上 |
| session/04 | 并发会话限制 | 多设备测试 | 多浏览器实例 |
| session/04 | 会话劫持防护 | 安全测试工具 | 使用 Burp Suite / OWASP ZAP |
| session/04 | 跨域会话 | 多域配置 | 配置多个域名指向测试环境 |
| session/04 | 会话持久化 | 重启测试 | 重启服务后验证会话 |
| webhook/02 | 触发测试 (1-5) | webhook 端点 | 部署 webhook.site 或本地 ngrok |
| webhook/03 | 可靠性测试 (1-4) | 模拟失败/慢端点 | 使用 mockbin.io 或本地模拟服务器 |

**建议优先级**: ⭐⭐ (低)

**统一解决方案**:
1. **Playwright 虚拟认证器** 用于 Passkey 测试
2. **ngrok/webhook.site** 用于 Webhook 测试
3. **多浏览器上下文** 用于并发会话测试

---

### 2.6 依赖失败的前置测试 (12项)

**问题描述**: 这些测试依赖其他失败的测试场景。

| 模块 | 场景 | 依赖 | 解决方案 |
|------|------|------|----------|
| service/01 | 更新服务 | 创建服务失败 | 先修复服务创建 bug |
| service/02 | 客户端管理 (1-5) | 服务创建问题 | 同上 |
| service/03 | OIDC 配置 (1-5) | 服务创建问题 | 同上 |

**阻塞问题**: 服务创建返回 "Unprocessable Entity" 错误

**建议优先级**: ⭐⭐⭐⭐⭐ (最高)

**解决方案**:
调查并修复 `service/01-service-crud.md` 场景 1 的 bug。可能原因：
- Keycloak 客户端配置问题
- 字段验证失败
- 需要检查 `/api/services` POST 请求的验证逻辑

---

### 2.7 边界条件/已覆盖 (9项)

**问题描述**: 这些测试是边界条件或已被其他测试覆盖。

| 模块 | 场景 | 原因 |
|------|------|------|
| tenant/02 | Slug 格式验证 | 已在创建测试中验证 |
| user/03 | 必填字段验证 | 基本验证已覆盖 |
| user/03 | 特殊字符处理 | 边界条件 |
| rbac/01 | 权限代码格式验证 | 格式提示已显示 |
| rbac/03 | 批量分配权限 | 单权限场景已测试 |
| webhook/04 | 无效 URL 验证 | 基本验证已测试 |
| webhook/04 | 空事件列表 | 边界条件 |
| webhook/04 | 特殊字符处理 | 边界条件 |
| analytics/01 | 无数据时的显示 | 系统有登录数据 |

**建议**: 这些可标记为 **低优先级** 或 **可选测试**，在回归测试时考虑。

---

## 3. 优先级行动计划

### 🔴 P0 - 立即修复 (阻塞其他测试)

| 问题 | 影响范围 | 预计工作量 |
|------|----------|------------|
| 服务创建失败 | 12 个测试 | 1-2 小时 |
| 身份提供商编辑按钮无响应 | 2 个测试 | 0.5-1 小时 |
| 失败登录事件未记录 | 4 个测试 | 需要 Keycloak Event Listener |

### 🟡 P1 - 短期 (1-2 天)

| 任务 | 说明 |
|------|------|
| 配置 Keycloak SMTP | 指向 Mailpit，解锁密码重置测试 |
| 创建 MFA 测试用户 | 解锁 MFA 登录测试 |
| 编写测试数据种子脚本 | 创建足够的租户/用户/角色数据 |

### 🟢 P2 - 中期 (1 周)

| 任务 | 说明 |
|------|------|
| 实现 Keycloak Event Listener SPI | 同步失败登录事件到 auth9 |
| 配置 OAuth 集成 | 申请 Google/GitHub OAuth 凭据 |
| 部署 Webhook 测试端点 | 使用 webhook.site 或本地服务 |

### 🔵 P3 - 长期 (功能增强)

| 功能 | 优先级 |
|------|--------|
| 审计日志筛选和时间范围 | 中 |
| 用户批量操作 | 中 |
| 数据导出功能 | 低 |
| 品牌设置重置按钮 | 低 |

---

## 4. 测试覆盖率提升路线图

```
当前状态:
  已通过: 69 (39%)
  已跳过: 100 (57%)
  已失败: 6 (4%)

修复 P0 后:
  已通过: 69 → 83 (+14)
  已跳过: 100 → 86 (-14)
  覆盖率: 39% → 47%

完成 P1 后:
  已通过: 83 → 110 (+27)
  已跳过: 86 → 59 (-27)
  覆盖率: 47% → 63%

完成 P2 后:
  已通过: 110 → 140 (+30)
  已跳过: 59 → 29 (-30)
  覆盖率: 63% → 80%
```

---

## 5. 附录: 按模块统计

| 模块 | 总场景 | PASS | SKIP | FAIL | 通过率 |
|------|--------|------|------|------|--------|
| auth | 23 | 4 | 18 | 0 | 17% |
| settings | 15 | 10 | 5 | 0 | 67% |
| tenant | 10 | 4 | 6 | 0 | 40% |
| user | 13 | 6 | 7 | 0 | 46% |
| rbac | 17 | 8 | 9 | 0 | 47% |
| service | 15 | 2 | 12 | 1 | 13% |
| invitation | 15 | 4 | 11 | 0 | 27% |
| session | 20 | 8 | 9 | 3 | 40% |
| webhook | 17 | 5 | 12 | 0 | 29% |
| identity-provider | 10 | 5 | 4 | 1 | 50% |
| passkeys | 5 | 1 | 4 | 0 | 20% |
| analytics | 10 | 8 | 2 | 0 | 80% |
| audit | 5 | 1 | 4 | 0 | 20% |
