# role_api_test 修复总结

**日期**: 2026-01-30
**状态**: ✅ 已完成

---

## 问题描述

`role_api_test.rs` 中的两个测试全部失败：
- `test_role_crud_flow` - 测试 Role 的完整 CRUD 流程
- `test_list_roles_by_service` - 测试按 Service 列出 Roles

**错误信息**:
```
assertion failed: service_res.status().is_success()
Status: 502 Bad Gateway
Body: {"error":"keycloak_error","message":"Authentication service error"}
```

---

## 根本原因

Service API 的 `create` 方法需要调用 Keycloak Admin API 来创建 OIDC 客户端：

```rust
// src/api/service.rs:97-100
let client_uuid = state
    .keycloak_client
    .create_oidc_client(&keycloak_client)
    .await?;
```

但测试中没有设置相应的 Keycloak mock，导致：
1. **Keycloak Admin Token 获取失败** - 没有 mock `/realms/master/protocol/openid-connect/token`
2. **OIDC 客户端创建失败** - 没有 mock `/admin/realms/test/clients`
3. **客户端密钥获取失败** - 没有 mock `/admin/realms/.../clients/.../client-secret`

---

## 修复方案

在测试中添加完整的 Keycloak mock 设置：

### 1. Mock Keycloak Admin Token

```rust
Mock::given(method("POST"))
    .and(path_regex("/realms/master/protocol/openid-connect/token.*"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
        "access_token": "mock-admin-token",
        "expires_in": 36000,  // Long expiry to avoid refresh
        "refresh_token": "mock-refresh-token",
        "token_type": "bearer"
    })))
    .named("keycloak_admin_token")
    .mount(&app.mock_server)
    .await;
```

**关键点**:
- 使用 `path_regex` 而非 `path` 以匹配可能的查询参数
- 设置长过期时间 (36000秒) 避免测试过程中 token 刷新
- 使用 `.named()` 便于调试

### 2. Mock 创建 OIDC 客户端

```rust
Mock::given(method("POST"))
    .and(path_regex("/admin/realms/.*/clients"))
    .respond_with(ResponseTemplate::new(201).insert_header(
        "Location",
        format!("{}/admin/realms/test/clients/{}", app.mock_server.uri(), mock_client_uuid)
    ))
    .named("create_oidc_client")
    .mount(&app.mock_server)
    .await;
```

**关键点**:
- 返回 201 Created 状态码
- 在 Location header 中返回客户端 UUID
- 使用 `path_regex` 匹配任意 realm

### 3. Mock 获取客户端密钥

```rust
Mock::given(method("GET"))
    .and(path_regex("/admin/realms/.*/clients/.*/client-secret"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
        "type": "secret",
        "value": "mock-client-secret"
    })))
    .named("get_client_secret")
    .mount(&app.mock_server)
    .await;
```

**关键点**:
- Keycloak 返回的格式是 `{"type": "secret", "value": "..."}`
- 使用正则表达式匹配路径中的动态部分

---

## 修复文件

**文件**: `tests/role_api_test.rs`

**修改内容**:
1. 添加 `wiremock::matchers::path_regex` 导入
2. 在两个测试中都添加完整的 Keycloak mock 设置
3. 清理调试代码

**代码变更量**:
- 添加行数: ~40 lines
- 删除行数: 0 lines
- 修改测试数: 2

---

## 测试结果

### 修复前
```
running 2 tests
test test_list_roles_by_service ... FAILED
test test_role_crud_flow ... FAILED

test result: FAILED. 0 passed; 2 failed
```

### 修复后
```
running 2 tests
test test_list_roles_by_service ... ok
test test_role_crud_flow ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.45s
```

✅ **100% 通过率**

---

## 经验教训

### 1. Keycloak Mock 是必需的

任何涉及 Service/Client 操作的测试都需要完整的 Keycloak mock：
- Admin token 获取
- OIDC 客户端 CRUD
- 客户端密钥管理

### 2. 使用 path_regex 而非精确路径

Keycloak API 路径包含动态部分（realm名称、客户端ID等），使用正则表达式更灵活：
```rust
.and(path_regex("/admin/realms/.*/clients"))  // ✅ 推荐
.and(path("/admin/realms/test/clients"))      // ❌ 过于严格
```

### 3. Mock 响应格式必须精确

Keycloak 的响应格式需要精确匹配，否则会导致解析失败：
```rust
// ✅ 正确的客户端密钥响应
{
    "type": "secret",
    "value": "mock-client-secret"
}

// ❌ 错误的格式
{
    "id": "...",
    "secret": "mock-client-secret"
}
```

### 4. 延长 Token 过期时间

测试可能运行较长时间（特别是创建多个资源时），设置较长的 token 过期时间避免中途刷新：
```rust
"expires_in": 36000  // 10小时，足够任何测试使用
```

---

## 影响

### 覆盖率提升
- **修复前**: role_api_test 0/2 通过
- **修复后**: role_api_test 2/2 通过 ✅

### 解锁的测试场景
1. Role CRUD 操作（创建、读取、更新、删除）
2. Permission 管理
3. Role-Permission 关联
4. Service 下的 Roles 列表
5. 跨 Service 的 RBAC 测试

---

## 后续任务

1. ✅ 修复 role_api_test - **已完成**
2. ⏭️ 补充其他 API 测试（user, service, audit, auth）
3. ⏭️ 生成详细覆盖率报告

---

## 参考

- **测试文件**: `tests/role_api_test.rs`
- **相关 API**: `src/api/service.rs`, `src/api/role.rs`
- **Keycloak 客户端**: `src/keycloak/mod.rs`
- **类似测试**: `tests/user_api_test.rs` (也使用 Keycloak mock)

---

**修复完成时间**: 2026-01-30 19:30
**总耗时**: 约 1.5 小时
**状态**: ✅ 成功修复
