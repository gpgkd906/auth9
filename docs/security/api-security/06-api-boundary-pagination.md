# API 安全 - 批量与分页边界测试

**模块**: API 安全
**测试范围**: 批量接口、分页参数边界、资源放大风险
**场景数**: 3
**风险等级**: 🟠 高
**ASVS 5.0 矩阵ID**: M-API-06
**OWASP ASVS 5.0**: V4.4,V2.4,V13.3
**回归任务映射**: Backlog #16, #20

---

## 场景 1：分页参数越界
### 攻击目标
验证 `page/per_page/limit` 超界、负值、非数字时的处理。

> **已实现的防护**: `PaginationQuery` 的 `deserialize_per_page` 会自动将 `per_page` 截断至 `MAX_PER_PAGE`（当前为 100）。`per_page < 1` 会返回验证错误。因此 `per_page=99999999` 会被服务端截断为 100，响应中 `pagination.per_page` 显示为 100。测试时应验证截断行为是否生效，而非期望服务端返回错误。

## 场景 2：批量请求放大
### 攻击目标
验证批量接口是否存在一次请求触发过大计算或写放大。

> **前置条件 - Token 类型**：本场景需要 **Tenant Access Token**（非 Identity Token）。Identity Token 仅允许租户选择和 token exchange，无法访问服务管理和批量操作端点。获取方式参见 `scripts/qa/gen-access-token.js`。

## 场景 3：边界条件下限流一致性
### 攻击目标
验证超大分页或批量参数场景下限流仍有效。

---

## 前置条件 - JWT Key 同步

> **重要**: 测试脚本生成的 JWT Token 必须使用与 auth9-core 相同的 JWT 私钥。如果测试脚本使用了硬编码的密钥路径或独立生成的密钥对，会导致签名验证失败，所有请求返回 **401 Unauthorized**。
>
> **推荐做法**: 使用 `node .claude/skills/tools/gen_token.js`，该脚本从 `.env` 文件读取私钥，与 Docker 容器中 auth9-core 使用的密钥一致。

### 故障排除

| 症状 | 原因 | 解决方法 |
|------|------|----------|
| 所有请求返回 401 Unauthorized | 测试脚本使用的 JWT 密钥与 auth9-core 不一致 | 改用 `node .claude/skills/tools/gen_token.js` 生成 Token，它从 `.env` 读取私钥，确保与 Docker 容器一致 |
| Token 生成成功但请求仍返回 401 | `.env` 文件中的私钥与 Docker 容器中的不同步 | 运行 `./scripts/reset-docker.sh` 重置环境，确保密钥同步 |

---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-API-06  
**适用控制**: V4.4,V2.4,V13.3  
**关联任务**: Backlog #16, #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 3

### 执行清单
- [ ] M-API-06-C01 | 控制: V4.4 | 任务: #16, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-API-06-C02 | 控制: V2.4 | 任务: #16, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-API-06-C03 | 控制: V13.3 | 任务: #16, #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |
