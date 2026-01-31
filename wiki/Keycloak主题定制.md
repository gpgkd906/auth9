# Keycloak 主题定制

Auth9 提供了自定义的 Keycloak 登录主题，支持动态品牌配置。主题会在运行时从 Auth9 API 获取品牌配置（Logo、颜色、公司名称等），无需重启 Keycloak 即可自定义外观。

## 特性

- **动态品牌** - 从 Auth9 API 实时获取 Logo、颜色和公司名称
- **自定义 CSS** - 支持注入自定义 CSS 样式
- **现代设计** - 简洁、响应式的登录和注册页面
- **零默认样式** - 完全自定义 UI，不使用 Keycloak 默认的 PatternFly CSS

## 架构说明

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  auth9-portal   │────▶│   auth9-core    │◀────│    Keycloak     │
│  (设置界面)     │     │  (品牌 API)     │     │  (主题渲染)     │
└─────────────────┘     └─────────────────┘     └─────────────────┘
        │                       │                       │
        │   PUT /api/v1/       │   GET /api/v1/        │
        │   system/branding    │   public/branding     │
        └──────────────────────┴───────────────────────┘
```

工作流程：
1. 管理员在 auth9-portal 中配置品牌设置
2. 配置保存到 auth9-core 的数据库
3. Keycloak 主题通过公开 API 获取品牌配置
4. 主题根据配置动态渲染登录页面

## 开发环境搭建

### 前置要求

- Node.js 20+
- npm 10+
- Maven（用于构建 JAR，或使用 Docker）

### 开发步骤

```bash
cd auth9-keycloak-theme

# 安装依赖
npm install

# 启动 Vite 开发服务器（热重载）
npm run dev

# 构建 React 应用
npm run build
```

开发服务器会在 `http://localhost:5173` 启动，可以实时预览主题效果。

## 构建主题 JAR

主题需要打包成 JAR 文件才能部署到 Keycloak。

### 方式 1：本地构建（需要 Maven）

```bash
cd auth9-keycloak-theme

# 构建主题 JAR
npm run build-keycloak-theme

# 输出：dist_keycloak/keycloak-theme-auth9.jar
```

### 方式 2：Docker 构建（推荐）

```bash
cd auth9-keycloak-theme

# 使用 Docker 构建主题
docker build -t auth9-keycloak-theme .

# 提取 JAR 到当前目录
docker run --rm -v $(pwd)/output:/theme-output auth9-keycloak-theme
```

### 方式 3：Docker Compose 集成构建

```bash
# 从项目根目录运行
cd /path/to/auth9

# 构建主题 JAR
docker-compose --profile build up auth9-theme-builder

# 启动所有服务（包括带主题的 Keycloak）
docker-compose up -d
```

## 部署主题

### 在 Docker Compose 中部署

主题已集成到 `docker-compose.yml` 中，自动部署：

```bash
# 启动所有服务
docker-compose up -d
```

Keycloak 会自动加载 auth9 主题。

### 手动部署到 Keycloak

如果需要手动部署：

1. 复制 JAR 到 Keycloak 的 providers 目录：

```bash
cp dist_keycloak/keycloak-theme-auth9.jar /opt/keycloak/providers/
```

2. 重启 Keycloak：

```bash
docker-compose restart keycloak
```

3. 在 Keycloak 管理控制台中启用主题：
   - 登录 Keycloak Admin Console
   - 进入 Realm Settings → Themes
   - Login Theme 选择 "auth9"
   - 点击 Save

## 配置

### 环境变量

主题通过 Keycloak 的主题属性读取配置：

| 环境变量 | 默认值 | 说明 |
|---------|-------|------|
| `AUTH9_API_URL` | `http://localhost:8080` | Auth9 API 的 URL |

在启动 Keycloak 时设置：

```bash
AUTH9_API_URL=https://api.example.com docker-compose up keycloak
```

或在 `docker-compose.yml` 中配置：

```yaml
keycloak:
  environment:
    - AUTH9_API_URL=https://api.example.com
```

## 品牌配置 API

主题从以下公开端点获取品牌配置：

```
GET /api/v1/public/branding
```

响应格式：

```json
{
  "data": {
    "logo_url": "https://example.com/logo.png",
    "primary_color": "#007AFF",
    "secondary_color": "#5856D6",
    "background_color": "#F5F5F7",
    "text_color": "#1D1D1F",
    "custom_css": ".custom { color: red; }",
    "company_name": "我的公司",
    "favicon_url": "https://example.com/favicon.ico"
  }
}
```

### 配置品牌

通过 Auth9 管理界面或 API 配置品牌：

```bash
curl -X PUT https://api.auth9.yourdomain.com/api/v1/system/branding \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "config": {
      "logo_url": "https://cdn.example.com/logo.png",
      "primary_color": "#FF6B6B",
      "secondary_color": "#4ECDC4",
      "company_name": "我的公司",
      "custom_css": "body { font-family: \"PingFang SC\", sans-serif; }"
    }
  }'
```

## 自定义页面

主题自定义了以下 Keycloak 页面：

| 页面 | 源文件 | 说明 |
|-----|--------|------|
| 登录 | `src/login/pages/Login.tsx` | 主登录页面 |
| 注册 | `src/login/pages/Register.tsx` | 用户注册页面 |
| 重置密码 | `src/login/pages/LoginResetPassword.tsx` | 密码重置请求 |
| OTP | `src/login/pages/LoginOtp.tsx` | 一次性密码输入 |

其他页面使用 Keycloakify 的默认实现。

## 自定义样式

### 方式 1：通过品牌配置注入 CSS

在品牌配置中添加自定义 CSS：

```json
{
  "custom_css": ".login-page { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); }"
}
```

### 方式 2：修改主题源码

编辑 `src/login/KcPage.tsx` 或相关组件文件，添加自定义样式。

### 方式 3：覆盖主题变量

在主题配置中设置颜色变量：

```json
{
  "primary_color": "#667eea",
  "secondary_color": "#764ba2",
  "background_color": "#ffffff",
  "text_color": "#333333"
}
```

## 技术栈

- **React** - UI 组件库
- **TypeScript** - 类型安全
- **Vite** - 快速构建工具
- **Keycloakify** - Keycloak 主题生成器
- **TailwindCSS** - 样式框架（可选）

## 目录结构

```
auth9-keycloak-theme/
├── src/
│   ├── login/           # 登录相关页面
│   │   ├── pages/       # 自定义页面组件
│   │   │   ├── Login.tsx
│   │   │   ├── Register.tsx
│   │   │   └── ...
│   │   └── KcPage.tsx   # 主题入口
│   ├── api/             # API 调用
│   └── types/           # TypeScript 类型定义
├── public/              # 静态资源
├── dist_keycloak/       # 构建输出
├── Dockerfile           # Docker 构建配置
├── package.json         # 项目配置
└── vite.config.ts       # Vite 配置
```

## 开发调试

### 本地调试

1. 启动 Auth9 后端服务
2. 启动主题开发服务器：

```bash
cd auth9-keycloak-theme
npm run dev
```

3. 在浏览器中访问 `http://localhost:5173`

### 集成测试

1. 构建主题 JAR
2. 重启 Keycloak
3. 访问 Keycloak 登录页面测试效果

## 常见问题

### Q: 为什么修改品牌配置后没有生效？

A: 主题会缓存品牌配置。可以：
1. 清除浏览器缓存
2. 强制刷新页面（Ctrl+F5）
3. 等待缓存过期（通常 5 分钟）

### Q: 如何调试主题样式？

A: 
1. 使用浏览器开发者工具检查元素
2. 在品牌配置的 `custom_css` 中添加调试样式
3. 使用 `npm run dev` 在开发模式下实时预览

### Q: 主题支持哪些 Keycloak 版本？

A: 主题基于 Keycloakify 构建，支持 Keycloak 22+ 版本。

### Q: 可以自定义更多页面吗？

A: 可以。在 `src/login/pages/` 中添加新的页面组件，并在 `KcPage.tsx` 中注册。

### Q: 主题是否支持多语言？

A: 支持。可以在页面组件中使用 Keycloak 的 i18n 功能。

## 最佳实践

### 1. 图片优化

- Logo 建议使用 SVG 或 PNG（透明背景）
- 图片大小不超过 100KB
- 使用 CDN 托管图片资源

### 2. 颜色选择

- 确保文字和背景有足够的对比度（WCAG AA 标准）
- 主色和辅色应协调
- 测试深色和浅色模式

### 3. 自定义 CSS

- 避免使用 `!important`
- 使用 CSS 变量便于维护
- 测试不同浏览器的兼容性

### 4. 性能优化

- 压缩图片资源
- 减少自定义 CSS 大小
- 使用浏览器缓存

## 版本控制

主题版本与 Auth9 版本同步：

- Auth9 0.1.0 → Theme 0.1.0
- Auth9 0.2.0 → Theme 0.2.0

## 相关文档

- [品牌定制 API](REST-API.md#品牌定制-api)
- [系统设置](配置说明.md)
- [本地开发](本地开发.md)
- [Keycloakify 官方文档](https://docs.keycloakify.dev/)

## 贡献

欢迎贡献主题改进和新页面：

1. Fork 仓库
2. 创建功能分支
3. 修改主题代码
4. 提交 Pull Request

---

**最后更新**: 2026-01-31
