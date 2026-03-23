# 21 - Hosted Login 灰度上线与回滚

**模块**: auth / rollout
**前置条件**: Docker 环境运行中，Portal 和 auth9-core 可访问
**关联 FR**: Hosted Login 灰度上线与回滚

> **⚠️ 实装状態**: 场景 2-5 描述的 `LOGIN_MODE` 环境变量切换功能（`oidc` / `percentage` 模式）**尚未实装**。当前 Portal 仅支持 hosted 模式（场景 1）。场景 2-5 应标记为 **SKIP（未实装）** 而非 FAIL。

---

## 场景 1: 默认模式 (hosted) — Auth9 登录表单渲染

**目标**: 验证 `LOGIN_MODE=hosted`（默认）时，用户看到 Auth9 自有登录页面

### 步骤

1. 确认环境变量 `LOGIN_MODE` 未设置或设为 `hosted`
2. 访问 `http://localhost:3000/login`
3. 验证页面包含密码登录表单、SSO 登录入口、Passkey 按钮
4. 验证页面 **不会** 重定向到外部登录界面

### 预期结果

- 页面 URL 保持在 `localhost:3000/login`
- 页面显示 Auth9 品牌化的登录表单
- 网络请求中无对外部认证服务域名的浏览器直接跳转

---

## 场景 2: OIDC 回退模式 — 重定向到 OIDC 授权流程

**目标**: 验证 `LOGIN_MODE=oidc` 时，用户被重定向到 OIDC 授权流程（注：Keycloak 已退役，OIDC 流程由 Auth9 内置引擎处理）

### 步骤

1. 设置环境变量 `LOGIN_MODE=oidc`，重启 Portal：
   ```bash
   # 在 docker-compose.dev.yml 的 portal 服务中添加/修改 environment 配置：
   #   environment:
   #     LOGIN_MODE: oidc
   # 然后重启：
   docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d
   ```
   > **注意**: 容器文件系统为只读，不要尝试修改容器内的 `/app/.env` 文件。必须通过 `docker-compose.dev.yml` 的 `environment` 字段覆盖环境变量。
2. 访问 `http://localhost:3000/login`
3. 验证页面被 302 重定向到 auth9-core 的 `/api/v1/auth/authorize`

### 预期结果

- 浏览器被重定向到 Auth9 OIDC 授权端点
- URL 包含 `response_type=code`、`client_id`、`redirect_uri` 等 OIDC 参数
- `Set-Cookie` 头包含 `oauth_state` cookie（用于回调验证）

---

## 场景 3: 百分比模式 — 灰度分流

**目标**: 验证 `LOGIN_MODE=percentage` + `LOGIN_ROLLOUT_PCT` 时，流量按比例分配

### 步骤

1. 在 `docker-compose.dev.yml` 的 portal 服务中设置 `LOGIN_MODE: percentage` 和 `LOGIN_ROLLOUT_PCT: "50"`，然后 `docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d` 重启 Portal
2. 使用不同的 User-Agent 和 IP 组合多次访问 `/login`
3. 统计显示 Auth9 登录表单 vs OIDC 重定向的比例

### 预期结果

- 部分请求显示 Auth9 登录表单（200 响应）
- 部分请求被 302 重定向到 OIDC 流程
- 相同 IP + User-Agent 组合始终得到相同结果（确定性哈希）

---

## 场景 4: 登录性能指标观测

**目标**: 验证 Prometheus 指标中包含 `backend` 维度标签

### 步骤 0 (Gate Check)

```bash
# 确认 metrics 端点可访问
curl -sf http://localhost:8080/metrics | head -5
```

### 步骤

1. 通过 Auth9 hosted login 执行一次密码登录
2. 查询 Prometheus 指标：

```bash
curl -s http://localhost:8080/metrics | grep auth9_auth_login_total
```

3. 验证指标输出：

```
auth9_auth_login_total{backend="hosted",result="success"} 1
```

4. 查询 hosted login 耗时指标：

```bash
curl -s http://localhost:8080/metrics | grep auth9_hosted_login_duration
```

### 预期结果

- `auth9_auth_login_total` 包含 `backend="hosted"` 和 `backend="oidc"` 两种标签
- `auth9_hosted_login_duration_seconds` 包含 `method="password"` 标签
- 指标值随登录操作正确递增

---

## 场景 5: 回滚验证 — 切换 LOGIN_MODE 立即生效

**目标**: 验证从 hosted 模式切换到 oidc 模式不需要数据迁移

### 步骤

1. 在 `LOGIN_MODE=hosted` 下正常登录一个用户，验证 session 正常
2. 在 `docker-compose.dev.yml` 中将 `LOGIN_MODE` 改为 `oidc`，运行 `docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d` 重启 Portal
3. 访问 `/login`，确认被重定向到 OIDC 授权流程
4. 验证之前创建的 session 仍然有效（访问 `/dashboard` 不需要重新登录）
5. 在 `docker-compose.dev.yml` 中将 `LOGIN_MODE` 改回 `hosted`，运行 `docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d` 重启 Portal
6. 访问 `/login`，确认回到 Auth9 登录表单

### 预期结果

- 切换 `LOGIN_MODE` 只需修改 `docker-compose.dev.yml` 中的环境变量 + 重启，无需数据库迁移
- 已有 session 在切换后仍然有效（session 表与登录模式无关）
- 回切到 hosted 后功能完全恢复
