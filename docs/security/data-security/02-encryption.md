# 数据安全 - 加密安全测试

**模块**: 数据安全
**测试范围**: 数据加密与传输安全
**场景数**: 5
**风险等级**: 🟠 高
**ASVS 5.0 矩阵ID**: M-DATA-02
**OWASP ASVS 5.0**: V11.1,V11.2,V12.1,V14.3
**回归任务映射**: Backlog #20


---

## 背景知识

Auth9 加密场景：
- **传输加密**: HTTPS/TLS
- **存储加密**: 密码哈希、敏感配置加密
- **Token 签名**: JWT (RS256/ES256)

使用的加密算法：
- 密码: Argon2id (Keycloak)
- JWT: RS256 (RSA-SHA256)
- 配置加密: AES-256-GCM

---

## 场景 1：传输层加密 (TLS)

### 前置条件
- HTTPS 端点可访问
- SSL 测试工具

### 攻击目标
验证 TLS 配置是否安全

### 攻击步骤
1. 检查支持的 TLS 版本
2. 检查加密套件
3. 检查证书有效性
4. 测试降级攻击

### 预期安全行为
- 仅支持 TLS 1.2+
- 禁用弱加密套件
- 证书有效且匹配
- 支持 HSTS

### 验证方法
```bash
# 使用 nmap 检查 TLS
nmap --script ssl-enum-ciphers -p 443 auth9.example.com

# 使用 openssl
openssl s_client -connect auth9.example.com:443 -tls1_2
openssl s_client -connect auth9.example.com:443 -tls1_1
# TLS 1.1 应该失败

# 检查证书
openssl s_client -connect auth9.example.com:443 </dev/null 2>/dev/null | \
  openssl x509 -text -noout

# 使用 testssl.sh
./testssl.sh auth9.example.com

# SSL Labs 在线测试
# https://www.ssllabs.com/ssltest/
```

### 修复建议
- 禁用 TLS 1.0/1.1
- 移除弱加密套件 (RC4, DES, 3DES)
- 使用 ECDHE 密钥交换
- 启用 HSTS

---

## 场景 2：密码哈希强度

### 前置条件
- 数据库访问权限 (测试环境)

### 攻击目标
验证密码存储的安全性

### 攻击步骤
1. 获取存储的密码哈希格式
2. 分析哈希算法和参数
3. 评估暴力破解难度
4. 检查盐值使用

### 预期安全行为
- 使用 Argon2id 或 bcrypt
- 足够的工作因子
- 每个密码独立盐值

### 验证方法
```sql
-- 在 Keycloak 数据库中检查
-- (Keycloak 使用自己的密码存储)
SELECT credential_data FROM credential WHERE user_id = 'xxx';

-- 分析返回的 JSON
-- 期望格式 (PBKDF2):
-- {"hashIterations":210000,"algorithm":"pbkdf2-sha512"}
-- 或 (Argon2):
-- {"algorithm":"argon2","memory":65536,"iterations":3,"parallelism":4}
```

```bash
# 使用 hashcat 评估强度
# 如果能在合理时间内破解，说明参数太弱

# 检查 Keycloak 配置
# Realm Settings -> Security Defenses -> Password Policy
```

### 修复建议
- Argon2id: memory=65536KB, iterations=3, parallelism=4
- bcrypt: cost=12+
- PBKDF2: 210,000+ 迭代
- 定期评估并升级参数

---

## 场景 3：JWT 签名安全

### 前置条件
- 获取有效的 JWT Token

### 攻击目标
验证 JWT 签名的安全性

### 攻击步骤
1. 解析 JWT 结构
2. 检查签名算法
3. 尝试算法混淆攻击
4. 检查密钥强度

### 预期安全行为
- 使用非对称签名 (RS256/ES256)
- 验证时指定算法
- 密钥长度足够

### 验证方法
```bash
# 解析 JWT
echo $TOKEN | cut -d'.' -f1 | base64 -d
# 检查 alg 字段

# 获取公钥
curl http://localhost:8080/.well-known/jwks.json | jq .

# 检查密钥长度
# RSA 应该 >= 2048 位
# ECDSA 应该使用 P-256 或更强

# 验证签名
# 使用 jwt.io 或 jose 库验证

# 尝试 alg:none 攻击
# 构造无签名的 Token 并测试
```

### 修复建议
- 使用 RS256 或 ES256
- RSA 密钥至少 2048 位
- 验证时固定算法
- 定期轮换密钥

---

## 场景 4：敏感配置加密

### 前置条件
- 数据库访问权限
- **`SETTINGS_ENCRYPTION_KEY` 环境变量已设置**（Docker dev 默认未设置，此时敏感字段以明文存储，这是设计行为）

### 攻击目标
验证敏感配置的加密存储（**仅在 `SETTINGS_ENCRYPTION_KEY` 已配置时有效**）

### 攻击步骤
1. 检查数据库中的敏感配置
2. 分析加密方式
3. 检查密钥管理
4. 评估加密强度

### 预期安全行为
- SMTP 密码等加密存储
- 使用 AES-256-GCM
- 密钥安全存储

### 验证方法
```sql
-- 检查系统设置表
SELECT setting_key, value FROM system_settings WHERE category = 'email';

-- 检查 client_secret_hash
SELECT client_id, client_secret_hash FROM clients;
-- 应该是哈希值，不是明文

-- 检查是否有明文存储
SELECT * FROM system_settings WHERE value LIKE '%password%';
```

```bash
# 检查环境变量中的密钥
env | grep -i key
env | grep -i secret
# 应该是加密的或来自安全存储

# 检查配置文件
cat /app/config.yaml
# 敏感值应该是环境变量引用
```

### 常见误报

| 症状 | 原因 | 解决方法 |
|------|------|---------|
| SMTP 密码明文存储 | `SETTINGS_ENCRYPTION_KEY` 未设置 | 设置环境变量后重新保存邮件配置：`export SETTINGS_ENCRYPTION_KEY=$(openssl rand -base64 32)` |
| Docker dev 环境中明文 | Docker dev 默认不配置加密密钥 | 这是设计行为：dev 环境可选加密，生产环境必须启用 |
| API 返回 `***` 但 DB 明文 | API 层始终 mask，与数据库加密独立 | 检查 `encrypted` 列是否为 `true`；若为 `false` 则表示未加密 |

### 修复建议
- 使用 AES-256-GCM 加密
- 加密密钥存储在 K8s Secrets 或 Vault
- 实现密钥轮换
- 审计加密密钥访问

---

## 场景 5：随机数生成安全

### 前置条件
- 代码审查权限或黑盒测试

### 攻击目标
验证随机数生成的安全性

### 攻击步骤
1. 分析需要随机数的场景：
   - Session ID
   - CSRF Token
   - 密码重置 Token
   - API Key
2. 检查随机性和熵
3. 尝试预测下一个值

### 预期安全行为
- 使用 CSPRNG
- 足够的位数 (>= 128 位)
- 不可预测

### 验证方法
```bash
# 获取多个 Token 分析
for i in {1..10}; do
  curl -X POST http://localhost:8080/api/v1/auth/forgot-password \
    -d '{"email":"test@example.com"}'
  # 从邮件或数据库获取 Token
done

# 分析 Token 格式和熵
# 1. 长度是否足够 (>= 32 字符)
# 2. 是否包含完整字符集
# 3. 是否有可预测的模式

# 检查 Session ID
curl -c - http://localhost:3000/login
# 分析 Cookie 中的 session ID

# 代码审查
# 查找 rand() 而非 crypto_rand() 的使用
grep -r "rand()" src/
grep -r "Math.random()" app/
```

### 修复建议
- 使用系统 CSPRNG
- Rust: `rand::thread_rng()` with `OsRng`
- Node: `crypto.randomBytes()`
- 最少 128 位熵
- 避免时间戳作为种子

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | 传输层加密 (TLS) | ☐ | | | |
| 2 | 密码哈希强度 | ☐ | | | |
| 3 | JWT 签名安全 | ☐ | | | |
| 4 | 敏感配置加密 | ☐ | | | |
| 5 | 随机数生成安全 | ☐ | | | |

---

## 推荐加密配置

| 场景 | 算法 | 参数 |
|-----|------|------|
| 密码哈希 | Argon2id | m=64MB, t=3, p=4 |
| JWT 签名 | RS256 | RSA 2048+ bits |
| 配置加密 | AES-256-GCM | 256-bit key |
| TLS | TLS 1.3 | ECDHE + AES-GCM |
| 随机数 | CSPRNG | 128+ bits |

---

## 参考资料

- [OWASP Cryptographic Storage](https://cheatsheetseries.owasp.org/cheatsheets/Cryptographic_Storage_Cheat_Sheet.html)
- [OWASP Password Storage](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
- [CWE-327: Broken Crypto Algorithm](https://cwe.mitre.org/data/definitions/327.html)
- [CWE-330: Insufficient Randomness](https://cwe.mitre.org/data/definitions/330.html)

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-DATA-02  
**适用控制**: V11.1,V11.2,V12.1,V14.3  
**关联任务**: Backlog #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 5

### 执行清单
- [ ] M-DATA-02-C01 | 控制: V11.1 | 任务: #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-DATA-02-C02 | 控制: V11.2 | 任务: #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-DATA-02-C03 | 控制: V12.1 | 任务: #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-DATA-02-C04 | 控制: V14.3 | 任务: #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
