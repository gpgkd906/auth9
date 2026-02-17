# 数据安全 - 敏感数据暴露测试

**模块**: 数据安全
**测试范围**: 敏感信息泄露检测
**场景数**: 5
**风险等级**: 🔴 极高

---

## 背景知识

Auth9 敏感数据类型：
- **认证凭证**: 密码、Token、API Key
- **个人信息**: 邮箱、手机号、IP 地址
- **业务机密**: Client Secret、加密密钥
- **系统信息**: 内部路径、版本号、配置

---

## 场景 1：API 响应数据泄露

### 前置条件
- 有效的认证 Token
- 不同权限级别账户

### 攻击目标
验证 API 响应是否泄露敏感数据

### 攻击步骤
1. 检查用户相关 API：
   - 用户详情是否包含密码哈希
   - 是否泄露其他用户敏感信息
2. 检查服务相关 API：
   - Client Secret 是否在响应中
   - 是否泄露内部配置
3. 检查系统 API：
   - SMTP 密码是否明文返回
   - 密钥是否暴露

### 预期安全行为
- 密码永不返回 (包括哈希)
- Client Secret 仅创建时返回一次
- 敏感配置脱敏显示

### 验证方法
```bash
# 用户详情检查
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/users/me | jq .
# 不应包含: password, password_hash, keycloak_id (如果敏感)

# 服务客户端检查
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/services/{id}/clients | jq .
# 不应包含: client_secret (仅创建时返回)

# 系统配置检查
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:8080/api/v1/system/email | jq .
# 密码应显示为: "password": "***" 或 "password": null
```

### 修复建议
- DTO 定义排除敏感字段
- Secret 存储后不可逆转
- 脱敏显示敏感配置
- 审计响应内容

---

## 场景 2：错误信息泄露

### 前置条件
- API 访问权限

### 攻击目标
验证错误响应是否泄露系统信息

### 攻击步骤
1. 触发各种错误：
   - 500 服务器错误
   - 数据库错误
   - 认证错误
2. 分析错误消息：
   - 堆栈跟踪
   - SQL 查询
   - 文件路径
   - 版本信息

### 预期安全行为
- 通用错误消息
- 不暴露堆栈跟踪
- 不暴露数据库信息

### 验证方法
```bash
# 触发数据库错误
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/users?order_by=invalid_column"
# 不应包含: SQL 语句、表名、列名

# 触发服务器错误
curl -X POST -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/crash-test
# 不应包含: 堆栈跟踪、文件路径

# 无效 JSON
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/users \
  -d '{invalid json}'
# 不应泄露解析器详细信息

# 检查响应内容
# 应该: {"error": "Internal server error", "code": "INTERNAL_ERROR"}
# 不应该: {"error": "SqlError: no such column: invalid_column at src/repo/user.rs:45"}
```

### 修复建议
- 生产环境隐藏详细错误
- 使用错误代码而非描述
- 将详细错误记录到日志
- 实现统一错误处理中间件

---

## 场景 3：日志敏感信息泄露

### 前置条件
- 日志访问权限 (测试环境)

### 攻击目标
验证日志是否记录敏感信息

### 攻击步骤
1. 执行敏感操作
2. 检查日志文件：
   - 密码/Token 是否被记录
   - 完整请求体是否记录
   - PII 是否未脱敏
3. 检查审计日志表

### 预期安全行为
- 密码/Token 从不记录
- PII 脱敏或不记录
- 审计日志安全存储

### 验证方法
```bash
# 触发包含敏感字段的认证相关请求
curl -X POST http://localhost:8080/api/v1/auth/forgot-password \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com"}'

# 触发 token 相关请求
curl -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/json" \
  -d '{"grant_type":"client_credentials","client_id":"test-client","client_secret":"TopSecretValue"}'

# 检查日志
docker logs auth9-core 2>&1 | grep -i "password\|secret\|token"
# 不应找到明文密码

# 检查审计日志
SELECT * FROM audit_logs WHERE action = 'user.login' ORDER BY created_at DESC LIMIT 1;
# 不应包含密码

# 检查请求日志级别
# DEBUG 级别可能记录过多信息
```

### 修复建议
- 敏感字段自动脱敏
- 生产环境日志级别 INFO+
- 日志保留策略
- 定期审计日志内容

---

## 场景 4：文件/备份泄露

### 前置条件
- 能够探测服务器文件

### 攻击目标
验证是否可访问敏感文件

### 攻击步骤
1. 探测常见敏感文件：
   - 配置文件
   - 数据库备份
   - 日志文件
   - 源代码
2. 检查目录列表
3. 检查 Git 目录

### 预期安全行为
- 敏感文件不可访问
- 禁用目录列表
- .git 目录受保护

### 验证方法
```bash
# 配置文件探测
curl http://localhost:8080/config.yaml
curl http://localhost:8080/.env
curl http://localhost:8080/application.properties
# 预期: 404

# 备份文件探测
curl http://localhost:8080/backup.sql
curl http://localhost:8080/db.sqlite
curl http://localhost:8080/dump.tar.gz
# 预期: 404

# Git 目录
curl http://localhost:8080/.git/config
curl http://localhost:8080/.git/HEAD
# 预期: 404

# 目录列表
curl http://localhost:8080/uploads/
# 预期: 403 或 404 (不是文件列表)

# 源码泄露
curl http://localhost:8080/main.rs
curl http://localhost:3000/app/routes/_index.tsx
# 预期: 404
```

### 修复建议
- 配置 Web 服务器禁止访问敏感文件
- 禁用目录索引
- 敏感文件放在 Web 根目录外
- 使用 .dockerignore 排除敏感文件

---

## 场景 5：元数据泄露

### 前置条件
- API 访问

### 攻击目标
验证 HTTP 头和元数据是否泄露信息

### 攻击步骤
1. 检查响应头：
   - Server 版本
   - X-Powered-By
   - 内部 IP
2. 检查 OpenAPI/Swagger：
   - 是否暴露内部端点
   - 是否泄露参数细节
3. 检查健康检查端点

### 预期安全行为
- 隐藏服务器版本
- 移除 X-Powered-By
- API 文档需要认证

### 验证方法
```bash
# 响应头检查
curl -I http://localhost:8080/api/v1/health
# 检查:
# Server: 不应显示版本 (如 nginx/1.19.0)
# X-Powered-By: 应该不存在
# X-AspNet-Version: 应该不存在

# OpenAPI 文档
curl http://localhost:8080/swagger.json
curl http://localhost:8080/openapi.yaml
curl http://localhost:8080/api-docs
# 预期: 404 或需要认证

# 健康检查详情
curl http://localhost:8080/health | jq .
# 不应包含: 数据库连接字符串、内部 IP、版本号

# 错误页面
curl http://localhost:8080/nonexistent
# 检查 404 页面是否泄露服务器信息
```

### 修复建议
- 移除版本信息头
- API 文档需要认证
- 健康检查返回简单状态
- 自定义错误页面

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | API 响应数据泄露 | ☐ | | | |
| 2 | 错误信息泄露 | ☐ | | | |
| 3 | 日志敏感信息泄露 | ☐ | | | |
| 4 | 文件/备份泄露 | ☐ | | | |
| 5 | 元数据泄露 | ☐ | | | |

---

## 敏感数据清单

| 数据类型 | 存储位置 | 处理要求 |
|---------|---------|---------|
| 用户密码 | Keycloak | 仅哈希存储，永不返回 |
| Client Secret | services 表 | 哈希存储，创建时返回一次 |
| API Key | api_keys 表 | 哈希存储，创建时返回一次 |
| JWT Secret | 环境变量 | 永不记录或返回 |
| SMTP 密码 | system_settings | 加密存储，脱敏显示 |
| 邮箱 | users 表 | PII，日志脱敏 |
| IP 地址 | sessions/logs | PII，保留期限限制 |

---

## 参考资料

- [OWASP Sensitive Data Exposure](https://owasp.org/www-project-top-ten/2017/A3_2017-Sensitive_Data_Exposure)
- [GDPR Data Protection](https://gdpr.eu/what-is-gdpr/)
- [CWE-200: Exposure of Sensitive Information](https://cwe.mitre.org/data/definitions/200.html)
- [CWE-209: Error Message Information Leak](https://cwe.mitre.org/data/definitions/209.html)
