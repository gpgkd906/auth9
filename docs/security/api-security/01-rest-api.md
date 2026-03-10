# API 安全 - REST API 安全测试

**模块**: API 安全
**测试范围**: REST API 端点安全
**场景数**: 5
**风险等级**: 🟠 高
**ASVS 5.0 矩阵ID**: M-API-01
**OWASP ASVS 5.0**: V4.1,V4.2,V4.3,V8.1
**回归任务映射**: Backlog #2, #20


---

## 背景知识

Auth9 REST API 概况：
- 总端点数: 69 个
- 公开端点: 11 个
- 认证端点: 58 个
- 认证方式: JWT Bearer Token

参考文档: `docs/api-access-control.md`

---

## 场景 1：未认证端点访问

### 前置条件
- 无需认证

### 攻击目标
验证所有需认证端点是否正确保护

### 攻击步骤
1. 收集所有 API 端点
2. 不带 Token 访问每个端点
3. 记录返回 200/2xx 的端点
4. 分析泄露的数据

### 预期安全行为
- 非公开端点返回 401
- 不泄露任何数据
- 错误信息不暴露内部信息

### 验证方法
```bash
# 批量测试脚本
ENDPOINTS=(
  "/api/v1/tenants"
  "/api/v1/users"
  "/api/v1/services"
  "/api/v1/roles"
  "/api/v1/audit-logs"
  "/api/v1/system/email"
)

for endpoint in "${ENDPOINTS[@]}"; do
  echo "Testing: $endpoint"
  curl -s -o /dev/null -w "%{http_code}" "http://localhost:8080$endpoint"
  echo ""
done
# 预期: 全部返回 401

# 公开端点验证
curl http://localhost:8080/health
curl http://localhost:8080/.well-known/openid-configuration
# 预期: 200 (这些应该是公开的)
```

### 修复建议
- 使用认证中间件覆盖所有路由
- 明确定义公开端点白名单
- 默认拒绝策略
- 定期审计端点保护

---

## 场景 2：Token 验证绕过

### 前置条件
- 有效和无效的 Token 样本

### 攻击目标
验证 Token 验证是否可被绕过

### 攻击步骤
1. 尝试各种无效 Token：
   - 空 Token
   - 格式错误的 Token
   - 过期 Token
   - 被篡改的 Token
   - 其他服务的 Token
2. 尝试 Token 位置变体：
   - Query 参数: `?token=xxx`
   - Cookie: `Authorization=Bearer xxx`
   - 小写 header: `authorization: Bearer xxx`

### 预期安全行为
- 所有无效 Token 返回 401
- Token 仅从标准位置读取
- 详细但不泄露信息的错误

### 验证方法
```bash
# 空 Token
curl -H "Authorization: Bearer " \
  http://localhost:8080/api/v1/users
# 预期: 401

# 格式错误
curl -H "Authorization: Bearer not.a.jwt" \
  http://localhost:8080/api/v1/users
# 预期: 401

# 过期 Token
curl -H "Authorization: Bearer $EXPIRED_TOKEN" \
  http://localhost:8080/api/v1/users
# 预期: 401 {"error": "token_expired"}

# Query 参数 Token (不应支持)
curl "http://localhost:8080/api/v1/users?access_token=$TOKEN"
# 预期: 401 (Token 从 Query 不被接受)

# Basic Auth 尝试
curl -u "admin:password" \
  http://localhost:8080/api/v1/users
# 预期: 401 (不支持 Basic Auth)
```

### 修复建议
- 仅从 Authorization header 读取 Token
- 验证 Token 格式、签名、过期
- 统一错误响应格式
- 不在 URL 中传递 Token

---

## 场景 3：API 版本与废弃端点

### 前置条件
- 了解 API 版本历史

### 攻击目标
验证旧版本 API 或废弃端点是否仍可访问

### 攻击步骤
1. 尝试访问旧版本端点：
   - `/api/v0/users`
   - `/api/users` (无版本)
   - `/v1/users` (无 api 前缀)
2. 尝试访问可能废弃的端点：
   - `/api/v1/admin/`
   - `/api/v1/internal/`
   - `/api/v1/debug/`
3. 检查是否存在隐藏端点

### 预期安全行为
- 旧版本端点返回 404 或重定向
- 内部端点不可访问
- 调试端点在生产环境禁用

### 验证方法
```bash
# 旧版本
curl http://localhost:8080/api/v0/users
curl http://localhost:8080/api/users
# 预期: 404

# 内部端点探测
curl http://localhost:8080/api/v1/internal/config
curl http://localhost:8080/api/v1/admin/settings
curl http://localhost:8080/api/v1/debug/vars
# 预期: 404

# 常见调试端点
curl http://localhost:8080/actuator
curl http://localhost:8080/metrics
curl http://localhost:8080/debug/pprof
# 预期: 404 或需认证
```

### 修复建议
- 移除废弃端点代码
- 内部端点仅在内网可访问
- 生产环境禁用调试端点
- 定期审计端点清单

---

## 场景 4：批量数据提取

### 前置条件
- 有效的认证 Token

### 攻击目标
验证是否可以大量提取数据

### 攻击步骤
1. 测试分页限制：
   - `?limit=1000000`
   - `?page=0&limit=0`
2. 测试批量导出功能
3. 检查响应大小限制
4. 尝试并发请求

### 预期安全行为
- 分页有最大限制 (MAX_PER_PAGE = 100)
- 导出有数量/频率限制
- 响应大小受限
- 并发请求限流

> **实现状态**: 已实现。`PaginationQuery` 使用 `deserialize_per_page` 自动将 `per_page`/`limit` 值 clamp 到 `MAX_PER_PAGE`(100)。超过 100 的值会被静默限制为 100，小于 1 的值返回 400 错误。默认 `per_page` = 20。

### 验证方法
```bash
# 超大分页 — 返回的 per_page 会被 clamp 到 100
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/users?limit=1000000"
# 预期: limit 被 clamp 到 100，如果数据库中不足 100 条则返回全部

# 检查实际返回数量 — 注意: 如果数据库用户不足 100 条，返回实际数量
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/users?limit=1000" | jq '.data | length'
# 预期: <= 100 (实际返回数取决于数据库中的记录数)

# 检查分页元数据 — 验证 per_page 是否被 clamp
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/users?limit=1000000" | jq '.per_page'
# 预期: 100 (已 clamp)

# 负数分页
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/users?limit=-1"
# 预期: 400 ("per_page must be a positive integer")

# 并发提取
for i in {1..100}; do
  curl -s -H "Authorization: Bearer $TOKEN" \
    "http://localhost:8080/api/v1/users?page=$i" &
done
# 观察是否触发限流
```

### 常见误报

| 症状 | 原因 | 解决 |
|------|------|------|
| limit=1000000 只返回 5 条数据 | 数据库中仅有 5 个用户，limit 已被 clamp 到 100 但数据不足 | 检查 `.per_page` 字段确认 clamp 生效，而非看 `.data` 长度 |
| 看起来 limit 没有限制 | 测试环境数据量太少，无法区分 "无限制" 和 "有限制但数据不足" | 先插入 >100 条测试数据再验证 |

### 修复建议
- ✅ 分页 limit 最大 100 (已实现)
- ✅ 默认 limit 为 20 (已实现)
- 负数参数使用默认值
- 实现请求限流

---

## 场景 5：敏感端点保护

### 前置条件
- 不同权限级别的账户

### 攻击目标
验证敏感端点的额外保护

### 攻击步骤
1. 识别敏感端点：
   - 系统配置
   - 审计日志
   - 密钥管理
   - 用户删除
2. 以低权限用户访问
3. 检查是否有额外保护 (MFA, 二次确认)

### 预期安全行为
- 敏感操作需要更高权限
- 可能需要二次验证
- 完整审计日志

### 验证方法
```bash
# 普通用户访问系统配置
curl -H "Authorization: Bearer $USER_TOKEN" \
  http://localhost:8080/api/v1/system/email
# 预期: 403

# 管理员访问 (应该成功但记录审计)
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:8080/api/v1/system/email
# 预期: 200

# 检查审计日志
SELECT * FROM audit_logs
WHERE action LIKE '%system%'
ORDER BY created_at DESC;

# 敏感操作二次验证
curl -X DELETE -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:8080/api/v1/tenants/{id}
# 预期: 需要额外确认或 OTP
```

### 修复建议
- 敏感端点需要管理员权限
- 危险操作要求二次确认
- 所有访问记录审计日志
- 实现 step-up 认证

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | 未认证端点访问 | ☐ | | | |
| 2 | Token 验证绕过 | ☐ | | | |
| 3 | API 版本与废弃端点 | ☐ | | | |
| 4 | 批量数据提取 | ☐ | | | |
| 5 | 敏感端点保护 | ☐ | | | |

---

## API 端点清单

根据 `api-access-control.md`，需要测试的端点分类：

**公开端点 (11 个)**
- `/health`, `/ready`
- `/.well-known/openid-configuration`, `/.well-known/jwks.json`
- `/api/v1/auth/authorize`, `/api/v1/auth/callback`, `/api/v1/auth/token`
- `/api/v1/auth/logout`, `/api/v1/auth/userinfo`
- `/api/v1/public/branding`, `/api/v1/invitations/accept`

**高敏感端点**
- `/api/v1/system/*` - 系统配置
- `/api/v1/tenants` POST/DELETE - 租户管理
- `/api/v1/services/*/clients/*/regenerate-secret` - 密钥重置

---

## 参考资料

- [OWASP API Security Top 10](https://owasp.org/www-project-api-security/)
- [REST API Security](https://cheatsheetseries.owasp.org/cheatsheets/REST_Security_Cheat_Sheet.html)
- [CWE-306: Missing Authentication](https://cwe.mitre.org/data/definitions/306.html)

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-API-01  
**适用控制**: V4.1,V4.2,V4.3,V8.1  
**关联任务**: Backlog #2, #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 5

### 执行清单
- [ ] M-API-01-C01 | 控制: V4.1 | 任务: #2, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-API-01-C02 | 控制: V4.2 | 任务: #2, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-API-01-C03 | 控制: V4.3 | 任务: #2, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-API-01-C04 | 控制: V8.1 | 任务: #2, #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
