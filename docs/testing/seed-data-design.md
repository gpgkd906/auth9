# Auth9 测试数据种子设计

本文档描述 Auth9 QA 测试和安全测试的专用测试数据（Seed Data）结构和生成策略。

---

## 1. 概述

### 1.1 目标

- 为 QA 手动测试提供一致的、可重复的测试数据
- 为安全测试提供包含已知漏洞配置的测试数据
- 为自动化测试（E2E、集成测试）提供数据基础
- 支持快速重置测试环境

### 1.2 数据分类

| 数据集 | 用途 | 数据量 | 特点 |
|--------|------|--------|------|
| **qa-basic** | 基础 QA 测试 | 小 (3 租户, 6 用户) | 简单场景，快速加载 |
| **qa-complex** | 高级 QA 测试 | 中 (50 租户, 1000 用户) | 复杂 RBAC 层级，多租户场景 |
| **security-vulnerable** | 安全测试 | 小 | 包含已知弱配置，用于渗透测试 |
| **performance** | 性能测试 | 大 (100 租户, 10K 用户) | 大数据量，用于压力测试 |

---

## 2. 数据文件格式

所有种子数据使用 YAML 格式定义，便于人类阅读和版本控制。数据文件位于 `scripts/seed-data/` 目录。

### 2.1 文件结构

```yaml
# 租户配置
tenants:
  - id: "tenant-id"
    slug: "tenant-slug"
    name: "Tenant Name"
    status: "active"
    password_policy: { ... }

# 用户配置
users:
  - username: "user1"
    email: "user1@example.com"
    password: "Password123!"
    tenants: [ ... ]

# 服务、客户端、权限、角色等
services: [ ... ]
clients: [ ... ]
permissions: [ ... ]
roles: [ ... ]
user_roles: [ ... ]
webhooks: [ ... ]
invitations: [ ... ]
system_settings: [ ... ]
```

---

## 3. QA 测试数据（qa-basic）

详细配置参考 `scripts/seed-data/qa-basic.yaml`

### 3.1 租户

- **qa-acme-corp**: 基本密码策略（8 位，大小写+数字）
- **qa-beta-inc**: 严格密码策略（12 位，大小写+数字+符号，历史检查）
- **qa-suspended-tenant**: 已暂停租户（用于测试暂停状态）

### 3.2 用户

- 租户管理员：完全控制租户资源
- 普通用户：受 RBAC 限制
- 跨租户用户：在多个租户中都有账户
- 未验证邮箱用户：用于测试邮箱验证流程

### 3.3 RBAC 配置

- **角色继承**：Viewer → Editor → Admin
- **权限粒度**：users:read, users:write, users:delete, reports:read, reports:write

---

## 4. 安全测试数据（security-vulnerable）

详细配置参考 `scripts/seed-data/security-vulnerable.yaml`

### 4.1 弱配置租户

- 最小密码长度 1 位
- 无字符类型要求
- 锁定阈值 999（几乎不锁定）

### 4.2 攻击测试用户

- **SQL 注入**: `admin' OR '1'='1` (用户名)
- **XSS**: `<script>alert('XSS')</script>` (显示名)
- **路径遍历**: `../../etc/passwd` (用户名)

### 4.3 配置错误客户端

- 通配符重定向 URI: `http://localhost:*`
- 恶意重定向: `http://evil.com/callback`
- 弱 client_secret

### 4.4 SSRF 测试 Webhook

- AWS 元数据端点: `http://169.254.169.254/latest/meta-data/`
- 内网 Redis: `http://localhost:6379`
- 内网数据库: `http://localhost:4000/admin`

---

## 5. 数据加载方式

### 5.1 使用脚本重置环境

```bash
./scripts/reset-test-env.sh
```

交互式选择数据集并自动清理旧数据。

### 5.2 手动加载（待实现）

```bash
# Rust 二进制
cd auth9-core
cargo run --bin seed-data -- --dataset=qa-basic --reset

# TypeScript 脚本
cd auth9-portal
npx ts-node scripts/seed-data.ts --config=../scripts/seed-data/qa-basic.yaml --reset
```

---

## 6. 测试账户速查

### qa-basic 账户

```
租户管理员:
  admin@qa-acme-corp.local / QaAcmeAdmin123!
  admin@qa-beta-inc.local / QaBetaAdmin456!

普通用户:
  user1@qa-acme-corp.local / QaUser123!
  user2@qa-acme-corp.local / QaUser123! (未验证邮箱)

跨租户用户:
  multi@qa-test.local / QaMulti123!
```

### security-vulnerable 账户

```
SQL 注入测试:
  sqli-test@security.local / SecTest123!

XSS 测试:
  xss-test@security.local / SecTest123!

弱密码测试:
  weak@security.local / 1 (仅 1 位密码!)
```

---

## 7. 数据清理规则

所有测试数据使用特定前缀标识：

- **租户**: `qa-*`, `sec-*`
- **用户邮箱**: `*@qa-test.local`, `*@security.local`
- **服务**: `qa-*`, `sec-*`

重置脚本会自动清理这些前缀的数据，不影响生产数据。

---

## 8. 扩展新数据集

1. 复制现有 YAML 文件：
   ```bash
   cp scripts/seed-data/qa-basic.yaml scripts/seed-data/my-dataset.yaml
   ```

2. 编辑 YAML，定义租户、用户、权限等

3. 实现数据加载器（Rust 或 TypeScript）

4. 更新 `scripts/reset-test-env.sh` 添加新选项

---

## 9. 注意事项

- **密码安全**: 测试数据中的密码仅用于测试，不要在生产环境使用
- **数据隔离**: 使用前缀标识测试数据，方便清理
- **Keycloak 同步**: 确保 Auth9 和 Keycloak 数据一致性
- **定期清理**: CI 自动化测试后应清理测试数据

---

## 10. 相关文档

- [QA 测试用例](../qa/README.md)
- [安全测试用例](../security/README.md)
- [Scripts README](../../scripts/README.md)

---

## 11. TODO

- [ ] 实现 Rust seed-data 二进制
- [ ] 实现 TypeScript seed-data 脚本
- [ ] 生成 SQL 脚本（基于 YAML）
- [ ] 完善 qa-complex.yaml 配置
- [ ] 添加数据验证脚本
- [ ] 集成到 CI/CD 流程
- [ ] 实现 Keycloak 用户自动清理
