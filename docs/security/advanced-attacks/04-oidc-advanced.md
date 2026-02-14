# 高级攻击 - OIDC 高级攻击测试

**模块**: 高级攻击
**测试范围**: Token 混淆、IdP 混淆、Client 凭证泄露利用
**场景数**: 3
**风险等级**: 🟠 高
**OWASP ASVS**: V2.2, V3.5, V3.6

---

## 背景知识

Auth9 使用 Keycloak 作为 OIDC Provider，签发三类 Token：
- **Identity Token** (ID Token): 用户身份信息，用于 Auth9 Portal
- **Tenant Access Token**: 由 Token Exchange 签发，包含角色/权限
- **Refresh Token**: 用于更新 Access Token

Token 混淆攻击利用不同类型 Token 之间的差异，尝试跨场景使用 Token。
Auth9 也支持多个 Identity Provider（社交登录），IdP 混淆攻击利用多 IdP 信任关系。

---

## 场景 1：Token 类型混淆攻击

### 前置条件
- 有效的 Identity Token
- 有效的 Tenant Access Token
- 有效的 Refresh Token

### 攻击目标
验证系统是否正确区分不同类型的 Token，防止跨类型使用

### 攻击步骤
1. 使用 Identity Token 访问需要 Tenant Access Token 的端点
2. 使用 Tenant Access Token 作为 Identity Token 调用 Token Exchange
3. 使用 Refresh Token 作为 Access Token 访问 API
4. 使用 Keycloak 原始 Access Token（非 Auth9 签发）访问 Auth9 API
5. 检查每种 Token 的 `typ` 或 `token_type` claim 是否被验证

### 预期安全行为
- Identity Token 不能访问需要权限检查的端点
- Tenant Access Token 不能作为 Identity Token 使用
- Refresh Token 仅能用于 Token 刷新端点
- Keycloak 原始 Token 不被 Auth9 API 直接接受
- 每个端点验证 Token 类型

### 验证方法
```bash
# 获取各类 Token
IDENTITY_TOKEN="..."   # 从 /api/v1/auth/callback 获取
TENANT_TOKEN="..."     # 从 gRPC ExchangeToken 获取
REFRESH_TOKEN="..."    # 从 /api/v1/auth/token 获取

# Identity Token 访问权限端点（应失败）
curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer $IDENTITY_TOKEN" \
  http://localhost:8080/api/v1/roles
# 预期: 403 (Identity Token 无 tenant 权限)

# Tenant Token 调用 Token Exchange（应失败）
grpcurl -plaintext \
  -H "x-api-key: $API_KEY" \
  -d '{"identity_token": "'$TENANT_TOKEN'", "tenant_id": "'$TENANT_ID'"}' \
  localhost:50051 auth9.TokenService/ExchangeToken
# 预期: UNAUTHENTICATED - Not an identity token

# Refresh Token 访问 API（应失败）
curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer $REFRESH_TOKEN" \
  http://localhost:8080/api/v1/auth/userinfo
# 预期: 401

# Keycloak 原始 Token 访问 Auth9 API
KC_TOKEN=$(curl -s -X POST \
  "http://localhost:8081/realms/auth9/protocol/openid-connect/token" \
  -d "grant_type=password&client_id=auth9-portal&username=test@test.com&password=TestUser123!" \
  | jq -r '.access_token')
curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer $KC_TOKEN" \
  http://localhost:8080/api/v1/tenants
# 预期: 401 (issuer 或 audience 不匹配)

# 解码各类 Token 比较结构
for token_name in IDENTITY TENANT REFRESH; do
  eval token=\$${token_name}_TOKEN
  echo "=== $token_name ==="
  echo $token | cut -d. -f2 | base64 -d 2>/dev/null | jq '{typ, iss, aud, token_type}' 2>/dev/null
done
```

### 修复建议
- 在 Token 中包含 `token_type` claim 区分类型
- 验证中间件根据端点要求的 Token 类型进行检查
- Identity Token 和 Tenant Access Token 使用不同的 audience
- Refresh Token 使用不同的签名密钥或标记

---

## 场景 2：IdP 混淆与账户劫持

### 前置条件
- 系统配置了多个 Identity Provider（如 Google + GitHub）
- 攻击者控制一个恶意 IdP

### 攻击目标
验证系统是否能防止通过恶意 IdP 劫持其他用户账户

### 攻击步骤
1. 用户 A 使用 Google 登录（邮箱 user@example.com）
2. 攻击者配置恶意 IdP，声称邮箱也是 user@example.com
3. 攻击者通过恶意 IdP 登录
4. 检查系统是否基于邮箱自动关联到用户 A 的账户
5. 测试不同 IdP 使用相同 `sub` 值是否冲突
6. 测试 IdP 返回的邮箱未验证时的处理

### 预期安全行为
- 不同 IdP 的账户通过 `(provider, provider_user_id)` 唯一标识
- 不仅基于邮箱自动关联账户
- 新 IdP 登录创建新账户或要求手动关联
- IdP 返回的 `email_verified: false` 时不信任邮箱
- 账户关联需要用户确认

### 验证方法
```bash
# 检查 linked_identities 表结构
# 确认使用 (provider, provider_user_id) 作为唯一标识

# 使用 Google 登录创建账户
# 1. 通过 /api/v1/auth/authorize?provider=google 登录
# 2. 记录创建的用户 ID

# 使用 GitHub（相同邮箱）登录
# 3. 通过 /api/v1/auth/authorize?provider=github 登录
# 4. 检查是否创建了新用户还是关联到已有用户

# 验证关联逻辑
curl -s -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/users/me
# 检查 linked_identities 字段

# 检查 email_verified 处理
# 如果 IdP 返回未验证邮箱，系统是否仍信任
```

### 修复建议
- 账户关联使用 `(provider, provider_user_id)` 复合键
- 自动关联仅限 `email_verified: true` 的情况
- 首次关联不同 IdP 时需用户确认
- 禁止自行配置不受信任的 IdP（仅管理员可配置）
- 定期审计 IdP 配置

---

## 场景 3：Client Credentials 泄露利用

### 前置条件
- 客户端 client_id 和 client_secret
- `/api/v1/auth/token` 端点

### 攻击目标
验证 Client Credentials 泄露后的影响范围和缓解措施

### 攻击步骤
1. 使用泄露的 client_id + client_secret 请求 Token
2. 检查通过 client_credentials 获取的 Token 权限范围
3. 尝试使用 client_credentials Token 进行管理操作
4. 测试 client_secret 轮转是否立即生效
5. 检查旧 secret 是否在轮转后仍可用

### 预期安全行为
- client_credentials 获取的 Token 权限受限
- 不能通过 client_credentials 获取用户级别权限
- Secret 轮转后旧 secret 立即失效
- client_credentials 使用有审计日志
- 异常使用模式触发告警

### 验证方法
```bash
# 使用 client_credentials 获取 Token
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -d "grant_type=client_credentials" \
  -d "client_id=auth9-portal" \
  -d "client_secret=$CLIENT_SECRET"
# 检查返回 Token 的 scope 和权限

# 检查 Token 权限范围
echo $CC_TOKEN | cut -d. -f2 | base64 -d | jq '{scope, roles, permissions}'
# 预期: 权限受限，不包含用户管理权限

# 尝试管理操作
curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer $CC_TOKEN" \
  http://localhost:8080/api/v1/users
# 预期: 403

# 轮转 Secret (需要 service_id 和 client_id)
NEW_SECRET=$(curl -s -X POST -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:8080/api/v1/services/$SERVICE_ID/clients/$CLIENT_ID/regenerate-secret | jq -r '.data.client_secret')

# 旧 Secret 应失效
curl -s -o /dev/null -w "%{http_code}" \
  -X POST http://localhost:8080/api/v1/auth/token \
  -d "grant_type=client_credentials&client_id=auth9-portal&client_secret=$CLIENT_SECRET"
# 预期: 401

# 新 Secret 应生效
curl -s -o /dev/null -w "%{http_code}" \
  -X POST http://localhost:8080/api/v1/auth/token \
  -d "grant_type=client_credentials&client_id=auth9-portal&client_secret=$NEW_SECRET"
# 预期: 200
```

### 修复建议
- client_credentials 授权范围最小化
- Secret 轮转立即生效（数据库更新 + 缓存清除）
- client_credentials 使用的审计日志
- 异常客户端认证模式告警（新 IP、高频率等）
- 定期轮转 Client Secret（建议 90 天）

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | Token 类型混淆攻击 | ☐ | | | |
| 2 | IdP 混淆与账户劫持 | ☐ | | | |
| 3 | Client Credentials 泄露利用 | ☐ | | | |

---

## 参考资料

- [RFC 8693 - OAuth 2.0 Token Exchange](https://datatracker.ietf.org/doc/html/rfc8693)
- [OAuth 2.0 Mix-Up Attacks](https://datatracker.ietf.org/doc/html/draft-ietf-oauth-mix-up-mitigation)
- [CWE-287: Improper Authentication](https://cwe.mitre.org/data/definitions/287.html)
- [OWASP OAuth Security](https://cheatsheetseries.owasp.org/cheatsheets/OAuth2_Cheat_Sheet.html)
