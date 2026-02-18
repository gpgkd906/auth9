# 数据安全 - 加密实现安全测试

**模块**: 数据安全
**测试范围**: AES-256-GCM 实现、Nonce 安全、密文完整性
**场景数**: 3
**风险等级**: 🟠 高
**ASVS 5.0 矩阵ID**: M-DATA-04
**OWASP ASVS 5.0**: V11.1,V11.2,V11.4
**回归任务映射**: Backlog #20


---

## 背景知识

Auth9 使用 AES-256-GCM 加密敏感系统设置（`src/crypto/aes.rs`）：
- **加密对象**: SMTP 密码、API Key、第三方服务凭证
- **算法**: AES-256-GCM（认证加密，提供机密性 + 完整性）
- **Nonce**: 每次加密生成 12 字节随机 nonce
- **格式**: `base64(nonce):base64(ciphertext+tag)`
- **密钥来源**: `SETTINGS_ENCRYPTION_KEY` 环境变量（Base64 编码的 32 字节密钥）

GCM 模式的安全性依赖于 nonce 不重复，密钥强度足够。

---

## 场景 1：Nonce 重用检测

### 前置条件
- 能够触发多次加密操作
- 访问加密后的密文输出

### 攻击目标
验证每次加密是否使用唯一的 nonce，nonce 重用会彻底破坏 GCM 安全性

### 攻击步骤
1. 多次更新同一个系统设置（如 SMTP 密码），触发加密
2. 收集每次加密的密文
3. 提取 nonce 部分（密文格式中的第一段）
4. 比较所有 nonce 是否唯一
5. 分析 nonce 生成是否使用 CSPRNG

### 预期安全行为
- 每次加密产生不同的 nonce
- 相同明文加密后产生不同密文
- Nonce 使用 CSPRNG 生成（`OsRng` 或 `thread_rng`）
- 12 字节 nonce 提供 96 位随机空间

### 验证方法
```bash
# 多次更新同一设置，收集密文
CIPHERTEXTS=()
for i in $(seq 1 20); do
  RESULT=$(curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    http://localhost:8080/api/v1/system/settings \
    -d '{"smtp_password": "TestPassword123!"}')
  CIPHERTEXTS+=("$RESULT")
done

# 如果能从数据库直接读取加密值
# SELECT encrypted_value FROM system_settings WHERE key = 'smtp_password';
# 比较 20 次加密的 nonce 前缀

# 分析 nonce 唯一性的 Python 脚本
python3 << 'PYEOF'
import base64

# 假设从数据库收集的密文列表
ciphertexts = [
    # "base64(nonce):base64(ciphertext)" 格式
    # 填入实际收集的数据
]

nonces = set()
for ct in ciphertexts:
    nonce_b64, _ = ct.split(':')
    nonce = base64.b64decode(nonce_b64)
    print(f"Nonce (hex): {nonce.hex()}, length: {len(nonce)} bytes")

    if nonce.hex() in nonces:
        print(f"!!! NONCE REUSE DETECTED: {nonce.hex()}")
    nonces.add(nonce.hex())

print(f"\nTotal: {len(ciphertexts)}, Unique nonces: {len(nonces)}")
assert len(nonces) == len(ciphertexts), "CRITICAL: Nonce reuse detected!"
PYEOF
```

### 修复建议
- 确保使用 `OsRng` 或 `rand::thread_rng()` 生成 nonce（已实现）
- 考虑使用 AES-256-GCM-SIV（对 nonce 重用更安全的变体）
- 如果加密次数可能超过 2^32，考虑密钥轮转
- 单元测试验证 nonce 唯一性

---

## 场景 2：密文篡改与认证标签验证

### 前置条件
- 能够访问/修改数据库中的加密值
- 了解密文格式

### 攻击目标
验证 GCM 认证标签是否正确防止密文篡改

### 攻击步骤
1. 获取一个有效的加密值
2. 修改密文中的一个 bit（bit-flip 攻击）
3. 修改 nonce 部分
4. 截断密文（移除认证标签）
5. 替换为完全不同的密文
6. 检查解密时是否检测到篡改

### 预期安全行为
- 任何密文修改导致解密失败（GCM 认证错误）
- 不会返回部分解密的数据
- 错误信息不泄露密钥或明文信息
- 密文格式不正确时优雅处理

### 验证方法
```bash
# 此测试需要数据库访问来修改加密值
# 或通过 API 间接验证

# 获取原始加密值
ORIGINAL=$(mysql -h 127.0.0.1 -P 4000 -u root auth9 \
  -e "SELECT encrypted_value FROM system_settings WHERE setting_key = 'smtp_password'" \
  -sN)
echo "Original: $ORIGINAL"

# Bit-flip 攻击 - 修改密文中一个字符
TAMPERED=$(echo "$ORIGINAL" | python3 -c "
import sys
ct = sys.stdin.read().strip()
nonce, ciphertext = ct.split(':')
# 修改密文第一个字符
import base64
ct_bytes = bytearray(base64.b64decode(ciphertext))
ct_bytes[0] ^= 0x01  # flip one bit
tampered = nonce + ':' + base64.b64encode(bytes(ct_bytes)).decode()
print(tampered)
")

# 写入篡改后的值
mysql -h 127.0.0.1 -P 4000 -u root auth9 \
  -e "UPDATE system_settings SET encrypted_value = '$TAMPERED' WHERE setting_key = 'smtp_password'"

# 尝试读取（触发解密）
curl -s -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/system/settings
# 预期: 解密失败，返回错误或空值（不返回垃圾数据）

# 截断密文（移除 GCM tag）
TRUNCATED=$(echo "$ORIGINAL" | python3 -c "
import sys, base64
ct = sys.stdin.read().strip()
nonce, ciphertext = ct.split(':')
ct_bytes = base64.b64decode(ciphertext)
truncated = base64.b64encode(ct_bytes[:-16]).decode()  # 移除 16 字节 tag
print(nonce + ':' + truncated)
")
# 写入并测试，预期同样解密失败

# 恢复原始值
mysql -h 127.0.0.1 -P 4000 -u root auth9 \
  -e "UPDATE system_settings SET encrypted_value = '$ORIGINAL' WHERE setting_key = 'smtp_password'"
```

### 修复建议
- GCM 模式自带认证标签验证（已实现）
- 解密失败时记录告警（可能表示数据库被篡改）
- 不返回部分解密结果，失败时返回明确错误
- 考虑加密值的完整性校验（额外 HMAC 或数据库级校验和）

---

## 场景 3：加密密钥强度与管理

### 前置条件
- 了解密钥存储和配置方式
- 访问部署配置

### 攻击目标
验证加密密钥是否足够强壮且安全管理

### 攻击步骤
1. 检查 `SETTINGS_ENCRYPTION_KEY` 的实际长度是否为 32 字节（256 位）
2. 检查密钥是否硬编码在代码或配置文件中
3. 检查 Git 历史中是否存在泄露的密钥
4. 检查密钥是否使用 CSPRNG 生成
5. 验证应用启动时是否验证密钥长度
6. 测试使用弱密钥（如 `0000...`）是否被拒绝

### 预期安全行为
- 密钥长度必须为 32 字节（AES-256）
- 密钥来自环境变量或密钥管理系统（不在代码/配置文件中）
- 应用启动时验证密钥有效性
- Git 历史无密钥泄露
- 弱密钥或短密钥被拒绝

### 验证方法
```bash
# 检查代码中是否硬编码密钥
grep -r "SETTINGS_ENCRYPTION_KEY\|encryption_key" auth9-core/src/ \
  --include="*.rs" | grep -v "env\|config\|test"
# 预期: 仅在配置读取处引用环境变量

# 检查 Git 历史
git log -p --all -S "SETTINGS_ENCRYPTION_KEY" -- '*.toml' '*.yaml' '*.yml' '*.env'
# 预期: 无密钥明文

# 检查 Docker 配置
grep -r "SETTINGS_ENCRYPTION_KEY" docker-compose*.yml .env* Dockerfile*
# 预期: 使用变量引用，不包含明文

# 测试短密钥
SETTINGS_ENCRYPTION_KEY="dG9vLXNob3J0" cargo run  # "too-short" base64
# 预期: 启动失败，提示密钥长度不足

# 测试空密钥
SETTINGS_ENCRYPTION_KEY="" cargo run
# 预期: 启动失败或加密功能禁用

# 验证密钥长度
python3 -c "
import base64
key = base64.b64decode('YOUR_BASE64_KEY_HERE')
print(f'Key length: {len(key)} bytes ({len(key)*8} bits)')
assert len(key) == 32, f'Key must be 256 bits, got {len(key)*8} bits'
print('OK: Key is 256 bits')
"

# 检查密钥轮转机制
# 是否支持同时使用多个密钥（解密旧数据 + 加密新数据）
```

### 修复建议
- 应用启动时验证密钥长度为 32 字节
- 使用 K8s Secrets 或 HashiCorp Vault 管理密钥
- 实现密钥轮转机制（新密钥加密，旧密钥仅解密）
- 定期扫描 Git 历史和配置文件的密钥泄露
- 生成密钥时使用：`openssl rand -base64 32`

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | Nonce 重用检测 | ☐ | | | |
| 2 | 密文篡改与认证标签验证 | ☐ | | | |
| 3 | 加密密钥强度与管理 | ☐ | | | |

---

## 参考资料

- [NIST SP 800-38D: GCM](https://csrc.nist.gov/publications/detail/sp/800-38d/final)
- [CWE-323: Reusing a Nonce, Key Pair in Encryption](https://cwe.mitre.org/data/definitions/323.html)
- [CWE-326: Inadequate Encryption Strength](https://cwe.mitre.org/data/definitions/326.html)
- [CWE-321: Use of Hard-coded Cryptographic Key](https://cwe.mitre.org/data/definitions/321.html)
- [OWASP Cryptographic Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Cryptographic_Storage_Cheat_Sheet.html)

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-DATA-04  
**适用控制**: V11.1,V11.2,V11.4  
**关联任务**: Backlog #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 3

### 执行清单
- [ ] M-DATA-04-C01 | 控制: V11.1 | 任务: #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-DATA-04-C02 | 控制: V11.2 | 任务: #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-DATA-04-C03 | 控制: V11.4 | 任务: #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
