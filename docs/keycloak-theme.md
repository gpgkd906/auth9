# Keycloak 登录主题管理

## 1. 概述

Auth9 使用 Keycloakify 构建自定义 Keycloak 登录主题，支持动态品牌配置。用户可以通过 Portal 设置 Logo、颜色等品牌元素，无需重启 Keycloak 即可生效。

### 1.1 架构

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  auth9-portal   │────▶│   auth9-core    │◀────│    Keycloak     │
│  (品牌设置页面)  │     │  (Branding API) │     │ (Keycloakify    │
│                 │     │                 │     │   主题)         │
└─────────────────┘     └─────────────────┘     └─────────────────┘
        │                       │                       │
        │   PUT /api/v1/       │   GET /api/v1/        │
        │   system/branding    │   public/branding     │
        └──────────────────────┴───────────────────────┘
```

### 1.2 工作原理

1. **管理员配置**: 在 Portal (`/dashboard/settings/branding`) 设置品牌信息
2. **存储**: 配置保存在 auth9-core 的 `system_settings` 表中
3. **运行时加载**: Keycloak 登录页面通过 JavaScript 从 auth9 API 获取品牌配置
4. **动态渲染**: 应用颜色、Logo、自定义 CSS 等样式

### 1.3 项目结构

```
auth9-keycloak-theme/
├── package.json              # 依赖和构建脚本
├── vite.config.ts            # Vite + Keycloakify 配置
├── Dockerfile                # Docker 构建 (包含 Maven)
├── src/
│   ├── main.tsx              # 入口文件
│   └── login/
│       ├── KcPage.tsx        # 页面路由
│       ├── hooks/
│       │   └── useBranding.ts    # 从 API 获取品牌配置
│       ├── components/
│       │   └── BrandingProvider.tsx  # 品牌上下文
│       └── pages/
│           ├── Login.tsx         # 登录页面
│           ├── Register.tsx      # 注册页面
│           ├── LoginResetPassword.tsx
│           └── LoginOtp.tsx
```

## 2. 本地开发

### 2.1 环境要求

- Node.js 20+
- npm 10+
- Maven (本地构建 JAR 时需要，使用 Docker 构建则不需要)

### 2.2 安装依赖

```bash
cd auth9-keycloak-theme
npm install
```

### 2.3 开发模式

```bash
# 启动 Vite 开发服务器
npm run dev
```

开发模式下会显示一个提示页面。要预览实际的登录页面效果，需要：

1. 取消 `src/main.tsx` 中的 mock context 注释
2. 或者使用 Docker 构建并部署到 Keycloak

### 2.4 TypeScript 检查

```bash
npx tsc --noEmit
```

## 3. 构建主题

### 3.1 本地构建 (需要 Maven)

```bash
# 构建 React 应用
npm run build

# 构建 Keycloak 主题 JAR
npm run build-keycloak-theme

# 输出文件
ls dist_keycloak/
# keycloak-theme-auth9.jar
```

### 3.2 Docker 构建 (推荐)

```bash
# 构建 Docker 镜像
docker build -t auth9-keycloak-theme .

# 提取 JAR 文件
mkdir -p output
docker run --rm -v $(pwd)/output:/theme-output auth9-keycloak-theme

# 查看输出
ls output/
# keycloak-theme-auth9.jar
```

### 3.3 Docker Compose 构建

```bash
# 在项目根目录执行
cd /path/to/auth9

# 构建主题 (使用 build profile)
docker-compose --profile build up auth9-theme-builder

# 主题 JAR 会被复制到 keycloak-theme volume
```

## 4. 部署

### 4.1 Docker Compose 部署

主题已集成到 `docker-compose.yml`：

```yaml
services:
  # 主题构建服务
  auth9-theme-builder:
    build:
      context: ./auth9-keycloak-theme
    volumes:
      - keycloak-theme:/theme-output
    profiles:
      - build

  # Keycloak 服务
  keycloak:
    image: quay.io/keycloak/keycloak:23.0
    environment:
      AUTH9_API_URL: http://auth9-core:8080
    volumes:
      - keycloak-theme:/opt/keycloak/providers:ro
```

**部署步骤：**

```bash
# 1. 首次构建主题
docker-compose --profile build up auth9-theme-builder

# 2. 启动所有服务
docker-compose up -d

# 3. 验证主题已加载
docker exec auth9-keycloak ls -la /opt/keycloak/providers/
```

### 4.2 手动部署到 Keycloak

```bash
# 1. 复制 JAR 到 Keycloak providers 目录
cp dist_keycloak/keycloak-theme-auth9.jar /opt/keycloak/providers/

# 2. 重启 Keycloak
# Docker:
docker restart auth9-keycloak

# 或 systemd:
sudo systemctl restart keycloak
```

### 4.3 激活主题

1. 登录 Keycloak Admin Console (http://localhost:8081)
2. 选择 Realm → **Realm Settings**
3. 切换到 **Themes** 标签
4. **Login Theme** 选择 `auth9`
5. 点击 **Save**

## 5. 配置

### 5.1 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `AUTH9_API_URL` | `http://localhost:8080` | auth9-core API 地址 |

在 docker-compose.yml 或 Kubernetes 中设置：

```yaml
environment:
  AUTH9_API_URL: http://auth9-core:8080
```

### 5.2 品牌配置 API

主题从以下端点获取品牌配置：

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
    "custom_css": ".login-form { border-radius: 8px; }",
    "company_name": "My Company",
    "favicon_url": "https://example.com/favicon.ico"
  }
}
```

### 5.3 支持的品牌选项

| 字段 | 类型 | 说明 |
|------|------|------|
| `logo_url` | string? | Logo 图片 URL |
| `primary_color` | string | 主色 (按钮、链接) |
| `secondary_color` | string | 辅色 (次要元素) |
| `background_color` | string | 页面背景色 |
| `text_color` | string | 文字颜色 |
| `custom_css` | string? | 自定义 CSS (最大 50KB) |
| `company_name` | string? | 公司名称 (无 Logo 时显示) |
| `favicon_url` | string? | 网站图标 URL |

## 6. 自定义页面

### 6.1 已自定义的页面

| 页面 | 文件 | 说明 |
|------|------|------|
| 登录 | `Login.tsx` | 用户名/密码登录 |
| 注册 | `Register.tsx` | 新用户注册 |
| 重置密码 | `LoginResetPassword.tsx` | 请求密码重置邮件 |
| OTP 验证 | `LoginOtp.tsx` | 一次性密码输入 |

其他页面使用 Keycloakify 默认实现。

### 6.2 添加新的自定义页面

```bash
# 1. 使用 Keycloakify CLI eject 页面
npx keycloakify eject-page

# 2. 选择要自定义的页面 (如 login-update-password.ftl)

# 3. 在 src/login/pages/ 创建对应组件

# 4. 在 KcPage.tsx 中添加路由
```

## 7. 故障排查

### 7.1 主题未显示

```bash
# 检查 JAR 是否正确挂载
docker exec auth9-keycloak ls -la /opt/keycloak/providers/

# 检查 Keycloak 日志
docker logs auth9-keycloak 2>&1 | grep -i theme
```

### 7.2 品牌配置未加载

```bash
# 测试 API 可访问性
curl http://localhost:8080/api/v1/public/branding

# 检查浏览器控制台是否有 CORS 错误
# auth9-core 已配置允许所有来源访问 /api/v1/public/* 端点
```

### 7.3 样式不生效

1. 确认颜色格式正确 (如 `#007AFF`，6位十六进制)
2. 检查 custom_css 是否有语法错误
3. 清除浏览器缓存后刷新

### 7.4 构建失败

```bash
# 清理并重新安装依赖
rm -rf node_modules dist dist_keycloak
npm install

# 检查 TypeScript 错误
npx tsc --noEmit

# 查看详细构建日志
npm run build 2>&1
```

## 8. CI/CD 集成

### 8.1 GitHub Actions 示例

```yaml
name: Build Keycloak Theme

on:
  push:
    paths:
      - 'auth9-keycloak-theme/**'
    branches: [main]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'
          cache-dependency-path: auth9-keycloak-theme/package-lock.json

      - name: Install dependencies
        working-directory: auth9-keycloak-theme
        run: npm ci

      - name: Build theme
        working-directory: auth9-keycloak-theme
        run: npm run build

      - name: Build Docker image
        run: docker build -t auth9-keycloak-theme auth9-keycloak-theme/

      - name: Extract JAR
        run: |
          mkdir -p output
          docker run --rm -v $(pwd)/output:/theme-output auth9-keycloak-theme

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: keycloak-theme-jar
          path: output/*.jar
```

### 8.2 部署到生产环境

1. 构建主题 JAR (通过 CI/CD)
2. 将 JAR 上传到共享存储或容器镜像
3. 在 Kubernetes 中通过 InitContainer 或 ConfigMap 挂载
4. 重启 Keycloak Pod

## 9. 更新主题

### 9.1 修改后重新部署

```bash
# 1. 修改源代码

# 2. 重新构建
docker-compose --profile build up --build auth9-theme-builder

# 3. 重启 Keycloak 加载新主题
docker-compose restart keycloak
```

### 9.2 热更新品牌配置

品牌配置（颜色、Logo 等）不需要重新构建主题：

1. 在 Portal 修改品牌设置
2. 刷新登录页面即可看到新配置

只有修改主题代码（布局、组件逻辑）才需要重新构建和部署。

## 10. 常见问题

### Q: 重新构建主题后需要在 Keycloak 中重新配置吗？

**不需要**。主题选择（如 "auth9"）存储在 Keycloak 数据库中（每个 realm 的设置）。
JAR 文件只提供主题的实际文件（HTML、CSS、JS）。

当你：
1. 重新构建主题 JAR
2. 重启 Keycloak

Keycloak 会自动使用新版本的主题，因为 realm 配置指向 "auth9"（按名称），
新的 JAR 提供更新后的 "auth9" 主题文件。

只有当你**更改主题名称**（如从 "auth9" 改为 "auth9-v2"）时，才需要更新 realm 配置。

### Q: 新创建的 realm 需要手动选择 auth9 主题吗？

**不需要**。在 `docker-compose.yml` 中配置了 `KC_SPI_THEME_DEFAULT: auth9`，
所有新创建的 realm 都会自动使用 auth9 作为默认登录主题。

### Q: 如何控制登录页面是否显示注册链接？

在 Portal 的 **Dashboard > Settings > Login Branding** 页面中，有一个 **Allow Registration** 开关。
默认情况下此开关关闭，登录页面不会显示 "Create account" 链接。

注意：此设置独立于 Keycloak realm 的 "User Registration" 设置。要使注册功能完全可用，需要：
1. 在 Keycloak realm 设置中启用 "User Registration"
2. 在 auth9 品牌设置中开启 "Allow Registration"
