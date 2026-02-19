# 集成测试 - Init 命令初始种子数据

**模块**: 集成测试
**测试范围**: `auth9-core init` 命令的初始种子数据功能，包括平台租户、演示租户、管理员用户、租户用户关联、租户服务关联的创建与幂等性
**场景数**: 5
**优先级**: 高

---

## 背景说明

`auth9-core init` 命令在 Keycloak 初始化完成后，自动向数据库注入初始种子数据，使系统在首次部署后即可正常使用 Portal 管理后台。

种子数据包括：
- **2 个租户**: "Auth9 Platform"（slug: `auth9-platform`）和 "Demo Organization"（slug: `demo`）
- **1 个管理员用户**: 从 Keycloak 获取 `keycloak_id` 和 `email`，display_name 为 "Admin User"
- **2 条 tenant_users**: 管理员关联到两个租户，角色为 `admin`
- **4 条 tenant_services**: 两个租户均启用 "Auth9 Admin Portal" 服务（公共服务），demo 租户额外启用 "Auth9 Demo Service"（私有服务）和 "Auth9 M2M Test Service"（私有服务）

> **服务类型说明**：私有服务的 `tenant_id` 有值（专属某租户），公共服务的 `tenant_id` 为 NULL（不专属任何租户，所有租户可通过 tenant_services 关联使用）

管理员邮箱可通过 `AUTH9_ADMIN_EMAIL` 环境变量配置，默认为 `admin@auth9.local`。

---

## 数据库表结构参考

### tenants 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| name | VARCHAR(255) | 租户名称 |
| slug | VARCHAR(63) | URL 友好标识（UNIQUE） |
| settings | JSON | 租户设置 |
| status | VARCHAR(20) | 状态（默认 active） |

### users 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| keycloak_id | VARCHAR(255) | Keycloak 用户 ID（UNIQUE） |
| email | VARCHAR(255) | 用户邮箱（UNIQUE） |
| display_name | VARCHAR(255) | 显示名称 |

### tenant_users 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| tenant_id | CHAR(36) | 租户 ID |
| user_id | CHAR(36) | 用户 ID |
| role_in_tenant | VARCHAR(50) | 租户内角色 |
| | | UNIQUE KEY (tenant_id, user_id) |

### tenant_services 表
| 字段 | 类型 | 说明 |
|------|------|------|
| tenant_id | CHAR(36) | 租户 ID（联合主键） |
| service_id | CHAR(36) | 服务 ID（联合主键） |
| enabled | BOOLEAN | 是否启用 |

---

## 场景 1：首次 Init 创建全部种子数据

### 初始状态
- Docker Compose 环境已启动（TiDB、Redis、Keycloak）
- 数据库已重置（执行过 `auth9-core reset` 或为全新环境）
- tenants、users、tenant_users、tenant_services 表均为空

### 目的
验证 `auth9-core init` 在全新环境中正确创建所有种子数据

### 测试操作流程
1. 重置本地环境：
   ```bash
   docker-compose exec auth9-core auth9-core reset
   ```
2. 执行初始化：
   ```bash
   docker-compose exec auth9-core auth9-core init
   ```
3. 观察日志输出，确认以下关键行：
   - `Found admin user in Keycloak: keycloak_id=..., email=...`
   - `Initial data seeded: tenants=[auth9-platform, demo], admin_user=..., email=...`
   - `Seeded tenant_services for both tenants → Auth9 Admin Portal`
4. 连接数据库验证数据

### 预期结果
- Init 命令成功完成，无错误
- 日志中显示种子数据创建成功

### 预期数据状态
```sql
-- 验证租户（2 行）
SELECT name, slug, status, JSON_EXTRACT(settings, '$.require_mfa') as require_mfa
FROM tenants WHERE slug IN ('auth9-platform', 'demo') ORDER BY slug;
-- 预期:
-- | Auth9 Platform   | auth9-platform | active | false |
-- | Demo Organization | demo           | active | false |

-- 验证管理员用户（1 行）
SELECT keycloak_id, email, display_name, mfa_enabled
FROM users WHERE display_name = 'Admin User';
-- 预期: keycloak_id 非空, email = admin@auth9.local（或 AUTH9_ADMIN_EMAIL 值）, mfa_enabled = 0

-- 验证租户用户关联（2 行）
SELECT t.slug, tu.role_in_tenant
FROM tenant_users tu
JOIN tenants t ON t.id = tu.tenant_id
JOIN users u ON u.id = tu.user_id
WHERE u.display_name = 'Admin User'
ORDER BY t.slug;
-- 预期:
-- | auth9-platform | admin |
-- | demo           | admin |

-- 验证租户服务关联（4 行）
SELECT t.slug, s.name, ts.enabled
FROM tenant_services ts
JOIN tenants t ON t.id = ts.tenant_id
JOIN services s ON s.id = ts.service_id
WHERE t.slug IN ('auth9-platform', 'demo')
ORDER BY t.slug, s.name;
-- 预期:
-- | auth9-platform | Auth9 Admin Portal     | 1 |
-- | demo           | Auth9 Admin Portal     | 1 |
-- | demo           | Auth9 Demo Service     | 1 |
-- | demo           | Auth9 M2M Test Service | 1 |
```

---

## 场景 2：重复执行 Init 保证幂等性

### 初始状态
- 场景 1 已成功执行
- 数据库中已存在种子数据

### 目的
验证多次执行 `auth9-core init` 不会产生重复数据或错误

### 测试操作流程
1. 再次执行初始化：
   ```bash
   docker-compose exec auth9-core auth9-core init
   ```
2. 观察日志，确认无错误输出
3. 第三次执行初始化：
   ```bash
   docker-compose exec auth9-core auth9-core init
   ```
4. 连接数据库验证数据未重复

### 预期结果
- 两次重复执行均成功完成，无错误
- 日志中可能显示 "already exists" 相关信息
- 数据行数不变

### 预期数据状态
```sql
-- 验证租户仍然只有 2 行（slug 为 seed 创建的）
SELECT COUNT(*) FROM tenants WHERE slug IN ('auth9-platform', 'demo');
-- 预期: 2

-- 验证管理员用户仍然只有 1 行
SELECT COUNT(*) FROM users WHERE display_name = 'Admin User';
-- 预期: 1

-- 验证 tenant_users 仍然只有 2 行
SELECT COUNT(*)
FROM tenant_users tu
JOIN users u ON u.id = tu.user_id
WHERE u.display_name = 'Admin User';
-- 预期: 2

-- 验证 tenant_services 仍然只有 4 行
SELECT COUNT(*)
FROM tenant_services ts
JOIN tenants t ON t.id = ts.tenant_id
WHERE t.slug IN ('auth9-platform', 'demo');
-- 预期: 4
```

---

## 场景 3：自定义管理员邮箱（AUTH9_ADMIN_EMAIL）

### 初始状态
- Docker Compose 环境已启动
- 数据库已重置

### 目的
验证通过 `AUTH9_ADMIN_EMAIL` 环境变量可以自定义管理员邮箱，且种子数据使用 Keycloak 中的实际邮箱

### 测试操作流程
1. 重置环境：
   ```bash
   docker-compose exec auth9-core auth9-core reset
   ```
2. 设置自定义邮箱并执行初始化：
   ```bash
   docker-compose exec -e AUTH9_ADMIN_EMAIL=ops@example.com auth9-core auth9-core init
   ```
3. 观察日志中管理员邮箱信息
4. 连接数据库验证

### 预期结果
- Init 成功完成
- Keycloak 中管理员用户使用指定邮箱创建
- 数据库中用户邮箱与 Keycloak 一致

### 预期数据状态
```sql
-- 验证用户邮箱来自 Keycloak（与 AUTH9_ADMIN_EMAIL 设置一致）
SELECT email, display_name FROM users WHERE display_name = 'Admin User';
-- 预期: email = ops@example.com, display_name = Admin User

-- 验证 Keycloak 中的邮箱一致
-- 通过 Keycloak Admin Console (http://localhost:8081/admin) 查看
-- Realm: auth9 → Users → admin → Email 字段应为 ops@example.com
```

---

## 场景 4：Keycloak 重置后重新 Init（keycloak_id 更新）

### 初始状态
- 场景 1 已成功执行，数据库中存在种子数据
- 记录当前管理员的 keycloak_id

### 目的
验证 Keycloak PVC 被删除重建后（新 keycloak_id），重新运行 init 能正确更新数据库中的 keycloak_id 关联

### 测试操作流程
1. 记录当前 keycloak_id：
   ```sql
   SELECT keycloak_id, email FROM users WHERE display_name = 'Admin User';
   ```
2. 停止 Keycloak 并删除数据（模拟 PVC 重置）：
   ```bash
   docker-compose stop keycloak
   docker volume rm auth9_keycloak-postgres-data
   docker-compose up -d keycloak
   ```
3. 等待 Keycloak 就绪后，重新执行初始化：
   ```bash
   docker-compose exec auth9-core auth9-core init
   ```
4. 查询新的 keycloak_id

### 预期结果
- Init 成功完成
- 管理员用户的 keycloak_id 被更新为 Keycloak 中的新 UUID
- email、display_name 等其他字段保持不变
- tenant_users 关联不受影响（通过 user_id 关联，非 keycloak_id）

### 预期数据状态
```sql
-- 验证 keycloak_id 已更新
SELECT keycloak_id, email, display_name FROM users WHERE display_name = 'Admin User';
-- 预期: keycloak_id 与步骤 1 记录的不同（新 UUID），email 保持不变

-- 验证用户仍然只有 1 行（不会创建重复用户）
SELECT COUNT(*) FROM users WHERE display_name = 'Admin User';
-- 预期: 1

-- 验证租户关联仍然有效
SELECT COUNT(*)
FROM tenant_users tu
JOIN users u ON u.id = tu.user_id
WHERE u.display_name = 'Admin User';
-- 预期: 2
```

---

## 场景 5：Portal 登录验证种子数据可用性

### 初始状态
- 场景 1 已成功执行
- auth9-core 和 auth9-portal 均已启动

### 目的
验证种子数据创建后，管理员可以通过 Portal 正常登录并看到租户列表

### 测试操作流程
1. 打开浏览器访问 Portal：http://localhost:3000
2. 点击「登录」按钮，页面跳转至 Keycloak 登录页
3. 输入管理员凭据（用户名: `admin`，密码: Init 时生成或通过 `AUTH9_ADMIN_PASSWORD` 设置的密码）
4. 登录成功后，若进入 `/tenant/select`，先选择任一 tenant 完成 token exchange
5. 验证 Dashboard 页面
6. 查看租户列表

### 预期结果
- 登录流程正常完成，无错误
- 多 tenant 账号先进入 `/tenant/select`；完成选择后 Dashboard 页面正常加载
- 租户列表中显示 "Auth9 Platform" 和 "Demo Organization" 两个租户
- 管理员在两个租户中均为 `admin` 角色
- 切换租户功能正常

### 预期数据状态
```sql
-- 登录后应创建会话记录
SELECT u.email, s.created_at
FROM sessions s
JOIN users u ON u.id = s.user_id
WHERE u.display_name = 'Admin User'
ORDER BY s.created_at DESC LIMIT 1;
-- 预期: 有最近的会话记录
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 首次 Init 创建全部种子数据 | ☐ | | | |
| 2 | 重复执行 Init 保证幂等性 | ☐ | | | |
| 3 | 自定义管理员邮箱（AUTH9_ADMIN_EMAIL） | ☐ | | | |
| 4 | Keycloak 重置后重新 Init（keycloak_id 更新） | ☐ | | | |
| 5 | Portal 登录验证种子数据可用性 | ☐ | | | |
