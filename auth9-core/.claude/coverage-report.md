# Auth9-Core 测试覆盖率健康度评估报告

**生成时间**: 2026-01-30
**总体覆盖率**: 18.35% (477/2599 lines)
**最低要求**: 90%

---

## 📊 执行摘要

**健康度评级**: 🔴 **不及格 (CRITICAL)**

当前测试覆盖率为 **18.35%**，远低于项目规则要求的 **90%** 最低标准。只有 Service 层勉强达标 (90.88%)，其他所有层都严重不足。

---

## 📈 分层覆盖率分析

### 1. Service Layer (服务层) ✅

**实际覆盖率**: 90.88% (319/351 lines)
**目标覆盖率**: 90%+
**状态**: **达标**

| 文件 | 覆盖率 | 覆盖行数 | 评价 |
|------|--------|----------|------|
| `service/user.rs` | 100% | 50/50 | ✅ 优秀 |
| `service/client.rs` | 92.56% | 137/148 | ✅ 良好 |
| `service/rbac.rs` | 86.45% | 83/96 | ⚠️ 接近达标 |
| `service/tenant.rs` | 85.96% | 49/57 | ⚠️ 接近达标 |

**评价**: Service 层是唯一达标的层级，说明业务逻辑的单元测试相对完善。但 rbac 和 tenant 服务需要补充边缘情况测试。

---

### 2. Domain Layer (领域层) ❌

**实际覆盖率**: 64.81% (70/108 lines)
**目标覆盖率**: 95%+
**状态**: **严重不足** (-30.19%)

| 文件 | 覆盖率 | 覆盖行数 | 评价 |
|------|--------|----------|------|
| `domain/user.rs` | 100% | 5/5 | ✅ 优秀 |
| `domain/rbac.rs` | 100% | 14/14 | ✅ 优秀 |
| `domain/tenant.rs` | 68.29% | 28/41 | ❌ 不足 |
| `domain/common.rs` | 61.53% | 16/26 | ❌ 不足 |
| `domain/service.rs` | 31.81% | 7/22 | ❌ 严重不足 |

**评价**: domain/service.rs 严重缺失测试。domain/tenant.rs 和 domain/common.rs 需要补充验证逻辑测试。

---

### 3. Repository Layer (数据访问层) ❌

**实际覆盖率**: 0% (0/139 lines)
**目标覆盖率**: 85%+
**状态**: **完全缺失** (-85%)

| 文件 | 覆盖率 | 覆盖行数 | 评价 |
|------|--------|----------|------|
| `repository/audit.rs` | 0% | 0/42 | ❌ 无覆盖 |
| `repository/rbac.rs` | 0% | 0/41 | ❌ 无覆盖 |
| `repository/user.rs` | 0% | 0/17 | ❌ 无覆盖 |
| `repository/tenant.rs` | 0% | 0/13 | ❌ 无覆盖 |
| `repository/service.rs` | 0% | 0/21 | ❌ 无覆盖 |
| `repository/mod.rs` | 0% | 0/5 | ❌ 无覆盖 |

**评价**: ⚠️ **关键发现** - 尽管存在集成测试文件（user_test.rs, audit_test.rs等），但 tarpaulin 没有捕获到 repository 层的代码执行，可能是：
1. 测试使用了 mock 对象而非实际实现
2. Repository 实现在独立模块中未被跟踪
3. 集成测试未启用覆盖率instrumentation

**建议**: 检查集成测试是否真正调用了 RepositoryImpl，而不是 Mock。

---

### 4. API Layer (HTTP/gRPC 层) ❌

**实际覆盖率**: 3.48% (36/1034 lines)
**目标覆盖率**: 80%+
**状态**: **几乎为零** (-76.52%)

| 文件 | 覆盖率 | 覆盖行数 | 评价 |
|------|--------|----------|------|
| `api/mod.rs` | 48.88% | 22/45 | ❌ 不足 |
| `api/audit.rs` | 40% | 4/10 | ❌ 不足 |
| `api/service.rs` | 2.28% | 4/175 | ❌ 几乎无覆盖 |
| `api/auth.rs` | 2.04% | 6/294 | ❌ 几乎无覆盖 |
| `api/tenant.rs` | 0% | 0/44 | ❌ 无覆盖 |
| `api/user.rs` | 0% | 0/146 | ❌ 无覆盖 |
| `api/role.rs` | 0% | 0/133 | ❌ 无覆盖 |
| `api/health.rs` | 0% | 0/12 | ❌ 无覆盖 |
| `grpc/token_exchange.rs` | 0% | 0/175 | ❌ 无覆盖 |
| `server/mod.rs` | 0% | 0/106 | ❌ 无覆盖 |

**评价**: API 层几乎完全缺失测试。health_api_test.rs 存在但失败。需要大量补充 API 集成测试。

---

### 5. 其他关键组件 ❌

| 组件 | 覆盖率 | 覆盖行数 | 状态 |
|------|--------|----------|------|
| `jwt/mod.rs` | 67.64% | 46/68 | ⚠️ 接近达标 |
| `keycloak/mod.rs` | 0% | 0/486 | ❌ 无覆盖 |
| `cache/mod.rs` | 0% | 0/92 | ❌ 无覆盖 |
| `migration/mod.rs` | 0% | 0/90 | ❌ 无覆盖 |
| `config/mod.rs` | 7.4% | 4/54 | ❌ 几乎无覆盖 |
| `error/mod.rs` | 4.87% | 2/41 | ❌ 几乎无覆盖 |

---

## 🔍 关键问题

### 1. Repository 层 0% 覆盖率异常 🚨

**现象**:
- 集成测试存在：`user_test.rs` (5个), `audit_test.rs` (5个), `rbac_test.rs` (11个), `tenant_test.rs` (7个), `service_test.rs` (6个)
- 所有测试都通过
- 但 repository/*.rs 文件显示 0% 覆盖率

**可能原因**:
1. 测试使用了 `MockXxxRepository` 而非 `XxxRepositoryImpl`
2. Repository 实现代码路径不在 tarpaulin 跟踪范围
3. Trait 方法没有默认实现，只有接口定义

**验证方法**:
```bash
# 检查测试是否使用了 Mock
grep -r "Mock.*Repository" tests/

# 检查是否使用了实际实现
grep -r "RepositoryImpl::new" tests/
```

### 2. API 层几乎无覆盖 🚨

**现象**:
- API handler 代码存在 (1034 lines)
- 但几乎没有 HTTP 集成测试运行
- health_api_test 失败（数据库连接问题）

**影响**:
- 无法验证 HTTP 路由正确性
- 无法验证请求/响应序列化
- 无法验证错误处理逻辑

### 3. Keycloak 客户端完全未测试 🚨

**现象**:
- keycloak/mod.rs: 486 lines, 0% 覆盖
- 这是关键的外部依赖集成

**风险**:
- Keycloak API 调用失败无法及时发现
- OIDC 流程问题可能到生产才暴露

---

## 📋 改进建议（优先级排序）

### 🔥 P0 - 紧急（必须立即修复）

#### 1. 修复 Repository 层覆盖率收集
```bash
# 验证测试是否调用实际实现
# tests/user_test.rs 应该包含:
use auth9_core::repository::user::UserRepositoryImpl;

let repo = UserRepositoryImpl::new(pool.clone());
let result = repo.create(&input).await?;
```

**目标**: 将 Repository 层从 0% 提升到 85%+

#### 2. 补充 API 集成测试
- 创建 API 测试脚手架（参考 health_api_test.rs）
- 优先测试核心接口：
  - `POST /api/v1/tenants` (创建租户)
  - `POST /api/v1/users` (创建用户)
  - `POST /api/v1/auth/token` (令牌交换)
  - `GET /api/v1/health` (健康检查)

**目标**: 将 API 层从 3.48% 提升到 60%+

#### 3. 补充 Domain 层测试
- domain/service.rs: 补充验证逻辑测试
- domain/tenant.rs: 补充 slug 验证、设置验证
- domain/common.rs: 补充 StringUuid 边缘情况

**目标**: 将 Domain 层从 64.81% 提升到 95%+

---

### ⚠️ P1 - 高优先级（1周内完成）

#### 4. 添加 Keycloak 集成测试
- 使用 wiremock 模拟 Keycloak API
- 测试关键流程：
  - 用户创建
  - Realm 配置
  - OIDC 客户端管理

**目标**: Keycloak 模块从 0% 提升到 70%+

#### 5. 添加 Cache 层测试
- Redis 操作测试（使用 testcontainers）
- 缓存失效测试
- 缓存一致性测试

**目标**: Cache 模块从 0% 提升到 80%+

#### 6. 完善 JWT 测试
- 当前 67.64%，接近达标
- 补充：令牌过期、签名验证失败、audience 不匹配等边缘情况

**目标**: JWT 模块从 67.64% 提升到 90%+

---

### 📌 P2 - 中优先级（2周内完成）

#### 7. 添加 gRPC 服务测试
- `grpc/token_exchange.rs`: 0/175 lines
- 使用 tonic test client 测试 gRPC 接口

**目标**: gRPC 层从 0% 提升到 75%+

#### 8. 添加 Migration 测试
- 测试数据库迁移的幂等性
- 测试回滚功能

**目标**: Migration 模块从 0% 提升到 60%+

#### 9. 补充 Config/Error 测试
- config/mod.rs: 7.4% → 70%+
- error/mod.rs: 4.87% → 80%+

---

## 📊 改进路线图

### 第1周（P0任务）
- [ ] 修复 Repository 层覆盖率收集问题
- [ ] 补充 API 核心接口测试（tenant, user, auth）
- [ ] 补充 Domain 层测试（service, tenant, common）

**预期结果**:
- Repository 层: 0% → 85%
- API 层: 3.48% → 60%
- Domain 层: 64.81% → 95%
- **总体覆盖率**: 18.35% → **~65%**

### 第2周（P1任务）
- [ ] 添加 Keycloak 集成测试（wiremock）
- [ ] 添加 Cache 层测试（testcontainers Redis）
- [ ] 完善 JWT 边缘情况测试

**预期结果**:
- Keycloak: 0% → 70%
- Cache: 0% → 80%
- JWT: 67.64% → 90%
- **总体覆盖率**: ~65% → **~82%**

### 第3周（P2任务）
- [ ] 添加 gRPC 服务测试
- [ ] 添加 Migration 测试
- [ ] 补充 Config/Error 测试

**预期结果**:
- gRPC: 0% → 75%
- Migration: 0% → 60%
- Config/Error: <10% → 75%
- **总体覆盖率**: ~82% → **~90%** ✅

---

## 🎯 质量门禁建议

### CI/CD 集成
```toml
# tarpaulin.toml
[report]
fail-under = 65  # 第1周后
# fail-under = 82  # 第2周后
# fail-under = 90  # 第3周后（最终目标）
```

### Git Hook
```bash
# .git/hooks/pre-push
#!/bin/bash
echo "Running coverage check..."
cargo tarpaulin --ignore-config --fail-under 65
```

---

## 📄 附录：测试文件清单

### 已存在的测试文件
```
tests/
├── common/mod.rs          # 测试工具
├── user_test.rs           # 5个测试 ✅
├── audit_test.rs          # 5个测试 ✅
├── rbac_test.rs           # 11个测试 ✅
├── tenant_test.rs         # 7个测试 ✅
├── service_test.rs        # 6个测试 ✅
├── health_api_test.rs     # 2个测试 ❌ (失败)
├── tenant_api_test.rs     # (存在但未运行)
├── role_api_test.rs       # (存在但未运行)
└── keycloak_*.rs          # (存在但被忽略)
```

### 库内单元测试
```rust
// 301个测试通过 ✅
// 分布在各模块的 #[cfg(test)] mod tests
```

---

## 🏁 总结

**当前状态**: 18.35% 覆盖率，**不及格**

**核心问题**:
1. Repository 层 0% 覆盖率异常（测试存在但未被统计）
2. API 层几乎无覆盖（缺失 HTTP 集成测试）
3. Keycloak/Cache/gRPC 等关键组件完全未测试

**改进目标**: 3周内从 18.35% → 90%

**建议行动**:
1. **立即**: 检查 repository 层覆盖率收集问题
2. **本周**: 补充 API 集成测试和 Domain 测试
3. **后续**: 按优先级逐步覆盖 Keycloak、Cache、gRPC 等模块

---

**报告生成**: `cargo tarpaulin --workspace --lib --test user_test --test audit_test --test rbac_test --test tenant_test --test service_test`
