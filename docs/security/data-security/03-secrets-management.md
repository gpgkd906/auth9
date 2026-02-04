# 数据安全 - 密钥管理安全测试

**模块**: 数据安全
**测试范围**: 密钥存储、轮换和访问控制
**场景数**: 4
**风险等级**: 🔴 极高

---

## 背景知识

Auth9 密钥类型：
- **JWT 签名密钥**: RS256 私钥/公钥
- **数据库凭证**: TiDB 连接密码
- **Redis 密码**: 缓存服务认证
- **Keycloak 凭证**: Admin API 访问
- **SMTP 凭证**: 邮件服务认证
- **Client Secret**: OIDC 客户端密钥

存储位置：
- 环境变量
- Kubernetes Secrets
- 配置文件 (不推荐)

---

## 场景 1：密钥存储安全

### 前置条件
- 部署环境访问权限

### 攻击目标
验证密钥是否安全存储

### 攻击步骤
1. 检查各种可能的密钥位置：
   - 代码仓库
   - 配置文件
   - 环境变量
   - Docker 镜像
2. 检查版本控制历史
3. 检查日志文件

### 预期安全行为
- 密钥不在代码中
- 配置文件不含密钥
- 使用 Secret 管理服务

### 验证方法
```bash
# 代码仓库搜索
git log -p | grep -i "password\|secret\|key\|token" | head -50
grep -r "password\s*=" --include="*.rs" --include="*.ts" src/
grep -r "sk_live\|pk_live" .  # API Key 模式

# 检查配置文件
cat config/default.yaml | grep -i password
cat .env.example  # 检查是否有真实密钥

# Docker 镜像检查
docker history auth9-core:latest
docker run --rm auth9-core:latest env | grep -i secret

# .git 目录泄露
curl http://localhost:8080/.git/config
curl http://localhost:3000/.git/config

# 检查 K8s Secrets (需要权限)
kubectl get secrets -n auth9
kubectl describe secret auth9-secrets -n auth9
```

### 修复建议
- 使用 K8s Secrets 或 HashiCorp Vault
- 添加 pre-commit 钩子扫描
- .gitignore 排除敏感文件
- 定期审计代码历史

---

## 场景 2：密钥轮换机制

### 前置条件
- 了解密钥轮换流程

### 攻击目标
验证密钥轮换机制是否存在

### 攻击步骤
1. 检查密钥是否有过期时间
2. 测试轮换过程：
   - 旧密钥是否立即失效
   - 是否支持平滑过渡
3. 检查轮换日志/审计

### 预期安全行为
- JWT 密钥支持轮换
- Client Secret 可重新生成
- 密钥轮换有审计日志

### 验证方法
```bash
# 检查 JWKS 是否支持多密钥
curl http://localhost:8080/.well-known/jwks.json | jq '.keys | length'
# > 1 表示支持密钥轮换过渡

# 检查 JWT kid (Key ID)
echo $TOKEN | cut -d'.' -f1 | base64 -d | jq .kid

# 测试 Client Secret 轮换
curl -X POST -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:8080/api/v1/services/{id}/clients/{client_id}/regenerate-secret
# 检查旧 secret 是否立即失效

# 检查审计日志
SELECT * FROM audit_logs WHERE action LIKE '%secret%' OR action LIKE '%key%';
```

### 修复建议
- JWKS 支持多 kid
- 设置密钥最大有效期
- 自动化轮换流程
- 轮换操作审计日志

---

## 场景 3：密钥访问控制

### 前置条件
- 不同权限级别账户

### 攻击目标
验证密钥访问是否有适当权限控制

### 攻击步骤
1. 尝试以低权限用户访问密钥：
   - 系统配置 (含 SMTP 密码)
   - Client Secret
   - API Key
2. 检查密钥操作的权限要求
3. 检查密钥是否可批量导出

### 预期安全行为
- 仅管理员可访问系统密钥
- Client Secret 仅服务所有者可管理
- 禁止批量导出密钥

### 验证方法
```bash
# 普通用户尝试访问系统配置
curl -H "Authorization: Bearer $USER_TOKEN" \
  http://localhost:8080/api/v1/system/email
# 预期: 403

# 尝试访问其他租户的 Client Secret
curl -H "Authorization: Bearer $TOKEN_TENANT_A" \
  http://localhost:8080/api/v1/services/{tenant_b_service}/clients/{client_id}
# 预期: 403 或 404

# 批量导出尝试
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:8080/api/v1/clients/export
# 预期: 不存在此端点或不含 secret

# 检查审计日志
# 所有密钥访问都应记录
```

### 修复建议
- 最小权限原则
- 密钥访问需要额外认证
- 禁止批量导出
- 所有访问记录审计

---

## 场景 4：密钥泄露检测

### 前置条件
- 监控系统访问

### 攻击目标
验证是否有密钥泄露检测机制

### 攻击步骤
1. 模拟密钥泄露场景：
   - 公开暴露 API Key
   - 异常使用模式
2. 检查告警机制
3. 检查自动吊销功能

### 预期安全行为
- 检测异常使用模式
- 自动告警
- 支持紧急吊销

### 验证方法
```bash
# 模拟异常使用
# 1. 从多个 IP 快速使用同一 API Key
for i in {1..100}; do
  curl -H "X-API-Key: $API_KEY" \
    -H "X-Forwarded-For: 192.168.1.$i" \
    http://localhost:8080/api/v1/users
done

# 检查是否触发告警
# 查看监控/日志

# 测试紧急吊销
curl -X POST -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:8080/api/v1/api-keys/{key_id}/revoke

# 验证吊销生效
curl -H "X-API-Key: $REVOKED_KEY" \
  http://localhost:8080/api/v1/users
# 预期: 401

# 检查是否通知用户
# 检查邮件/通知
```

### 修复建议
- 实现异常检测
- 集成安全告警系统
- 支持一键吊销
- 泄露后自动通知

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | 密钥存储安全 | ☐ | | | |
| 2 | 密钥轮换机制 | ☐ | | | |
| 3 | 密钥访问控制 | ☐ | | | |
| 4 | 密钥泄露检测 | ☐ | | | |

---

## 密钥清单与轮换周期

| 密钥类型 | 存储位置 | 推荐轮换周期 | 轮换方式 |
|---------|---------|-------------|---------|
| JWT 签名密钥 | K8s Secret | 90 天 | 添加新 kid，逐步废弃旧 |
| 数据库密码 | K8s Secret | 90 天 | 更新 Secret + 重启服务 |
| Redis 密码 | K8s Secret | 90 天 | 更新 Secret + 重启服务 |
| Keycloak Admin | K8s Secret | 90 天 | 更新配置 |
| SMTP 密码 | 数据库 (加密) | 按需 | Admin 手动更新 |
| Client Secret | 数据库 (哈希) | 按需 | 用户自助重新生成 |
| API Key | 数据库 (哈希) | 按需 | 用户自助重新生成 |

---

## 密钥扫描工具

```bash
# truffleHog - Git 历史扫描
trufflehog git file://. --since-commit HEAD~100

# gitleaks
gitleaks detect --source=. --verbose

# detect-secrets (pre-commit)
detect-secrets scan

# AWS git-secrets
git secrets --scan
```

---

## 参考资料

- [OWASP Key Management](https://cheatsheetseries.owasp.org/cheatsheets/Key_Management_Cheat_Sheet.html)
- [HashiCorp Vault Best Practices](https://www.vaultproject.io/docs/concepts/seal)
- [CWE-321: Hard-coded Cryptographic Key](https://cwe.mitre.org/data/definitions/321.html)
- [CWE-798: Hard-coded Credentials](https://cwe.mitre.org/data/definitions/798.html)
