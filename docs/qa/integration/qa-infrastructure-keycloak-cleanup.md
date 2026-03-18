# Infrastructure Keycloak Cleanup - Phase 5 FR5 基础设施清理验证

**模块**: 集成测试 / 基础设施
**测试范围**: Docker Compose、K8s 配置、Portal 登录模式、健康检查、脚本清理 — 验证所有 Keycloak 残留已从基础设施层移除
**场景数**: 5
**优先级**: 高

---

## 背景说明

Phase 5 FR5 是 Keycloak 退役计划的最后一步：清理基础设施层（Docker Compose、K8s 配置、部署脚本、Portal 环境变量）中所有 Keycloak 相关引用。清理后：

- `docker-compose.yml` 不再包含 Keycloak 服务定义或 `KEYCLOAK_*` 环境变量
- `deploy/k8s/configmap.yaml` 和 `secrets.yaml.example` 不含 Keycloak 配置
- `scripts/reset-docker.sh` 不含 Keycloak 容器/卷清理逻辑
- Portal 默认以 hosted 模式运行，无需 `LOGIN_MODE` 环境变量
- auth9-core 在无 Keycloak 依赖的情况下正常启动并通过健康检查

---

## 场景 1：Docker Compose 无 Keycloak 依赖

### 步骤 0：Gate Check（编译与静态验证）

在项目根目录执行以下命令，确认 Docker Compose 配置中不含 Keycloak 引用：

```bash
# 1. 检查 docker-compose.yml 中无 keycloak 关键字（不区分大小写）
grep -i keycloak docker-compose.yml docker-compose.dev.yml docker-compose.observability.yml 2>/dev/null
# 预期: 无输出（exit code 1）

# 2. 渲染完整配置并搜索
docker compose -f docker-compose.yml -f docker-compose.dev.yml config 2>/dev/null | grep -i keycloak
# 预期: 无输出（exit code 1）

# 3. 确认无 KEYCLOAK_* 环境变量
docker compose -f docker-compose.yml -f docker-compose.dev.yml config 2>/dev/null | grep -i 'KEYCLOAK_'
# 预期: 无输出（exit code 1）
```

### 初始状态
- 项目代码已拉取到最新 `feat/auth9-oidc-replacement-program` 分支
- Docker 环境可用

### 目的
验证 Docker Compose 配置文件已完全移除 Keycloak 服务定义和环境变量引用

### 测试操作流程
1. 在项目根目录执行 Gate Check 中的三条命令
2. 检查 `docker-compose.yml` 中的 `services:` 段，确认不存在 `keycloak` 服务
3. 检查所有服务的 `environment` 段，确认不存在 `KEYCLOAK_URL`、`KEYCLOAK_ADMIN_URL`、`KEYCLOAK_CLIENT_ID` 等变量
4. 检查 `depends_on` 段，确认无服务依赖 `keycloak`
5. 启动服务并确认无 Keycloak 容器：
   ```bash
   docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d
   docker ps --format 'table {{.Names}}\t{{.Status}}' | grep -i keycloak
   # 预期: 无输出
   ```

### 预期结果
- `grep -i keycloak` 在所有 Docker Compose 文件中无匹配
- 渲染后的完整配置中无 `keycloak` 字符串
- 无 `KEYCLOAK_*` 环境变量
- 启动后无 Keycloak 容器运行
- auth9-core 的 `depends_on` 仅包含 `auth9-init` 和 `redis`

---

## 场景 2：Portal 登录模式简化

### 初始状态
- Docker 环境已启动（场景 1 完成）
- Portal 服务运行中（`http://localhost:3000`）

### 目的
验证 Portal 在无 `LOGIN_MODE` 环境变量的情况下默认以 hosted 模式运行，登录页面不触发 OIDC 重定向

### 测试操作流程
1. 确认 Portal 容器无 `LOGIN_MODE` 环境变量：
   ```bash
   docker compose -f docker-compose.yml config | grep -A 50 'auth9-portal' | grep -i 'LOGIN_MODE'
   # 预期: 无输出
   ```
2. 打开浏览器访问 `http://localhost:3000/login`
3. 观察页面加载行为 — 应直接渲染 Auth9 品牌认证页（hosted 登录表单），**不应**发生 302 重定向到外部 OIDC Provider
4. 检查浏览器 Network 面板：
   - 首次请求 `GET /login` 应返回 `200 OK`
   - 不应出现 `302 Location: http://localhost:8081/realms/...` 或任何 Keycloak URL
5. 确认登录表单包含邮箱和密码输入框

### 预期结果
- Portal 容器的环境变量中不存在 `LOGIN_MODE`
- `/login` 页面直接返回 200 并渲染 hosted 登录表单
- 无 OIDC 重定向发生（无 302 到 Keycloak）
- 登录表单包含邮箱输入框、密码输入框和「Sign in」按钮

---

## 场景 3：API 健康检查

### 步骤 0：Gate Check

```bash
# 确认 auth9-core 进程正在运行
docker ps --format '{{.Names}} {{.Status}}' | grep auth9-core
# 预期: auth9-core Up ... (healthy)
```

### 初始状态
- Docker 环境已启动，auth9-core 运行中

### 目的
验证 auth9-core 在移除 Keycloak 依赖后，健康检查端点正常工作

### 测试操作流程
1. 执行 `/health` 端点检查：
   ```bash
   curl -s http://localhost:8080/health | jq .
   # 预期: {"status":"ok"} 或包含 status: "ok" 的 JSON
   ```
2. 执行 `/ready` 端点检查：
   ```bash
   curl -s http://localhost:8080/ready | jq .
   # 预期: 200 OK，包含各依赖状态
   ```
3. 确认 `/ready` 响应中无 Keycloak 依赖项：
   ```bash
   curl -s http://localhost:8080/ready | grep -i keycloak
   # 预期: 无输出（无 keycloak 依赖项）
   ```
4. 验证 auth9-core 启动日志无 Keycloak 连接错误：
   ```bash
   docker logs auth9-core 2>&1 | grep -i keycloak
   # 预期: 无输出（无 Keycloak 相关日志）
   ```

### 预期结果
- `/health` 返回 200，`status` 为 `ok`
- `/ready` 返回 200，列出 database 和 redis 依赖状态，**不含** keycloak
- 启动日志中无 Keycloak 连接尝试或错误

---

## 场景 4：K8s 配置清理验证

### 步骤 0：Gate Check（静态文件扫描）

```bash
# 1. configmap.yaml 无 keycloak 引用
grep -i keycloak deploy/k8s/configmap.yaml
# 预期: 无输出（exit code 1）

# 2. secrets.yaml.example 无 keycloak 引用
grep -i keycloak deploy/k8s/secrets.yaml.example
# 预期: 无输出（exit code 1）

# 3. 整个 K8s 目录无 keycloak 引用
grep -ri keycloak deploy/k8s/
# 预期: 无输出（exit code 1）

# 4. 无 keycloak 子目录
ls deploy/k8s/keycloak/ 2>/dev/null
# 预期: 目录不存在
```

### 初始状态
- 项目代码已拉取到最新分支

### 目的
验证 K8s 部署配置中已移除所有 Keycloak 相关配置（ConfigMap、Secrets、Deployment）

### 测试操作流程
1. 执行 Gate Check 中的四条命令
2. 检查 `deploy/k8s/configmap.yaml`：
   - 不含 `KEYCLOAK_URL`、`KEYCLOAK_ADMIN_URL`、`KEYCLOAK_REALM` 等键
   - 不含 `IDENTITY_BACKEND` 键（已移除，默认 auth9_oidc）
3. 检查 `deploy/k8s/secrets.yaml.example`：
   - 不含 `KEYCLOAK_ADMIN_PASSWORD`、`KEYCLOAK_CLIENT_SECRET` 等键
4. 确认不存在 `deploy/k8s/keycloak/` 子目录（无 Keycloak Deployment/Service YAML）
5. 检查 `deploy/k8s/auth9-core/deployment.yaml` 中 `envFrom` 或 `env` 不引用 keycloak 相关 ConfigMap/Secret：
   ```bash
   grep -i keycloak deploy/k8s/auth9-core/deployment.yaml
   # 预期: 无输出
   ```

### 预期结果
- `configmap.yaml` 中无任何 `keycloak` 或 `KEYCLOAK_` 关键字
- `secrets.yaml.example` 中无 Keycloak 凭证字段
- `deploy/k8s/keycloak/` 目录不存在
- auth9-core Deployment 不引用 Keycloak ConfigMap 或 Secret
- `IDENTITY_BACKEND` 键不再出现在 ConfigMap 中（已成为硬编码默认值）

---

## 场景 5：脚本清理验证

### 步骤 0：Gate Check

```bash
# 1. reset-docker.sh 无 keycloak 引用
grep -i keycloak scripts/reset-docker.sh
# 预期: 无输出（exit code 1）

# 2. 已删除的 Keycloak 辅助脚本不存在
ls scripts/kc-* scripts/*keycloak* 2>/dev/null
# 预期: 无文件（exit code 非 0）

# 3. 整个 scripts 目录无 keycloak 引用（排除 __pycache__）
grep -ri keycloak scripts/ --include='*.sh' --include='*.py' --include='*.ts' 2>/dev/null
# 预期: 无输出（exit code 1）
```

### 初始状态
- 项目代码已拉取到最新分支

### 目的
验证部署和运维脚本中已移除所有 Keycloak 相关逻辑，已删除的辅助脚本不再存在

### 测试操作流程
1. 执行 Gate Check 中的三条命令
2. 检查 `scripts/reset-docker.sh`：
   - 不含 `keycloak` 容器停止/删除命令
   - 不含 `keycloak-data` 卷清理
   - `docker volume rm` 命令中不含 keycloak 相关卷名
3. 确认以下文件/脚本已被删除（不存在）：
   ```bash
   # 以下文件应均不存在
   test -f scripts/kc-export.sh && echo "EXISTS" || echo "DELETED"
   test -f scripts/kc-import.sh && echo "EXISTS" || echo "DELETED"
   test -f scripts/keycloak-setup.sh && echo "EXISTS" || echo "DELETED"
   # 预期: 全部输出 DELETED
   ```
4. 验证 `reset-docker.sh` 可正常执行（dry-run 级别 — 仅检查语法）：
   ```bash
   bash -n scripts/reset-docker.sh
   # 预期: 无语法错误（exit code 0）
   ```

### 预期结果
- `reset-docker.sh` 中无任何 `keycloak` 字符串
- 所有 Keycloak 辅助脚本（`kc-export.sh`、`kc-import.sh`、`keycloak-setup.sh` 等）已删除
- `scripts/` 目录下无任何脚本文件包含 `keycloak` 引用
- `reset-docker.sh` 语法检查通过

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | Docker Compose 无 Keycloak 依赖 | ☐ | | | Gate Check + 运行时验证 |
| 2 | Portal 登录模式简化 | ☐ | | | 需浏览器验证 |
| 3 | API 健康检查 | ☐ | | | curl 命令验证 |
| 4 | K8s 配置清理验证 | ☐ | | | 纯静态文件扫描 |
| 5 | 脚本清理验证 | ☐ | | | 纯静态文件扫描 + 语法检查 |
