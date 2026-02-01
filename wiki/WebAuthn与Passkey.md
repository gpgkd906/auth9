# WebAuthn 与 Passkey

Auth9 支持 WebAuthn/FIDO2 标准的无密码认证，通过 Keycloak 原生集成提供 Passkey 功能。

## 核心概念

### 什么是 Passkey

Passkey 是基于 WebAuthn 标准的无密码认证凭据，具有以下特点：

- **无密码**：使用生物识别（指纹、面容）或设备 PIN 认证
- **抗钓鱼**：私钥绑定到特定域名，无法被钓鱼网站使用
- **跨设备**：支持云同步，可在多个设备间使用
- **安全**：私钥永不离开设备

### Passkey 类型

| 类型 | 描述 | 使用场景 |
|------|------|---------|
| **Passwordless** | 完全替代密码 | 日常登录 |
| **Two-Factor** | 作为第二因素 | 高安全场景 |

### 支持的认证器

- **平台认证器**：设备内置（Touch ID, Face ID, Windows Hello）
- **漫游认证器**：外部设备（YubiKey, Security Key）

## Keycloak 集成

### 架构说明

```
用户 → Auth9 Portal → Keycloak → WebAuthn API → 认证器
                         ↓
              Passkey 凭据存储
```

Auth9 不直接处理 WebAuthn 注册/认证流程，而是通过 Keycloak 的原生 WebAuthn 支持实现。

### Keycloak 配置

在 Keycloak Realm 设置中启用 WebAuthn：

1. 登录 Keycloak Admin Console
2. 选择目标 Realm
3. 进入 Authentication > Required Actions
4. 启用 "WebAuthn Register" 和 "WebAuthn Passwordless Register"

### 认证策略配置

```json
// Keycloak WebAuthn Policy
{
  "rpEntityName": "Auth9",
  "signatureAlgorithms": ["ES256", "RS256"],
  "rpId": "auth9.yourdomain.com",
  "attestationConveyancePreference": "none",
  "authenticatorAttachment": "cross-platform",
  "requireResidentKey": "No",
  "userVerificationRequirement": "preferred"
}
```

## 用户端操作

### 添加 Passkey

**通过管理界面**：
1. 导航到 Settings > Passkeys
2. 点击 "Add Passkey" 按钮
3. 系统跳转到 Keycloak WebAuthn 注册页面
4. 按照浏览器提示完成注册：
   - 选择认证器（Touch ID、安全密钥等）
   - 验证身份（指纹、面容、PIN）
   - 为 Passkey 命名（可选）
5. 注册成功后自动返回 Auth9

**通过 REST API**：

获取注册 URL：

```bash
curl https://api.auth9.yourdomain.com/api/v1/webauthn/register-url \
  -H "Authorization: Bearer <access_token>"
```

响应：

```json
{
  "data": {
    "url": "https://keycloak.yourdomain.com/realms/auth9/protocol/openid-connect/auth?response_type=code&client_id=auth9-portal&redirect_uri=...&kc_action=CONFIGURE_TOTP_OR_WEBAUTHN"
  }
}
```

### 查看已注册的 Passkey

**通过管理界面**：
1. 导航到 Settings > Passkeys
2. 查看已注册的 Passkey 列表

**通过 REST API**：

```bash
curl https://api.auth9.yourdomain.com/api/v1/webauthn/credentials \
  -H "Authorization: Bearer <access_token>"
```

响应：

```json
{
  "data": [
    {
      "id": "credential-id-1",
      "label": "MacBook Pro Touch ID",
      "type": "passwordless",
      "aaguid": "de1e552d-db1d-4423-a619-566b625cdc84",
      "created_at": "2024-01-01T10:00:00Z"
    },
    {
      "id": "credential-id-2",
      "label": "YubiKey 5",
      "type": "two_factor",
      "aaguid": "cb69481e-8ff7-4039-93ec-0a2729a154a8",
      "created_at": "2024-01-15T14:30:00Z"
    }
  ]
}
```

### 删除 Passkey

**通过管理界面**：
1. 在 Passkey 列表中找到要删除的凭据
2. 点击 "Delete" 按钮
3. 确认删除

**通过 REST API**：

```bash
curl -X DELETE https://api.auth9.yourdomain.com/api/v1/webauthn/credentials/{credential_id} \
  -H "Authorization: Bearer <access_token>"
```

响应：

```json
{
  "message": "Passkey deleted successfully."
}
```

## Keycloak Admin API

### 获取用户 WebAuthn 凭据

```bash
curl https://keycloak.yourdomain.com/admin/realms/auth9/users/{user_id}/credentials \
  -H "Authorization: Bearer <admin_token>"
```

响应：

```json
[
  {
    "id": "credential-uuid",
    "type": "webauthn",
    "userLabel": "MacBook Pro Touch ID",
    "createdDate": 1704067200000,
    "credentialData": "{\"aaguid\":\"de1e552d-db1d-4423-a619-566b625cdc84\",...}"
  }
]
```

### 删除用户凭据

```bash
curl -X DELETE \
  https://keycloak.yourdomain.com/admin/realms/auth9/users/{user_id}/credentials/{credential_id} \
  -H "Authorization: Bearer <admin_token>"
```

## 认证流程

### Passkey 登录流程

```
1. 用户访问登录页
2. 点击 "Sign in with Passkey"
3. 重定向到 Keycloak
4. Keycloak 发起 WebAuthn 认证请求
5. 浏览器调用认证器
6. 用户验证身份（指纹/面容/PIN）
7. 认证器返回签名
8. Keycloak 验证签名
9. 认证成功，重定向回 Auth9
```

### 浏览器 API 调用

```javascript
// 认证请求（由 Keycloak 自动处理）
const credential = await navigator.credentials.get({
  publicKey: {
    challenge: new Uint8Array([...]),
    rpId: "auth9.yourdomain.com",
    userVerification: "preferred",
    timeout: 60000,
  }
});
```

## 前端集成

### React 组件示例

```tsx
import { useState, useEffect } from 'react';
import { webauthnApi } from '~/services/api';

function PasskeySettings() {
  const [passkeys, setPasskeys] = useState([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadPasskeys();
  }, []);

  const loadPasskeys = async () => {
    const response = await webauthnApi.listPasskeys();
    setPasskeys(response.data);
    setLoading(false);
  };

  const handleAddPasskey = async () => {
    const { data } = await webauthnApi.getRegisterUrl();
    // 跳转到 Keycloak 注册页面
    window.location.href = data.url;
  };

  const handleDeletePasskey = async (credentialId: string) => {
    await webauthnApi.deletePasskey(credentialId);
    loadPasskeys();
  };

  return (
    <div>
      <button onClick={handleAddPasskey}>Add Passkey</button>

      {passkeys.map(passkey => (
        <div key={passkey.id}>
          <span>{passkey.label}</span>
          <span>{passkey.type}</span>
          <button onClick={() => handleDeletePasskey(passkey.id)}>
            Delete
          </button>
        </div>
      ))}
    </div>
  );
}
```

## AAGUID 参考

常见认证器的 AAGUID：

| 认证器 | AAGUID |
|--------|--------|
| Apple Touch ID | de1e552d-db1d-4423-a619-566b625cdc84 |
| Apple Face ID | de1e552d-db1d-4423-a619-566b625cdc84 |
| Windows Hello | 08987058-cadc-4b81-b6e1-30de50dcbe96 |
| YubiKey 5 | cb69481e-8ff7-4039-93ec-0a2729a154a8 |
| Google Titan | 42b4fb4a-2866-43b2-bc6c-5c2e02d5a5e5 |
| 1Password | d548826e-79b4-db40-a3d8-11116f7e8349 |

## 安全考虑

### 优势

- ✅ **抗钓鱼**：凭据绑定到特定域名
- ✅ **无密码泄露风险**：私钥永不传输
- ✅ **防重放攻击**：每次认证使用唯一挑战
- ✅ **用户验证**：强制生物识别或 PIN

### 限制

- ⚠️ 需要现代浏览器支持
- ⚠️ 部分设备可能不支持
- ⚠️ 丢失所有认证器会导致无法登录

### 恢复机制

建议用户：
1. 注册多个 Passkey（不同设备）
2. 保留密码作为备用
3. 配置账户恢复选项

## 浏览器兼容性

| 浏览器 | 版本 | 支持情况 |
|--------|------|---------|
| Chrome | 67+ | ✅ 完全支持 |
| Firefox | 60+ | ✅ 完全支持 |
| Safari | 14+ | ✅ 完全支持 |
| Edge | 79+ | ✅ 完全支持 |

### 检测浏览器支持

```javascript
const isWebAuthnSupported = () => {
  return window.PublicKeyCredential !== undefined;
};

const isPlatformAuthenticatorAvailable = async () => {
  if (!isWebAuthnSupported()) return false;
  return await PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable();
};
```

## 故障排查

### 注册失败

| 错误 | 原因 | 解决方案 |
|------|------|---------|
| NotAllowedError | 用户取消或超时 | 重新尝试注册 |
| NotSupportedError | 浏览器不支持 | 更新浏览器 |
| SecurityError | 非 HTTPS 环境 | 使用 HTTPS |
| InvalidStateError | 凭据已存在 | 删除旧凭据后重试 |

### 认证失败

| 错误 | 原因 | 解决方案 |
|------|------|---------|
| NotAllowedError | 用户取消或超时 | 重新尝试 |
| NotFoundError | 找不到凭据 | 检查是否已注册 |
| SecurityError | 域名不匹配 | 检查 RP ID 配置 |

## 审计日志

WebAuthn 相关操作会记录审计日志：

| 事件类型 | 描述 |
|---------|------|
| `webauthn.register_started` | 开始注册 Passkey |
| `webauthn.register_completed` | 完成注册 Passkey |
| `webauthn.register_failed` | 注册失败 |
| `webauthn.authenticate_success` | Passkey 认证成功 |
| `webauthn.authenticate_failed` | Passkey 认证失败 |
| `webauthn.credential_deleted` | 删除 Passkey |

## 最佳实践

### 用户引导

1. 提供清晰的 Passkey 介绍
2. 说明注册步骤和设备要求
3. 建议注册多个 Passkey
4. 提供备用登录方式

### 安全建议

1. 强制使用用户验证（userVerification: required）
2. 定期审计已注册的凭据
3. 监控异常认证尝试
4. 为高权限账户强制使用 Passkey

### 管理建议

1. 配置合理的认证超时
2. 定期清理未使用的凭据
3. 监控认证成功率
4. 提供凭据恢复流程

## 常见问题

### Q: Passkey 和安全密钥有什么区别？

A: Passkey 是一个更广泛的概念，包括：
- 平台认证器（内置在设备中）
- 漫游认证器（安全密钥等外部设备）

安全密钥是漫游认证器的一种。

### Q: 丢失设备后如何恢复访问？

A: 建议的恢复方式：
1. 使用其他已注册的 Passkey
2. 使用密码登录（如果保留）
3. 联系管理员重置凭据
4. 使用账户恢复流程

### Q: 为什么无法在某些网站使用同一个 Passkey？

A: Passkey 绑定到特定域名（RP ID），无法跨站点使用。这是安全设计，防止钓鱼攻击。

### Q: Passkey 支持共享吗？

A: 不支持直接共享。但某些密码管理器支持在家庭组内共享 Passkey。

## 相关文档

- [认证流程](认证流程.md)
- [密码管理](密码管理.md)
- [会话管理](会话管理.md)
- [Keycloak 主题定制](Keycloak主题定制.md)
