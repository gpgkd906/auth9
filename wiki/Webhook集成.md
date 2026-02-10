# Webhook 集成指南

Auth9 的 Webhook 系统允许您的应用程序实时订阅身份平台中发生的事件。当特定事件（如用户注册、登录成功、安全告警）发生时，Auth9 会向您配置的 HTTPS 端点发送 POST 请求。

## 1. 支持的事件类型

您可以在创建或更新 Webhook 时订阅以下一种或多种事件：

| 事件类型 (`event_type`) | 说明 | 触发时机 |
| :--- | :--- | :--- |
| `login.success` | 登录成功 | 用户成功通过身份验证并获得令牌时 |
| `login.failed` | 登录失败 | 用户密码错误、被锁定或 MFA 验证失败时 |
| `user.created` | 用户创建 | 新用户注册或通过 API 创建时 |
| `user.updated` | 用户更新 | 用户资料（Profile）被修改时 |
| `user.deleted` | 用户删除 | 用户被从系统中删除时 |
| `password.changed` | 密码修改 | 用户重置或修改密码成功时 |
| `mfa.enabled` | MFA 启用 | 用户绑定了新的 MFA 设备（如 OTP 或 Passkey） |
| `mfa.disabled` | MFA 禁用 | 用户移除了 MFA 设备 |
| `session.revoked` | 会话撤销 | 用户登出或管理员强制下线时 |
| `security.alert` | 安全告警 | 系统检测到异常行为（如异地登录、暴力破解）时 |

## 2. 请求格式

Auth9 发送的 Webhook 请求是一个标准的 HTTP `POST` 请求。

### Header

```http
Content-Type: application/json
User-Agent: Auth9-Webhook/1.0
X-Webhook-Event: login.success
X-Webhook-Timestamp: 2023-10-27T10:00:00Z
X-Webhook-Signature: sha256=...
```

- **X-Webhook-Event**: 当前触发的事件类型。
- **X-Webhook-Timestamp**: 事件发生的时间（ISO 8601 格式）。
- **X-Webhook-Signature**: 用于验证请求来源的签名（见下文）。

### Body (Payload)

Payload 是一个 JSON 对象，包含事件的详细信息。

```json
{
  "event_type": "login.success",
  "timestamp": "2023-10-27T10:00:00Z",
  "data": {
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "email": "user@example.com",
    "ip_address": "192.168.1.1",
    "user_agent": "Mozilla/5.0..."
  }
}
```

`data` 字段的内容会根据 `event_type` 的不同而变化。

## 3. 安全验证 (Signature Verification)

为了确保接收到的 Webhook 请求确实来自 Auth9，而非恶意伪造，您**必须**验证请求签名。

Auth9 使用 HMAC-SHA256 算法计算签名。签名包含在 `X-Webhook-Signature` 请求头中，格式为 `sha256=<hex_signature>`。

### 验证步骤

1. 获取 Webhook 的 **Secret**（在 Auth9 控制台创建 Webhook 时生成）。
2. 获取请求的原始 Body（Raw Body，不要解析为 JSON 对象，以免字段顺序变化导致签名不匹配）。
3. 使用 Secret 作为密钥，对 Raw Body 进行 HMAC-SHA256 计算。
4. 将计算出的 Hex 字符串与 `X-Webhook-Signature` 头部中的值（去掉 `sha256=` 前缀）进行比对。建议使用恒定时间比较函数（Constant-time comparison）以防止时序攻击。

### 代码示例

#### Node.js (Express)

```javascript
const crypto = require('crypto');
const express = require('express');
const app = express();

// 必须获取 raw body
app.use(express.raw({ type: 'application/json' }));

const WEBHOOK_SECRET = 'whsec_...'; // 您的 Webhook Secret

app.post('/webhook', (req, res) => {
  const signatureHeader = req.headers['x-webhook-signature'];
  
  if (!signatureHeader) {
    return res.status(400).send('Missing signature');
  }

  const [algo, signature] = signatureHeader.split('=');
  
  if (algo !== 'sha256') {
    return res.status(400).send('Unsupported algorithm');
  }

  const expectedSignature = crypto
    .createHmac('sha256', WEBHOOK_SECRET)
    .update(req.body)
    .digest('hex');

  // 使用 timingSafeEqual 防止时序攻击
  const valid = crypto.timingSafeEqual(
    Buffer.from(signature),
    Buffer.from(expectedSignature)
  );

  if (!valid) {
    return res.status(401).send('Invalid signature');
  }

  // 签名验证通过，处理业务逻辑
  const event = JSON.parse(req.body.toString());
  console.log('Received event:', event.event_type);

  res.status(200).send('OK');
});
```

#### Python (FastAPI)

```python
import hmac
import hashlib
from fastapi import FastAPI, Request, HTTPException

app = FastAPI()
WEBHOOK_SECRET = "whsec_..."

@app.post("/webhook")
async def handle_webhook(request: Request):
    signature_header = request.headers.get("X-Webhook-Signature")
    if not signature_header:
        raise HTTPException(status_code=400, detail="Missing signature")
    
    # 读取原始 body bytes
    body = await request.body()
    
    # 计算签名
    expected_signature = hmac.new(
        WEBHOOK_SECRET.encode(),
        body,
        hashlib.sha256
    ).hexdigest()
    
    received_signature = signature_header.split("=")[1]
    
    # 验证签名
    if not hmac.compare_digest(received_signature, expected_signature):
        raise HTTPException(status_code=401, detail="Invalid signature")
    
    event = await request.json()
    print(f"Received event: {event['event_type']}")
    
    return {"status": "ok"}
```

#### Rust (Axum)

```rust
use axum::{
    body::Bytes,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Router,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

const WEBHOOK_SECRET: &str = "whsec_...";

async fn webhook_handler(headers: HeaderMap, body: Bytes) -> impl IntoResponse {
    let signature_header = headers
        .get("X-Webhook-Signature")
        .and_then(|v| v.to_str().ok());

    let Some(sig_str) = signature_header else {
        return StatusCode::BAD_REQUEST;
    };

    let signature_hex = sig_str.strip_prefix("sha256=").unwrap_or("");

    let mut mac = HmacSha256::new_from_slice(WEBHOOK_SECRET.as_bytes())
        .expect("HMAC can take any size key");
    mac.update(&body);

    if mac.verify_slice(&hex::decode(signature_hex).unwrap_or_default()).is_err() {
        return StatusCode::UNAUTHORIZED;
    }

    // 签名验证通过
    println!("Webhook verified!");
    StatusCode::OK
}
```

## 4. 重试策略

如果您的服务器未能成功响应（返回非 2xx 状态码或超时），Auth9 将会尝试重新发送 Webhook。

- **重试机制**: 指数退避 (Exponential Backoff)。
- **最大重试次数**: 3 次。
- **重试间隔**: 第一次重试间隔 1 秒，第二次 2 秒，第三次 4 秒。

### 自动禁用

为了防止向失效的端点无限发送请求，Auth9 实施了断路器机制：

- 如果一个 Webhook 连续失败次数达到 **10 次**，系统将自动**禁用**该 Webhook。
- 管理员需要在修复接收端问题后，在 Auth9 控制台手动重新启用该 Webhook。

## 5. 最佳实践

1.  **快速响应**: Webhook 处理器应该尽可能快地返回 `200 OK`。如果需要执行耗时操作（如发送邮件、生成报表），请将任务放入您内部的队列中异步处理，而不是在 Webhook 请求中同步等待。
2.  **幂等性处理**: 尽管 Auth9 尽量保证每个事件只发送一次，但网络波动可能导致您收到重复的 Webhook。请使用事件中的 `timestamp` 或内容中的 ID 来实现幂等处理。
3.  **使用 HTTPS**: 生产环境中务必使用 HTTPS URL，以防止 Secret 和数据在传输过程中被窃听。
