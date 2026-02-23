# 系统设置 - 登录页品牌设置测试

**模块**: 系统设置
**测试范围**: 登录页外观自定义、品牌配置
**场景数**: 5

---

## 架构说明

Auth9 采用 Headless Keycloak 架构，品牌设置支持两级配置：

1. **系统级品牌**（本页面） → 管理员在 Auth9 Portal 的「设置 → 登录页品牌」页面配置颜色、Logo、公司名称等，作为全局默认
2. **Service 级品牌**（见 [service/06-service-branding.md](../service/06-service-branding.md)） → 在 Service 详情页「Branding」标签页可为单个 Service 覆盖品牌配置
3. **auth9-keycloak-theme 消费品牌配置** → Keycloak 主题通过公开端点 `GET /api/v1/public/branding?client_id={client_id}` 获取品牌配置；若该 client 所属 Service 有自定义品牌则优先使用，否则降级到系统默认
4. **最终用户看到的效果** → 用户在 Keycloak 托管的登录/注册/忘记密码等页面上看到的是品牌风格的界面（可能因 Service 不同而不同）

**页面归属**：
- 「设置 → 登录页品牌」管理页面 → Auth9 Portal
- 受品牌设置影响的登录/注册页面 → Keycloak 托管（auth9-keycloak-theme 渲染）

**测试原则**：
- 默认通过 Auth9 登录入口触发并观察品牌化登录页效果
- 不要求必须手工直接访问 Keycloak 登录页面 URL
- 如需排障，可直接访问 Keycloak URL 进行补充验证

---

## 数据库表结构参考

### system_settings 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| category | VARCHAR(50) | 设置类别：branding |
| setting_key | VARCHAR(100) | 设置键名 |
| value | JSON | 品牌配置 JSON |
| updated_at | TIMESTAMP | 更新时间 |

### BrandingConfig 字段
| 字段 | 类型 | 说明 |
|------|------|------|
| logo_url | TEXT | Logo 图片 URL |
| primary_color | VARCHAR(7) | 主色调（HEX） |
| secondary_color | VARCHAR(7) | 次色调（HEX） |
| background_color | VARCHAR(7) | 背景色（HEX） |
| text_color | VARCHAR(7) | 文字颜色（HEX） |
| company_name | VARCHAR(100) | 公司名称 |
| favicon_url | TEXT | Favicon URL |
| custom_css | TEXT | 自定义 CSS |
| allow_registration | BOOLEAN | 是否允许注册 |

---

## 场景 1：查看品牌设置页面

### 初始状态
- 管理员已登录
- 系统使用默认品牌设置

### 目的
验证品牌设置页面正确加载并显示当前配置

### 测试操作流程
1. 进入「设置」→「登录页品牌」
2. 观察页面加载

### 预期结果
- 显示公司标识区域（Company Name、Logo URL、Favicon URL）
- 显示颜色选择器（Primary、Secondary、Background、Text）
- 显示登录选项开关（允许注册）
- 显示自定义 CSS 输入框
- 显示预览区域，实时展示效果
- 默认值：Primary=#007AFF, Secondary=#5856D6, Background=#F5F5F7, Text=#1D1D1F

### 预期数据状态
```sql
SELECT category, setting_key, value FROM system_settings WHERE category = 'branding';
-- 可能为空（使用默认值）或存在自定义配置
```

---

## 场景 2：更新品牌颜色

### 初始状态
- 管理员已登录
- 系统使用默认品牌设置

### 目的
验证品牌颜色更新功能

### 测试操作流程
1. 进入「设置」→「登录页品牌」
2. 修改以下颜色：
   - Primary Color：`#FF5733`
   - Secondary Color：`#33FF57`
   - Background Color：`#FFFFFF`
   - Text Color：`#333333`
3. 点击「Save Changes」

### 预期结果
- 显示「Branding settings saved successfully」提示
- 预览区域实时更新显示新颜色
- 刷新页面后设置保持

### 预期数据状态
```sql
SELECT JSON_EXTRACT(value, '$.primary_color') as primary_color,
       JSON_EXTRACT(value, '$.secondary_color') as secondary_color
FROM system_settings
WHERE category = 'branding' AND setting_key = 'config';
-- 预期: primary_color = "#FF5733", secondary_color = "#33FF57"
```

---

## 场景 3：设置 Logo 和公司名称

### 初始状态
- 管理员已登录

### 目的
验证 Logo URL 和公司名称设置功能

### 测试操作流程
1. 进入「设置」→「登录页品牌」
2. 填写：
   - Company Name：`Test Corporation`
   - Logo URL：`https://cdn.example.com/logo.png`
   - Favicon URL：`https://assets.example.com/favicon.ico`
3. 点击「Save Changes」

### 预期结果
- 显示保存成功提示
- Logo 预览区显示图片
- 预览登录表单显示公司名称或 Logo

> **注意**: Logo URL 和 Favicon URL 的域名必须在 `BRANDING_ALLOWED_DOMAINS` 允许列表中（默认：`cdn.example.com`, `assets.example.com`）。使用未授权域名会返回验证错误。

### 预期数据状态
```sql
SELECT JSON_EXTRACT(value, '$.company_name') as company_name,
       JSON_EXTRACT(value, '$.logo_url') as logo_url,
       JSON_EXTRACT(value, '$.favicon_url') as favicon_url
FROM system_settings
WHERE category = 'branding' AND setting_key = 'config';
-- 预期: company_name = "Test Corporation", logo_url = "https://cdn.example.com/logo.png"
```

---

## 场景 4：启用/禁用注册功能

### 初始状态
- 管理员已登录
- 当前允许注册设置为关闭

### 目的
验证注册开关功能影响 Keycloak 登录页（auth9-keycloak-theme 定制外观）

### 测试操作流程
1. 进入「设置」→「登录页品牌」
2. 开启「Allow Registration」开关
3. 点击「Save Changes」
4. 通过 Auth9 登录入口触发登录流程，进入品牌化登录页

### 预期结果
- 设置保存成功
- 品牌化登录页（底层由 Keycloak 渲染）显示「Create account」链接

### 预期数据状态
```sql
SELECT JSON_EXTRACT(value, '$.allow_registration') as allow_registration
FROM system_settings
WHERE category = 'branding' AND setting_key = 'config';
-- 预期: allow_registration = true
```

### Keycloak 验证
- 访问 `http://localhost:8081/realms/auth9/protocol/openid-connect/auth?...`
- 页面应显示注册链接

### 故障排查

| 症状 | 原因 | 解决 |
|------|------|------|
| DB 中 `allow_registration = true` 但 Keycloak `registrationAllowed` 未变 | Keycloak 同步是异步 fire-and-forget，可能因 Keycloak 暂时不可达而静默失败 | 检查 auth9-core 日志中 `Failed to sync realm settings to Keycloak` 错误；确认 Keycloak 容器健康后重新保存设置 |
| 保存后 Keycloak 返回 401/403 | Keycloak admin token 过期或权限不足 | 重启 auth9-core 刷新 admin token |
| 登录页未显示注册链接 | 浏览器缓存或 Keycloak theme 缓存 | 清除浏览器缓存，或重启 Keycloak 容器清除 theme 缓存 |

---

## 场景 5：重置为默认品牌设置

### 初始状态
- 管理员已登录
- 品牌设置已自定义（非默认值）

### 目的
验证重置功能恢复默认设置

### 测试操作流程
1. 进入「设置」→「登录页品牌」
2. 确认当前设置非默认（颜色或 Logo 已修改）
3. 点击「Reset to Defaults」按钮
4. 确认重置

### 预期结果
- 显示「Branding reset to defaults」提示
- 所有颜色恢复默认值
- Logo URL、Favicon URL、Company Name 清空
- Custom CSS 清空
- Allow Registration 关闭

### 预期数据状态
```sql
SELECT JSON_EXTRACT(value, '$.primary_color') as primary_color,
       JSON_EXTRACT(value, '$.logo_url') as logo_url
FROM system_settings
WHERE category = 'branding' AND setting_key = 'config';
-- 预期: primary_color = "#007AFF", logo_url = null
```

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
| 1 | 查看品牌设置页面 | ☐ | | | |
| 2 | 更新品牌颜色 | ☐ | | | |
| 3 | 设置 Logo 和公司名称 | ☐ | | | |
| 4 | 启用/禁用注册功能 | ☐ | | | |
| 5 | 重置为默认品牌设置 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
