# Auth9 Scripts

本目录包含 Auth9 项目的各种脚本和工具。

## 目录结构

```
scripts/
├── seed-data/              # 测试数据种子
│   ├── qa-basic.yaml       # 基础 QA 测试数据
│   ├── qa-complex.yaml     # 复杂 QA 测试数据 (待实现)
│   └── security-vulnerable.yaml  # 安全测试数据（包含已知弱配置）
├── reset-test-env.sh       # 重置测试环境脚本
└── README.md               # 本文件
```

## 脚本说明

### reset-test-env.sh

重置 Auth9 测试环境，清理所有测试数据并可选择加载种子数据。

**用法：**

```bash
# 交互式重置环境
./scripts/reset-test-env.sh

# 脚本会执行以下步骤：
# 1. 清理数据库中的测试数据（qa-*, sec-* 前缀）
# 2. 提示清理 Keycloak 测试用户（手动）
# 3. 清理 Redis 缓存
# 4. 可选：加载种子数据（qa-basic, qa-complex, security-vulnerable）
```

**注意事项：**
- 需要数据库服务运行（TiDB on port 4000）
- 测试数据使用特定前缀标识（`qa-*`, `sec-*`）
- 生产数据不会被影响

## 测试数据种子

### qa-basic.yaml

基础 QA 测试数据，包含：
- 3 个租户（2 个活跃，1 个暂停）
- 6 个用户（包含跨租户用户）
- 2 个服务，3 个客户端
- RBAC 配置（权限、角色、角色继承）
- Webhook、邀请等

**用途：**
- 日常 QA 手动测试
- E2E 自动化测试基础数据
- 快速演示和功能验证

**测试账户：**
```
admin@qa-acme-corp.local / QaAcmeAdmin123!   (租户管理员)
user1@qa-acme-corp.local / QaUser123!        (普通用户)
multi@qa-test.local / QaMulti123!            (跨租户用户)
```

### qa-complex.yaml

复杂 QA 测试数据（待实现），包含：
- 50+ 租户
- 1000+ 用户
- 深层 RBAC 层级（5 层角色继承）
- 多身份提供商配置
- 大量历史数据

**用途：**
- 性能测试
- 复杂场景测试（深层权限继承、大量用户）

### security-vulnerable.yaml

安全测试数据，**包含故意设置的安全漏洞配置**，用于渗透测试。

**⚠️ 警告：请勿在生产环境使用**

包含：
- 弱密码策略租户
- SQL/XSS 注入测试用户
- 配置错误的客户端（redirect_uri 通配符）
- SSRF 测试 Webhook
- 循环角色继承
- 明文密码配置

**用途：**
- 安全渗透测试
- 漏洞验证
- 安全培训

**测试账户：**
```
sqli-test@security.local / SecTest123!       (SQL 注入测试)
xss-test@security.local / SecTest123!        (XSS 测试)
weak@security.local / 1                      (弱密码测试)
```

## 数据加载方式

### 方式 1：使用 Rust seed-data 二进制（推荐，待实现）

```bash
cd auth9-core
cargo run --bin seed-data -- --dataset=qa-basic --reset
```

### 方式 2：使用 SQL 脚本（手动）

```bash
# TODO: 生成 SQL 脚本
mysql -h 127.0.0.1 -P 4000 -u root -D auth9 < scripts/seed-data/qa-basic.sql
```

### 方式 3：使用 TypeScript 脚本（待实现）

```bash
cd auth9-portal
npx ts-node scripts/seed-data.ts --config=../scripts/seed-data/qa-basic.yaml --reset
```

## 开发指南

### 添加新的种子数据

1. 创建 YAML 配置文件：
   ```bash
   cp scripts/seed-data/qa-basic.yaml scripts/seed-data/my-dataset.yaml
   ```

2. 编辑 YAML 文件，定义：
   - 租户（tenants）
   - 用户（users）
   - 服务与客户端（services, clients）
   - 权限与角色（permissions, roles）
   - 其他数据（webhooks, invitations, system_settings）

3. 实现数据加载器（Rust 或 TypeScript）

4. 更新本 README 文档

### YAML 配置格式

详细的 YAML 配置格式和数据模型，请参考 [测试数据种子设计文档](../docs/testing/seed-data-design.md)。

## 相关文档

- [QA 测试用例文档](../docs/qa/README.md)
- [安全测试用例文档](../docs/security/README.md)
- [测试数据种子设计](../docs/testing/seed-data-design.md)

## TODO

- [ ] 实现 Rust seed-data 二进制（`auth9-core/src/bin/seed-data.rs`）
- [ ] 实现 TypeScript seed-data 脚本（`auth9-portal/scripts/seed-data.ts`）
- [ ] 生成 SQL 脚本（基于 YAML 配置）
- [ ] 完善 qa-complex.yaml 配置
- [ ] 添加数据验证脚本（`validate-seed-data.sh`）
- [ ] 集成到 CI/CD 流程（自动化测试前加载数据）
- [ ] 实现 Keycloak 用户自动清理（通过 Admin API）
