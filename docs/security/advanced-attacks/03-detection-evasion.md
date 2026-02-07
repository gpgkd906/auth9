# 高级攻击 - 安全检测规避测试

**模块**: 高级攻击
**测试范围**: 安全检测系统有效性、规避技术、告警响应
**场景数**: 4
**风险等级**: 🟠 高
**OWASP ASVS**: V7.2, V11.1

---

## 背景知识

Auth9 实现了 `SecurityDetectionService`（`src/service/security_detection.rs`），包含以下检测机制：
- **暴力破解检测**: 同一账户 5 次失败登录 / 10 分钟 → HIGH 告警
- **密码喷洒检测**: 同一 IP 对 5+ 不同账户尝试 / 10 分钟 → CRITICAL 告警
- **新设备检测**: 新 IP + User-Agent 组合 → INFO 告警
- **不可能旅行检测**: GeoIP 距离 > 500km / 1 小时 → MEDIUM 告警

攻击者了解检测规则后可能尝试绕过，本文档测试检测系统的鲁棒性。

---

## 场景 1：暴力破解检测阈值边界测试

### 前置条件
- 测试账户
- 了解检测阈值（5 次 / 10 分钟）

### 攻击目标
验证检测阈值的精确性和边界条件

### 攻击步骤
1. 精确发送 4 次失败登录（阈值 - 1），验证不触发告警
2. 发送第 5 次，验证触发告警
3. 等待 10 分钟后再发送 4 次，验证窗口重置
4. 在窗口边界（第 9 分 59 秒到第 10 分 01 秒间）发送请求
5. 混合成功和失败登录，验证仅失败计数
6. 测试告警触发后是否有自动阻断

### 预期安全行为
- 4 次失败不触发告警（阈值严格）
- 第 5 次失败触发 HIGH 告警
- 滑动窗口正确重置
- 成功登录不影响失败计数
- 告警包含 IP、用户、时间等上下文

### 验证方法
```bash
# 清理之前的告警（如有此能力）
# 或在新窗口期开始测试

# 发送 4 次失败（不应触发）
for i in $(seq 1 4); do
  curl -s -o /dev/null \
    -X POST http://localhost:8080/api/v1/auth/token \
    -d "grant_type=password&client_id=auth9-portal&username=test@test.com&password=wrong$i"
  sleep 2
done

# 检查告警
ALERTS_BEFORE=$(curl -s -H "Authorization: Bearer $ADMIN_TOKEN" \
  "http://localhost:8080/api/v1/security-alerts?type=brute_force&severity=high" | jq '.total')
echo "Alerts after 4 attempts: $ALERTS_BEFORE"
# 预期: 0 (或之前的数量不变)

# 第 5 次失败（应触发）
curl -s -o /dev/null \
  -X POST http://localhost:8080/api/v1/auth/token \
  -d "grant_type=password&client_id=auth9-portal&username=test@test.com&password=wrong5"

ALERTS_AFTER=$(curl -s -H "Authorization: Bearer $ADMIN_TOKEN" \
  "http://localhost:8080/api/v1/security-alerts?type=brute_force&severity=high" | jq '.total')
echo "Alerts after 5 attempts: $ALERTS_AFTER"
# 预期: ALERTS_BEFORE + 1

# 验证告警详情
curl -s -H "Authorization: Bearer $ADMIN_TOKEN" \
  "http://localhost:8080/api/v1/security-alerts?type=brute_force&limit=1" | jq '.'
# 预期: 包含 source_ip, target_user, attempt_count, severity=HIGH
```

### 修复建议
- 使用滑动窗口（而非固定窗口）防止边界绕过
- 阈值可配置但设有合理范围
- 告警触发后考虑自动临时阻断（如 15 分钟锁定）
- 提供管理员手动解锁机制

---

## 场景 2：低速攻击规避

### 前置条件
- 了解检测时间窗口（10 分钟）

### 攻击目标
验证低频率攻击是否能绕过检测系统

### 攻击步骤
1. 每 3 分钟发送 1 次失败登录（低于 5 次 / 10 分钟阈值）
2. 持续 24 小时，累计 480 次
3. 每 2 分 01 秒发送 1 次（在 10 分钟内恰好不超过 5 次）
4. 从多个 IP 交替发送（绕过 IP 级检测）
5. 对多个不同账户缓慢尝试同一密码（低速密码喷洒）

### 预期安全行为
- 理想情况：支持多时间窗口聚合检测（10分钟 + 1小时 + 24小时）
- 至少：24 小时内累计 >50 次失败触发告警
- IP 聚合分析检测分布式低速攻击
- 全局异常模式识别

### 验证方法
```bash
# 低速暴力破解 - 每 3 分钟 1 次
for i in $(seq 1 20); do
  echo "Attempt $i at $(date)"
  curl -s -o /dev/null \
    -X POST http://localhost:8080/api/v1/auth/token \
    -d "grant_type=password&client_id=auth9-portal&username=test@test.com&password=slow-attack-$i"
  sleep 180
done

# 1 小时后检查告警
curl -s -H "Authorization: Bearer $ADMIN_TOKEN" \
  "http://localhost:8080/api/v1/security-alerts?limit=10" | jq '.'
# 观察: 是否有任何告警产生

# 低速密码喷洒 - 每个账户间隔 3 分钟
USERS=("user1@test.com" "user2@test.com" "user3@test.com" "user4@test.com" "user5@test.com"
       "user6@test.com" "user7@test.com" "user8@test.com" "user9@test.com" "user10@test.com")
for user in "${USERS[@]}"; do
  echo "Trying $user at $(date)"
  curl -s -o /dev/null \
    -X POST http://localhost:8080/api/v1/auth/token \
    -d "grant_type=password&client_id=auth9-portal&username=$user&password=common-password"
  sleep 180
done

# 检查是否检测到低速喷洒模式
curl -s -H "Authorization: Bearer $ADMIN_TOKEN" \
  "http://localhost:8080/api/v1/security-alerts?type=password_spray" | jq '.'
```

### 修复建议
- 实现多级时间窗口：10分钟（急性）+ 1小时（中期）+ 24小时（长期）
- 长期窗口阈值：24 小时内 > 50 次失败 → 告警
- 全局统计：所有账户的失败总数异常增长 → 告警
- 考虑集成威胁情报 IP 黑名单
- 异常模式机器学习检测（长期方案）

---

## 场景 3：分布式攻击规避

### 前置条件
- 多个不同 IP 地址（代理/VPN/Tor）
- 了解 IP 级检测机制

### 攻击目标
验证检测系统对分布式（多 IP）攻击的识别能力

### 攻击步骤
1. 从 10 个不同 IP 对同一账户各发送 4 次失败（总计 40 次，每个 IP 不超过阈值）
2. 使用 X-Forwarded-For 头伪造来源 IP
3. 混合 IPv4 和 IPv6 地址
4. 使用 Tor 出口节点 IP
5. 检查系统是否有跨 IP 聚合检测

### 预期安全行为
- 同一账户从多 IP 收到大量失败登录应触发告警（账户级聚合）
- X-Forwarded-For 仅从受信任代理接受
- 已知 Tor 出口节点可选标记
- 跨 IP 的失败模式被检测

### 验证方法
```bash
# 模拟多 IP 攻击（使用 X-Forwarded-For，假设不受信任会被忽略）
for ip_suffix in $(seq 1 10); do
  for attempt in $(seq 1 4); do
    curl -s -o /dev/null \
      -H "X-Forwarded-For: 192.168.1.$ip_suffix" \
      -X POST http://localhost:8080/api/v1/auth/token \
      -d "grant_type=password&client_id=auth9-portal&username=target@test.com&password=wrong"
  done
done
# 总计 40 次失败，但每个 IP 仅 4 次

# 验证 X-Forwarded-For 处理
# 如果服务端信任 X-Forwarded-For（应仅在反向代理后），每个 IP 4 次不触发
# 如果服务端看到真实 IP，则同一 IP 40 次会触发
curl -s -H "Authorization: Bearer $ADMIN_TOKEN" \
  "http://localhost:8080/api/v1/security-alerts?limit=10" | jq '.'

# 验证账户级聚合
# 即使从不同 IP，同一账户 40 次失败是否有告警
curl -s -H "Authorization: Bearer $ADMIN_TOKEN" \
  "http://localhost:8080/api/v1/security-alerts?details.target_user=target@test.com" | jq '.'
```

### 修复建议
- 实现账户级失败计数（不仅 IP 级）
- X-Forwarded-For 仅从配置的信任代理 IP 接受
- 多维度聚合：IP + 账户 + 时间窗口
- 已知恶意 IP 列表集成（Tor 出口、已知代理）
- 异常检测：短时间内某账户从多个不同地理位置收到登录尝试

---

## 场景 4：不可能旅行检测准确性

### 前置条件
- 支持 GeoIP 查询的环境
- 不同地理位置的 IP 地址

### 攻击目标
验证不可能旅行检测的准确性，减少误报和漏报

### 攻击步骤
1. 从北京 IP 成功登录
2. 5 分钟后从纽约 IP 登录（不可能旅行场景）
3. 从相邻城市 IP 登录（合理旅行，不应告警）
4. 使用 VPN IP（可能导致误报）
5. 使用 CDN IP（回源 IP 不同于用户 IP）
6. 检查 GeoIP 数据库缺失情况（某些 IP 无地理信息）

### 预期安全行为
- 500km / 1 小时规则正确触发 MEDIUM 告警
- 合理旅行距离不触发告警
- GeoIP 查询失败时降级（不阻断，仅记录）
- 告警包含地理位置详情

### 验证方法
```bash
# 模拟北京登录
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -H "X-Forwarded-For: 123.123.123.123" \
  -d "grant_type=password&client_id=auth9-portal&username=travel@test.com&password=correct"
# 假设此 IP 解析为北京

# 等待 5 分钟后模拟纽约登录
sleep 300
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -H "X-Forwarded-For: 74.125.224.72" \
  -d "grant_type=password&client_id=auth9-portal&username=travel@test.com&password=correct"
# 假设此 IP 解析为纽约

# 检查不可能旅行告警
curl -s -H "Authorization: Bearer $ADMIN_TOKEN" \
  "http://localhost:8080/api/v1/security-alerts?type=impossible_travel" | jq '.'
# 预期: MEDIUM 告警，包含两个位置和距离

# 检查无 GeoIP 数据的 IP
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -H "X-Forwarded-For: 10.0.0.1" \
  -d "grant_type=password&client_id=auth9-portal&username=travel@test.com&password=correct"
# 预期: 正常登录，GeoIP 查询失败不阻断

# 同城市 IP 切换（不应触发）
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -H "X-Forwarded-For: 123.123.123.124" \
  -d "grant_type=password&client_id=auth9-portal&username=travel@test.com&password=correct"
# 预期: 无告警（同城市 IP 段）
```

### 修复建议
- GeoIP 数据库定期更新（GeoLite2 每月更新）
- 考虑 VPN/CDN IP 的白名单机制
- 速度阈值可配置（默认 500km/h，可根据业务调整）
- GeoIP 查询失败时降级为"未知位置"，不阻断但记录
- 用户可标记"我在旅行"临时放宽检测

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | 暴力破解检测阈值边界测试 | ☐ | | | |
| 2 | 低速攻击规避 | ☐ | | | |
| 3 | 分布式攻击规避 | ☐ | | | |
| 4 | 不可能旅行检测准确性 | ☐ | | | |

---

## 参考资料

- [OWASP Credential Stuffing Prevention](https://cheatsheetseries.owasp.org/cheatsheets/Credential_Stuffing_Prevention_Cheat_Sheet.html)
- [CWE-307: Improper Restriction of Excessive Authentication Attempts](https://cwe.mitre.org/data/definitions/307.html)
- [MITRE ATT&CK T1110: Brute Force](https://attack.mitre.org/techniques/T1110/)
- [MITRE ATT&CK T1078: Valid Accounts](https://attack.mitre.org/techniques/T1078/)
