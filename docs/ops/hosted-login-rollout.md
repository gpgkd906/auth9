# Hosted Login 灰度上线运维手册

## 概述

Auth9 Hosted Login 替代 Keycloak 的浏览器登录页面。本手册描述灰度上线步骤、监控指标和回滚流程。

## 开关位置

| 环境变量 | 值 | 说明 |
|----------|------|------|
| `LOGIN_MODE` | `hosted` | Auth9 自有登录页面（默认） |
| `LOGIN_MODE` | `oidc` | 重定向到 Keycloak 登录（回退） |
| `LOGIN_MODE` | `percentage` | 按百分比灰度，配合 `LOGIN_ROLLOUT_PCT` |
| `LOGIN_ROLLOUT_PCT` | `0`–`100` | 灰度百分比（仅 `percentage` 模式有效） |

**配置位置**:
- Docker: `docker-compose.yml` → `auth9-portal.environment`
- K8s: `deploy/k8s/configmap.yaml` → `LOGIN_MODE` / `LOGIN_ROLLOUT_PCT`

## 灰度上线步骤

### Phase 1: 内部验证 (0%)
```yaml
LOGIN_MODE: "oidc"  # 全量走 Keycloak
```
- 部署新版本，确认无启动错误
- 验证 Prometheus 指标注册正常

### Phase 2: 小范围灰度 (10%)
```yaml
LOGIN_MODE: "percentage"
LOGIN_ROLLOUT_PCT: "10"
```
- 观察 10% 流量走 Auth9 hosted login
- 监控登录成功率和延迟

### Phase 3: 扩大灰度 (50%)
```yaml
LOGIN_ROLLOUT_PCT: "50"
```
- 对比 `backend=hosted` vs `backend=oidc` 的 P50/P95 延迟
- 确认无异常告警

### Phase 4: 全量切换 (100%)
```yaml
LOGIN_MODE: "hosted"
```
- 全量使用 Auth9 hosted login
- 保留 Keycloak 服务运行（热备）

## 观测指标

### Prometheus 查询

**登录成功率 (按 backend)**:
```promql
rate(auth9_auth_login_total{result="success"}[5m])
```

**登录失败率 (按 backend)**:
```promql
rate(auth9_auth_login_total{result="failure"}[5m])
```

**Hosted login 延迟 P95**:
```promql
histogram_quantile(0.95, rate(auth9_hosted_login_duration_seconds_bucket[5m]))
```

**两种模式对比**:
```promql
sum by (backend) (rate(auth9_auth_login_total[5m]))
```

### Grafana Dashboard

指标已注册在 `auth9-auth` dashboard 中。新增面板建议：
- 按 `backend` 分组的登录请求速率
- Hosted login P50/P95 延迟趋势

### 告警

现有告警 `HighLoginFailureRate`（登录失败率 > 30%）仍然有效，会覆盖两种 backend。

## 回滚流程

### 触发条件
- Hosted login 错误率 > 5%（持续 2 分钟）
- Hosted login P95 延迟 > 3s（持续 5 分钟）
- 用户大量报告无法登录

### 回滚步骤

1. **修改配置**:
   ```bash
   # Docker
   export LOGIN_MODE=oidc
   docker-compose up -d auth9-portal

   # K8s
   kubectl -n auth9 set env deployment/auth9-portal LOGIN_MODE=oidc
   ```

2. **验证回滚**:
   - 访问 `/login` 确认重定向到 Keycloak
   - 检查 `/dashboard` 确认已有 session 仍有效

3. **无需数据迁移**:
   - `sessions` 表在两种模式下共享
   - JWT 令牌由 auth9-core 签发，与登录模式无关
   - 用户数据无差异

### 回滚耗时
- Docker: ~10 秒（容器重启）
- K8s: ~30 秒（滚动更新）
