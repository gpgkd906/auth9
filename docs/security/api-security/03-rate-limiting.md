# API 安全 - 限流与 DoS 防护测试

**模块**: API 安全
**测试范围**: 速率限制、资源耗尽防护
**场景数**: 4
**风险等级**: 🟠 高
**ASVS 5.0 矩阵ID**: M-API-03
**OWASP ASVS 5.0**: V4.4,V13.3,V2.5
**回归任务映射**: Backlog #20

---

## 背景知识

DoS 攻击向量：
- **请求洪水**: 大量请求耗尽服务器资源
- **慢速攻击**: Slowloris 等长连接攻击
- **资源耗尽**: CPU、内存、数据库连接
- **业务逻辑滥用**: 昂贵操作的滥用

Auth9 需要保护的端点：
- 登录端点: 防止暴力破解
- Token Exchange: 防止滥用
- 搜索/列表: 防止数据提取
- 管理操作: 防止自动化攻击

---

## 场景 1：登录端点限流

### 前置条件
- 登录端点可访问
- 压力测试工具

### 攻击目标
验证登录是否有速率限制

### 攻击步骤
1. 快速发送多个登录请求
2. 测试不同维度的限制：
   - 单 IP 限制
   - 单用户限制
   - 全局限制
3. 检查限流响应

### 预期安全行为
- IP 级别: 10-20 次/分钟
- 用户级别: 5 次/分钟 (失败后)
- 返回 429 Too Many Requests
- 包含 Retry-After 头

### 验证方法
```bash
# 说明：Auth9 的 /api/v1/auth/token 不支持 grant_type=password。
# 这里以 OIDC 授权入口作为“登录前入口”进行限流验证。

# 快速请求测试
for i in {1..50}; do
  curl -s -o /dev/null -w "%{http_code}\n" \
    "http://localhost:8080/api/v1/auth/authorize?client_id=auth9-portal&redirect_uri=http://localhost:3000/callback&response_type=code&scope=openid&state=rl-$i"
done
# 预期: 前 N 次 302/307, 之后 429

# 检查响应头
curl -i \
  "http://localhost:8080/api/v1/auth/authorize?client_id=auth9-portal&redirect_uri=http://localhost:3000/callback&response_type=code&scope=openid&state=rl-head"
# 检查 X-RateLimit-Limit, X-RateLimit-Remaining, Retry-After

# 不同 IP 测试 (使用代理)
# 验证 IP 级别限制
```

### 修复建议
- 实现滑动窗口限流
- IP + 用户名 组合限制
- 失败后增加延迟 (指数退避)
- 返回标准限流响应头

---

## 场景 2：慢速攻击防护

### 前置条件
- 能够建立 HTTP 连接
- slowloris 或类似工具

### 攻击目标
验证服务器是否防护慢速攻击

### 攻击步骤
1. Slowloris 攻击：
   - 建立多个连接
   - 缓慢发送请求头
   - 保持连接不关闭
2. Slow POST 攻击：
   - 发送大 Content-Length
   - 缓慢发送 body
3. 检查服务器响应

### 预期安全行为
- 连接超时 (30-60秒)
- 请求头/体大小限制
- 并发连接限制

### 验证方法
```bash
# Slowloris 测试 (需要工具)
# 使用 slowhttptest
slowhttptest -c 1000 -H -g -o slowloris \
  -i 10 -r 200 -t GET \
  -u http://localhost:8080/api/v1/users

# 手动慢速请求
# 1. 建立连接
exec 3<>/dev/tcp/localhost/8080
# 2. 缓慢发送
echo -ne "GET /api/v1/users HTTP/1.1\r\n" >&3
sleep 5
echo -ne "Host: localhost\r\n" >&3
sleep 5
# ... 观察是否超时断开

# 检查服务器配置
# Nginx/反向代理的超时设置
```

### 修复建议
- 请求头超时: 10-30 秒
- 请求体超时: 30-60 秒
- 单 IP 最大连接数限制
- 使用反向代理防护

---

## 场景 3：资源耗尽攻击

### 前置条件
- 有效的认证 Token
- 了解资源密集型操作

### 攻击目标
验证资源密集操作的保护

### 攻击步骤
1. 识别耗资源操作：
   - 复杂搜索查询
   - 大量数据导出
   - 报表生成
   - 批量操作
2. 并发执行这些操作
3. 监控服务器资源

### 预期安全行为
- 查询复杂度限制
- 导出数量限制
- 并发执行限制
- 队列/异步处理

### 验证方法
```bash
# 复杂搜索
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/users?search=a*&include=roles,permissions,tenants"
# 预期: 限制 include 深度

# 大量导出
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/audit-logs?limit=100000&export=csv"
# 预期: 限制数量或异步处理

# 并发批量操作
for i in {1..10}; do
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    http://localhost:8080/api/v1/services/batch-create \
    -d '{"count":100}' &
done
# 预期: 限制并发或排队
```

### 修复建议
- 查询超时限制 (30秒)
- 结果集大小限制
- 后台任务异步化
- 资源配额系统

---

## 场景 4：业务逻辑滥用

### 前置条件
- 有效账户
- 了解业务流程

### 攻击目标
验证业务逻辑是否可被滥用

### 攻击步骤
1. 邮件发送滥用：
   - 大量发送密码重置邮件
   - 大量发送邀请邮件
2. 短信/通知滥用：
   - 触发大量通知
3. 资源创建滥用：
   - 创建大量租户/服务

### 预期安全行为
- 邮件发送限制
- 通知频率限制
- 资源创建配额
- CAPTCHA 保护

### 验证方法
```bash
# 密码重置滥用
for i in {1..20}; do
  curl -X POST http://localhost:8080/api/v1/auth/forgot-password \
    -d '{"email":"victim@example.com"}'
  echo "Attempt: $i"
done
# 预期: 5 次后限流或 CAPTCHA

# 邀请发送滥用
for i in {1..50}; do
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    http://localhost:8080/api/v1/tenants/{id}/invitations \
    -d "{\"email\":\"spam$i@example.com\"}"
done
# 预期: 限制每日邀请数量

# 检查邮件日志
# 验证是否发送了大量邮件
```

### 修复建议
- 邮件: 每用户每小时 5 封
- 邀请: 每租户每日 50 个
- 需要 CAPTCHA 验证
- 管理员可配置限额

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | 登录端点限流 | ☐ | | | |
| 2 | 慢速攻击防护 | ☐ | | | |
| 3 | 资源耗尽攻击 | ☐ | | | |
| 4 | 业务逻辑滥用 | ☐ | | | |

---

## 推荐限流配置

| 端点类型 | 限制 | 维度 |
|---------|------|------|
| 登录 | 10/分钟 | IP + 用户名 |
| 密码重置 | 3/小时 | 邮箱 |
| Token Exchange | 100/分钟 | client_id |
| 读取 API | 100/分钟 | Token |
| 写入 API | 30/分钟 | Token |
| 搜索 API | 20/分钟 | Token |
| 批量操作 | 5/分钟 | Token |
| 文件上传 | 10/分钟 | Token |

---

## 测试工具

```bash
# hey - HTTP 压力测试
brew install hey
hey -n 1000 -c 50 http://localhost:8080/api/v1/users

# ab - Apache Bench
ab -n 1000 -c 50 http://localhost:8080/api/v1/users

# slowhttptest - 慢速攻击
brew install slowhttptest
slowhttptest -c 500 -H -u http://localhost:8080

# locust - Python 负载测试
pip install locust
locust -f loadtest.py --host=http://localhost:8080
```

---

## 参考资料

- [OWASP Rate Limiting](https://cheatsheetseries.owasp.org/cheatsheets/Denial_of_Service_Cheat_Sheet.html)
- [Cloudflare Rate Limiting](https://developers.cloudflare.com/waf/rate-limiting-rules/)
- [CWE-770: Resource Allocation Without Limits](https://cwe.mitre.org/data/definitions/770.html)
- [Slowloris Attack](https://en.wikipedia.org/wiki/Slowloris_(computer_security))

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-API-03  
**适用控制**: V4.4,V13.3,V2.5  
**关联任务**: Backlog #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 4

### 执行清单
- [ ] M-API-03-C01 | 控制: V4.4 | 任务: #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-API-03-C02 | 控制: V13.3 | 任务: #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-API-03-C03 | 控制: V2.5 | 任务: #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
