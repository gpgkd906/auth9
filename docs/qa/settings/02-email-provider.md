# 系统设置 - 邮件服务商配置测试

**模块**: 系统设置
**测试范围**: 邮件服务商 SMTP/SES/Oracle 配置
**场景数**: 5

---

## 数据库表结构参考

### system_settings 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| category | VARCHAR(50) | 设置类别：email |
| setting_key | VARCHAR(100) | 设置键名：provider_config |
| value | JSON | 邮件服务商配置 |
| updated_at | TIMESTAMP | 更新时间 |

### EmailProviderConfig 类型
| 类型 | 必填字段 |
|------|---------|
| none | 无 |
| smtp | host, port, from_email |
| ses | region, from_email |
| oracle | smtp_endpoint, port, username, password, from_email |

---

## 场景 1：配置 SMTP 邮件服务

### 初始状态
- 管理员已登录
- 当前无邮件服务商配置（显示 Email Provider Not Configured）

### 目的
验证 SMTP 邮件服务配置功能

### 测试操作流程
1. 进入「设置」→「邮件设置」
2. 在 Provider Type 下拉选择「SMTP」
3. 填写配置：
   - Server Host：`smtp.example.com`
   - Port：`587`
   - Username：`testuser`
   - Password：`testpass`
   - From Email：`noreply@example.com`
   - From Name：`Auth9`
   - 勾选「Use TLS encryption」
4. 点击「Save Settings」

### 预期结果
- 显示「Email settings saved successfully」提示
- 状态卡片变为「Email Provider Active」
- 显示「Using SMTP (smtp.example.com:587)」

### 预期数据状态
```sql
SELECT JSON_EXTRACT(value, '$.type') as type,
       JSON_EXTRACT(value, '$.host') as host,
       JSON_EXTRACT(value, '$.port') as port
FROM system_settings
WHERE category = 'email' AND setting_key = 'provider_config';
-- 预期: type = "smtp", host = "smtp.example.com", port = 587
```

---

## 场景 2：配置 AWS SES 邮件服务

### 初始状态
- 管理员已登录
- 当前无邮件服务商配置或使用其他类型

### 目的
验证 AWS SES 邮件服务配置功能

### 测试操作流程
1. 进入「设置」→「邮件设置」
2. 在 Provider Type 下拉选择「AWS SES」
3. 如果提示切换服务商，确认切换
4. 填写配置：
   - AWS Region：`us-east-1`
   - Access Key ID：`AKIAXXXXXXXXXX`
   - Secret Access Key：`secret123`
   - From Email：`noreply@example.com`
   - From Name：`Auth9`
5. 点击「Save Settings」

### 预期结果
- 显示保存成功提示
- 状态卡片显示「Using AWS SES (Region: us-east-1)」

### 预期数据状态
```sql
SELECT JSON_EXTRACT(value, '$.type') as type,
       JSON_EXTRACT(value, '$.region') as region
FROM system_settings
WHERE category = 'email' AND setting_key = 'provider_config';
-- 预期: type = "ses", region = "us-east-1"
```

---

## 场景 3：测试邮件连接

### 初始状态
- 管理员已登录
- 已配置有效的邮件服务商

### 目的
验证邮件连接测试功能

### 测试操作流程
1. 进入「设置」→「邮件设置」
2. 确认已配置邮件服务商
3. 点击「Test Connection」按钮

### 预期结果
- 成功情况：显示「Connection test successful」
- 失败情况：显示具体错误信息（如认证失败、连接超时）

### 预期数据状态
无数据库变更

---

## 场景 4：发送测试邮件

### 初始状态
- 管理员已登录
- 已配置有效的邮件服务商

### 目的
验证测试邮件发送功能

### 测试操作流程
1. 进入「设置」→「邮件设置」
2. 点击「Send Test Email」按钮
3. 在弹窗中输入：`test@example.com`
4. 点击「Send Test Email」

### 预期结果
- 显示「Test email sent to test@example.com」
- 收件箱收到测试邮件
- 邮件发件人显示配置的 From Name 和 From Email

### 预期数据状态
无数据库变更

---

## 场景 5：禁用邮件服务

### 初始状态
- 管理员已登录
- 当前已配置邮件服务商

### 目的
验证禁用邮件服务功能

### 测试操作流程
1. 进入「设置」→「邮件设置」
2. 在 Provider Type 下拉选择「None (Email disabled)」
3. 点击「Save Settings」

### 预期结果
- 显示保存成功提示
- 状态卡片变为黄色，显示「Email Provider Not Configured」
- 「Test Connection」和「Send Test Email」按钮消失

### 预期数据状态
```sql
SELECT JSON_EXTRACT(value, '$.type') as type
FROM system_settings
WHERE category = 'email' AND setting_key = 'provider_config';
-- 预期: type = "none"
```

---

## 通用场景：认证状态检查

### 初始状态
- 用户已登录管理后台
- 页面正常显示

### 目的
验证页面正确检查认证状态，未登录或 session 失效时重定向到登录页

### 测试操作流程
1. 关闭浏览器
2. 重新打开浏览器，访问本页面对应的 URL

### 预期结果
- 页面自动重定向到 `/login`
- 不显示 dashboard 内容
- 登录后可正常访问原页面

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 配置 SMTP 邮件服务 | ☐ | | | |
| 2 | 配置 AWS SES 邮件服务 | ☐ | | | |
| 3 | 测试邮件连接 | ☐ | | | |
| 4 | 发送测试邮件 | ☐ | | | |
| 5 | 禁用邮件服务 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
