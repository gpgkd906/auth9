# 授权安全 - ABAC 策略治理与执行安全测试

**模块**: 授权安全
**测试范围**: ABAC 策略管理接口、发布流程、跨租户隔离、模拟接口安全
**场景数**: 5
**风险等级**: 🔴 极高
**OWASP ASVS 5.0**: V8.1,V8.2,V8.3,V13.1

---

## 背景知识

Auth9 授权链路已升级为：

1. RBAC 基线授权
2. ABAC 条件评估（`disabled/shadow/enforce`）

ABAC 管理面接口为租户级高敏感能力，若被越权调用将造成授权策略被篡改，影响面覆盖整租户。

---

## 场景 1：非管理员越权创建 ABAC 草稿

### 前置条件
- 普通成员 `TENANT_MEMBER_TOKEN`（无 `abac:*`/`rbac:*`）
- 目标租户 `TENANT_ID`

### 攻击目标
验证普通成员无法创建策略草稿。

### 攻击步骤
1. 使用普通成员 token 调用 `POST /api/v1/tenants/{TENANT_ID}/abac/policies`
2. 提交合法 `policy` JSON

### 预期安全行为
- 返回 `403 Forbidden`
- `abac_policy_set_versions` 不新增记录

### 验证方法
```bash
curl -i -X POST "http://localhost:8080/api/v1/tenants/{TENANT_ID}/abac/policies" \
  -H "Authorization: Bearer {TENANT_MEMBER_TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{"change_note":"attack","policy":{"rules":[]}}'
```

---

## 场景 2：跨租户策略篡改

### 前置条件
- 攻击者持有 `TENANT_A` 管理员 token
- 目标为 `TENANT_B`

### 攻击目标
验证 token 不能操作非本租户 ABAC 策略。

### 攻击步骤
1. 使用 `TENANT_A_ADMIN_TOKEN` 调用 `TENANT_B` 的 ABAC 列表/创建/发布接口

### 预期安全行为
- 全部返回 `403 Forbidden`
- `TENANT_B` 的策略记录不变

### 验证方法
```bash
curl -i -X GET "http://localhost:8080/api/v1/tenants/{TENANT_B}/abac/policies" \
  -H "Authorization: Bearer {TENANT_A_ADMIN_TOKEN}"
```

---

## 场景 3：发布/回滚流程篡改与状态一致性

### 前置条件
- 同租户存在多个版本（v1/v2）
- 当前发布版本为 v2

### 攻击目标
验证发布与回滚不会产生双 `published` 状态或悬挂引用。

### 攻击步骤
1. 对 v1 执行 rollback
2. 立即对 v2 执行 publish（并发/连续操作）
3. 重复查询版本状态

### 预期安全行为
- 任意时刻仅 1 个 `published`
- `abac_policy_sets.published_version_id` 与版本表状态一致

### 验证方法
```sql
SELECT status, COUNT(*) c
FROM abac_policy_set_versions
WHERE policy_set_id = '{policy_set_id}'
GROUP BY status;
-- 预期: published 的 c <= 1
```

---

## 场景 4：恶意策略 JSON 注入与解析鲁棒性

### 前置条件
- 管理员 token

### 攻击目标
验证策略 JSON 的结构校验能拒绝畸形输入，防止解释器崩溃或绕过。

### 攻击步骤
1. 提交非对象策略（如数组、字符串）
2. 提交不存在字段/非法操作符
3. 提交超深嵌套 JSON

### 预期安全行为
- 返回 `400/422`，不写入无效策略
- 服务不 panic，不出现 500

### 验证方法
```bash
curl -i -X POST "http://localhost:8080/api/v1/tenants/{TENANT_ID}/abac/policies" \
  -H "Authorization: Bearer {TENANT_ADMIN_TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{"policy":"invalid"}'
```

---

## 场景 5：simulate 接口滥用与信息泄露

### 前置条件
- 两类 token：管理员 token、普通成员 token

### 攻击目标
验证 `simulate` 不向低权限用户泄露策略命中细节。

### 攻击步骤
1. 普通成员调用 `POST /api/v1/tenants/{tenant_id}/abac/simulate`
2. 管理员调用同接口并对比返回字段

### 预期安全行为
- 普通成员返回 `403`
- 仅管理员可看到 `matched_allow_rule_ids/matched_deny_rule_ids`

### 验证方法
```bash
curl -i -X POST "http://localhost:8080/api/v1/tenants/{TENANT_ID}/abac/simulate" \
  -H "Authorization: Bearer {TENANT_MEMBER_TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{"simulation":{"action":"user_manage","resource_type":"tenant"}}'
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 非管理员越权创建 ABAC 草稿 | ☐ | | | |
| 2 | 跨租户策略篡改 | ☐ | | | |
| 3 | 发布/回滚流程篡改与状态一致性 | ☐ | | | |
| 4 | 恶意策略 JSON 注入与解析鲁棒性 | ☐ | | | |
| 5 | simulate 接口滥用与信息泄露 | ☐ | | | |
