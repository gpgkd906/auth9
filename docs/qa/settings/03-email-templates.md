# 系统设置 - 邮件模板管理测试

**模块**: 系统设置
**测试范围**: 邮件模板查看、编辑、预览、重置
**场景数**: 5

---

## 数据库表结构参考

### email_templates 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| template_type | VARCHAR(50) | 模板类型 |
| subject | VARCHAR(255) | 邮件主题 |
| html_body | TEXT | HTML 正文 |
| text_body | TEXT | 纯文本正文 |
| is_customized | BOOLEAN | 是否已自定义 |
| created_at | TIMESTAMP | 创建时间 |
| updated_at | TIMESTAMP | 更新时间 |

### 模板类型
| 类型 | 说明 |
|------|------|
| invitation | 邀请邮件 |
| password_reset | 密码重置 |
| email_mfa | 邮箱 MFA 验证码 |
| welcome | 欢迎邮件 |
| email_verification | 邮箱验证 |
| password_changed | 密码已更改通知 |
| security_alert | 安全告警 |

---

## 场景 1：查看邮件模板列表

### 初始状态
- 管理员已登录
- 系统存在默认模板

### 目的
验证邮件模板列表页面正确显示

### 测试操作流程
1. 进入「设置」→「邮件模板」

### 预期结果
- 显示所有模板类型的列表
- 每个模板显示：名称、描述、状态（Default/Custom）
- 未修改的模板显示「Default」标签
- 已修改的模板显示「Custom」标签（蓝色）
- 每行有「Edit」按钮

### 预期数据状态
```sql
SELECT template_type, is_customized FROM email_templates;
-- 预期: 列出所有模板类型
```

---

## 场景 2：编辑邮件模板

### 初始状态
- 管理员已登录
- 存在默认的 invitation 模板

### 目的
验证邮件模板编辑功能

### 测试操作流程
1. 进入「设置」→「邮件模板」
2. 点击「Invitation」模板的「Edit」按钮
3. 修改内容：
   - Subject：`You've been invited to join {{tenant_name}}`
   - HTML Body：添加自定义样式
   - Text Body：修改纯文本版本
4. 点击「Save Changes」

### 预期结果
- 显示「Template saved successfully」提示
- 返回列表页，该模板显示「Custom」标签
- 再次编辑时显示修改后的内容

### 预期数据状态
```sql
SELECT template_type, subject, is_customized, updated_at
FROM email_templates
WHERE template_type = 'invitation';
-- 预期: is_customized = true, subject = "You've been invited to join {{tenant_name}}"
```

---

## 场景 3：预览邮件模板

### 初始状态
- 管理员已登录
- 正在编辑某个模板

### 目的
验证模板预览功能，使用示例数据渲染

### 测试操作流程
1. 进入某个模板的编辑页面
2. 在编辑区域修改内容，使用变量如 `{{name}}`
3. 点击「Preview」按钮

### 预期结果
- 显示预览窗口
- 变量被替换为示例值（如 `{{name}}` → `John Doe`）
- 同时显示 HTML 和纯文本版本
- 预览不会保存更改

### 预期数据状态
无数据库变更

---

## 场景 4：发送模板测试邮件

### 初始状态
- 管理员已登录
- 已配置有效的邮件服务商
- 正在编辑某个模板

### 目的
验证使用当前编辑内容发送测试邮件

### 测试操作流程
1. 进入某个模板的编辑页面
2. 修改模板内容
3. 点击「Send Test Email」
4. 输入收件人邮箱：`test@example.com`
5. 可选填写测试变量
6. 点击发送

### 预期结果
- 显示「Test email sent successfully」
- 收件箱收到测试邮件
- 邮件内容使用当前编辑的模板（非已保存版本）
- 变量使用填写的测试值或默认示例值

### 预期数据状态
无数据库变更（除非点击保存）

---

## 场景 5：重置模板为默认

### 初始状态
- 管理员已登录
- 某个模板已被自定义（显示 Custom 标签）

### 目的
验证模板重置功能

### 测试操作流程
1. 进入已自定义模板的编辑页面
2. 点击「Reset to Default」按钮
3. 确认重置

### 预期结果
- 显示「Template reset to default」提示
- 模板内容恢复为系统默认值
- 列表页该模板显示「Default」标签

### 预期数据状态
```sql
SELECT template_type, is_customized
FROM email_templates
WHERE template_type = '{template_type}';
-- 预期: is_customized = false，内容为默认值
```

---

## 模板变量参考

### 通用变量
| 变量 | 说明 |
|------|------|
| `{{app_name}}` | 应用名称（Auth9） |
| `{{year}}` | 当前年份 |

### 邀请模板 (invitation)
| 变量 | 说明 |
|------|------|
| `{{invitee_email}}` | 被邀请人邮箱 |
| `{{tenant_name}}` | 租户名称 |
| `{{inviter_name}}` | 邀请人名称 |
| `{{invite_link}}` | 邀请链接 |
| `{{expires_at}}` | 过期时间 |

### 密码重置模板 (password_reset)
| 变量 | 说明 |
|------|------|
| `{{name}}` | 用户名称 |
| `{{reset_link}}` | 重置链接 |
| `{{expires_in}}` | 有效期 |

---

## 通用场景：认证状态检查

### 初始状态
- 用户已登录管理后台
- 页面正常显示

### 目的
验证页面正确检查认证状态，未登录或 session 失效时重定向到登录页

### 测试操作流程
1. 通过以下任一方式构造未认证状态：
   - 使用浏览器无痕/隐私窗口访问
   - 手动清除 auth9_session cookie
   - 在当前会话点击「Sign out」退出登录
2. 访问本页面对应的 URL

### 预期结果
- 页面自动重定向到 `/login`
- 不显示 dashboard 内容
- 登录后可正常访问原页面

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 查看邮件模板列表 | ☐ | | | |
| 2 | 编辑邮件模板 | ☐ | | | |
| 3 | 预览邮件模板 | ☐ | | | |
| 4 | 发送模板测试邮件 | ☐ | | | |
| 5 | 重置模板为默认 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
