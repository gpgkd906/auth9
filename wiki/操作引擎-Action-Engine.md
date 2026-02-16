# 操作引擎 (Action Engine)

## 概述

Auth9 Action Engine 是一个强大的自动化工作流系统，允许您在特定的认证事件触发时执行自定义的 JavaScript/TypeScript 代码。通过 Action Engine，您可以实现复杂的业务逻辑自动化，如用户注册后发送欢迎邮件、登录时记录审计日志、或集成第三方服务。

### 核心特性

- ✅ **事件驱动**: 在用户登录、注册、密码重置等关键事件时自动执行
- ✅ **JavaScript/TypeScript 支持**: 使用熟悉的语言编写自动化脚本
- ✅ **异步执行**: 支持 `async/await`、`fetch()` API 调用、`setTimeout` 延时
- ✅ **V8 沙箱隔离**: 安全执行用户代码，防止恶意操作
- ✅ **高性能**: 线程本地 Runtime 复用，平均执行时间 < 1ms
- ✅ **丰富的上下文**: 访问用户信息、租户数据、事件详情
- ✅ **日志和调试**: 完整的执行日志和错误追踪

## 支持的触发器 (Triggers)

Action Engine 支持以下触发器类型：

| 触发器 | 触发时机 | 常见用途 |
|--------|---------|---------|
| `login.success` | 用户登录成功后 | 记录审计日志、发送登录通知、更新最后登录时间 |
| `login.failed` | 用户登录失败后 | 安全告警、暴力破解检测、IP 黑名单 |
| `user.created` | 新用户创建后 | 发送欢迎邮件、初始化用户数据、同步到 CRM |
| `user.updated` | 用户信息更新后 | 同步到外部系统、验证数据完整性 |
| `user.deleted` | 用户删除后 | 清理关联数据、归档用户信息 |
| `password.changed` | 密码修改后 | 发送确认邮件、撤销所有会话 |
| `password.reset` | 密码重置请求 | 发送重置邮件、记录安全事件 |
| `mfa.enabled` | 启用 MFA 后 | 发送确认邮件、更新安全等级 |
| `mfa.disabled` | 禁用 MFA 后 | 发送安全告警 |
| `session.revoked` | 会话撤销后 | 通知用户、记录操作 |
| `invitation.created` | 创建邀请后 | 自定义邮件内容、通知管理员 |
| `webhook.triggered` | Webhook 触发时 | 集成第三方服务 |

## 快速开始

### 创建第一个 Action

1. 登录 Auth9 Portal
2. 选择目标租户
3. 导航到 **Actions** 页面
4. 点击 **Create Action** 按钮
5. 填写以下信息：
   - **Name**: Action 名称（如 "Send Welcome Email"）
   - **Trigger**: 选择触发器（如 `user.created`）
   - **Code**: 编写 JavaScript/TypeScript 代码
   - **Enabled**: 是否启用
6. 点击 **Save** 保存

### 示例：欢迎邮件

```javascript
// 用户注册后自动发送欢迎邮件
async function handler(context) {
  const { user, tenant } = context;
  
  // 调用邮件服务 API
  const response = await fetch('https://api.sendgrid.com/v3/mail/send', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${process.env.SENDGRID_API_KEY}`,
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({
      personalizations: [{
        to: [{ email: user.email, name: user.name }]
      }],
      from: { email: 'noreply@example.com', name: tenant.name },
      subject: `欢迎加入 ${tenant.name}！`,
      content: [{
        type: 'text/html',
        value: `
          <h1>欢迎，${user.name}！</h1>
          <p>感谢您注册 ${tenant.name}。</p>
          <p>您的账户已成功创建。</p>
        `
      }]
    })
  });
  
  if (!response.ok) {
    console.error('Failed to send email:', await response.text());
    throw new Error('Email delivery failed');
  }
  
  console.log(`Welcome email sent to ${user.email}`);
  return { success: true };
}
```

### 示例：登录安全检测

```javascript
// 检测异常登录行为
async function handler(context) {
  const { user, event } = context;
  const { ip_address, device_type, location } = event;
  
  // 检查是否为新设备
  const knownDevices = await fetch(
    `https://api.yourservice.com/users/${user.id}/devices`,
    { headers: { 'Authorization': `Bearer ${process.env.API_KEY}` } }
  ).then(r => r.json());
  
  const isNewDevice = !knownDevices.some(d => d.fingerprint === device_type);
  
  if (isNewDevice) {
    // 发送安全告警邮件
    await fetch('https://api.yourservice.com/alerts', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        user_id: user.id,
        type: 'new_device_login',
        ip: ip_address,
        location: location,
        device: device_type,
        timestamp: new Date().toISOString()
      })
    });
    
    console.log(`Security alert: New device login for ${user.email}`);
  }
  
  return { new_device: isNewDevice };
}
```

## Action Context API

每个 Action 都会接收一个 `context` 对象，包含以下信息：

### 基础字段

```typescript
interface ActionContext {
  // 触发器类型
  trigger: string;
  
  // 用户信息
  user: {
    id: string;
    email: string;
    name: string;
    first_name?: string;
    last_name?: string;
    created_at: string;
  };
  
  // 租户信息
  tenant: {
    id: string;
    name: string;
    slug: string;
    logo_url?: string;
  };
  
  // 事件详情（根据触发器类型不同）
  event?: {
    type: string;
    timestamp: string;
    ip_address?: string;
    device_type?: string;
    location?: string;
    user_agent?: string;
    // ... 其他事件特定字段
  };
  
  // 环境变量（配置的密钥）
  secrets?: Record<string, string>;
}
```

### 可用的全局 API

Action 代码运行在安全的 V8 沙箱中，可以使用以下 API：

| API | 说明 | 示例 |
|-----|------|------|
| `fetch()` | HTTP 请求 | `await fetch('https://api.example.com')` |
| `console.log()` | 日志输出 | `console.log('User:', user.email)` |
| `setTimeout()` | 延时执行 | `setTimeout(() => {}, 1000)` |
| `Promise` | 异步编程 | `await new Promise(resolve => ...)` |
| `JSON` | JSON 处理 | `JSON.stringify(data)` |
| `Math` | 数学运算 | `Math.random()` |
| `Date` | 日期时间 | `new Date().toISOString()` |

**注意**：出于安全考虑，以下 API 不可用：
- ❌ `require()` / `import` - 不支持动态模块加载
- ❌ 文件系统操作 - 无法读写文件
- ❌ 子进程 - 无法执行外部命令
- ❌ 网络监听 - 无法创建服务器

## 环境变量和密钥管理

Action 经常需要访问第三方 API 密钥。Auth9 提供安全的密钥管理机制：

### 配置密钥

1. 在 Action 编辑页面，点击 **Secrets** 标签
2. 添加环境变量：
   - **Name**: 变量名（如 `SENDGRID_API_KEY`）
   - **Value**: 密钥值
3. 点击 **Save**

### 在代码中使用

```javascript
async function handler(context) {
  // 通过 context.secrets 访问
  const apiKey = context.secrets.SENDGRID_API_KEY;
  
  const response = await fetch('https://api.sendgrid.com/v3/mail/send', {
    headers: {
      'Authorization': `Bearer ${apiKey}`
    }
  });
  
  return { success: true };
}
```

**安全提示**：
- 密钥在数据库中加密存储
- 不会出现在日志或错误消息中
- 仅在 Action 执行时可访问

## 测试和调试

### 测试 Action

在保存 Action 之前，可以先测试执行：

1. 在 Action 编辑页面，点击 **Test** 按钮
2. 提供测试上下文（模拟真实事件数据）
3. 点击 **Run Test**
4. 查看执行结果和日志输出

示例测试上下文：

```json
{
  "trigger": "login.success",
  "user": {
    "id": "user_123",
    "email": "test@example.com",
    "name": "Test User"
  },
  "tenant": {
    "id": "tenant_456",
    "name": "Acme Corp",
    "slug": "acme"
  },
  "event": {
    "type": "login",
    "timestamp": "2026-02-16T10:00:00Z",
    "ip_address": "192.168.1.1",
    "device_type": "Desktop - Chrome"
  }
}
```

### 查看执行日志

1. 导航到 **Actions** > **Logs** 页面
2. 查看所有 Action 的执行历史：
   - 执行时间
   - 状态（成功/失败）
   - 执行时长
   - 日志输出
   - 错误信息
3. 使用筛选器按 Action、日期范围、状态过滤

### 调试技巧

**使用 console.log**:
```javascript
async function handler(context) {
  console.log('Context:', JSON.stringify(context, null, 2));
  console.log('User email:', context.user.email);
  
  try {
    const result = await someOperation();
    console.log('Operation result:', result);
  } catch (error) {
    console.error('Operation failed:', error.message);
    throw error;
  }
}
```

**错误处理**:
```javascript
async function handler(context) {
  try {
    // 主逻辑
    await sendEmail(context.user.email);
    return { success: true };
  } catch (error) {
    // 记录详细错误
    console.error('Error details:', {
      message: error.message,
      stack: error.stack,
      user: context.user.id
    });
    
    // 可以选择抛出错误（标记为失败）或返回（标记为成功但有警告）
    return { success: false, error: error.message };
  }
}
```

## 性能优化

### Runtime 复用

Auth9 使用线程本地 Runtime 复用技术，显著提升 Action 执行性能：

| 指标 | 时间 |
|------|------|
| 首次执行 | ~15ms（包含 V8 初始化） |
| 后续执行 | ~0.16ms（复用 Runtime） |
| 性能提升 | 91.3 倍 |

### 最佳实践

1. **最小化外部 API 调用**
   ```javascript
   // ❌ 不好：多次重复调用
   await fetch(url1);
   await fetch(url2);
   await fetch(url3);
   
   // ✅ 好：批量调用
   const [r1, r2, r3] = await Promise.all([
     fetch(url1),
     fetch(url2),
     fetch(url3)
   ]);
   ```

2. **缓存配置数据**
   ```javascript
   // 将静态配置存储在外部，而不是硬编码
   const config = await fetch('https://api.example.com/config').then(r => r.json());
   ```

3. **超时控制**
   ```javascript
   // 为外部 API 调用设置超时
   const controller = new AbortController();
   const timeout = setTimeout(() => controller.abort(), 5000);
   
   try {
     const response = await fetch(url, { signal: controller.signal });
     return await response.json();
   } finally {
     clearTimeout(timeout);
   }
   ```

4. **避免阻塞操作**
   ```javascript
   // ❌ 不好：长时间计算阻塞执行
   for (let i = 0; i < 1000000; i++) {
     heavyComputation();
   }
   
   // ✅ 好：将重计算移到外部服务
   const result = await fetch('https://api.example.com/compute', {
     method: 'POST',
     body: JSON.stringify({ data })
   }).then(r => r.json());
   ```

## 监控和统计

### Action 统计信息

在 Action 列表页面，每个 Action 显示：
- **Total Executions**: 总执行次数
- **Success Rate**: 成功率
- **Avg Duration**: 平均执行时长
- **Last Run**: 最后执行时间

### 查看详细统计

1. 点击 Action 名称进入详情页
2. 查看统计图表：
   - 执行次数趋势
   - 成功率变化
   - 执行时长分布
3. 分析失败原因和性能瓶颈

## 安全性

### V8 沙箱隔离

Action Engine 使用 V8 isolate 沙箱技术，确保：
- ✅ 用户代码与系统代码完全隔离
- ✅ 无法访问文件系统
- ✅ 无法执行系统命令
- ✅ 无法访问其他租户的数据
- ✅ 内存和 CPU 使用限制

### 安全审计

所有 Action 执行都会记录审计日志：
- 谁创建/修改了 Action
- 何时执行
- 执行结果
- 访问的资源

### 权限控制

- 只有租户管理员可以创建和修改 Action
- 普通用户无法查看 Action 代码
- 密钥只能被 Action 代码访问，无法在界面中查看

## 常见用例

### 1. 用户入职自动化

```javascript
// 新用户注册后自动执行
async function handler(context) {
  const { user, tenant } = context;
  
  // 1. 发送欢迎邮件
  await sendWelcomeEmail(user.email, tenant.name);
  
  // 2. 创建默认配置
  await fetch(`https://api.yourservice.com/users/${user.id}/init`, {
    method: 'POST',
    headers: { 'Authorization': `Bearer ${context.secrets.API_KEY}` },
    body: JSON.stringify({ user_id: user.id })
  });
  
  // 3. 同步到 CRM
  await fetch('https://api.hubspot.com/contacts/v1/contact', {
    method: 'POST',
    headers: { 'Authorization': `Bearer ${context.secrets.HUBSPOT_KEY}` },
    body: JSON.stringify({
      properties: [
        { property: 'email', value: user.email },
        { property: 'firstname', value: user.first_name },
        { property: 'lastname', value: user.last_name }
      ]
    })
  });
  
  return { success: true };
}
```

### 2. 登录行为分析

```javascript
// 记录登录行为到分析平台
async function handler(context) {
  const { user, event } = context;
  
  // 发送到 Google Analytics
  await fetch('https://www.google-analytics.com/collect', {
    method: 'POST',
    body: new URLSearchParams({
      v: '1',
      tid: context.secrets.GA_TRACKING_ID,
      cid: user.id,
      t: 'event',
      ec: 'Authentication',
      ea: 'Login',
      el: event.device_type,
      cd1: event.ip_address,
      cd2: event.location
    })
  });
  
  return { tracked: true };
}
```

### 3. 实时 Slack 通知

```javascript
// 发送 Slack 通知
async function handler(context) {
  const { user, event } = context;
  
  await fetch(context.secrets.SLACK_WEBHOOK_URL, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      text: `🎉 新用户注册`,
      blocks: [
        {
          type: 'section',
          text: {
            type: 'mrkdwn',
            text: `*新用户注册*\n• 邮箱: ${user.email}\n• 姓名: ${user.name}\n• 租户: ${context.tenant.name}`
          }
        }
      ]
    })
  });
  
  return { notified: true };
}
```

### 4. 数据同步

```javascript
// 同步用户数据到外部系统
async function handler(context) {
  const { user } = context;
  
  // 同步到多个系统
  const results = await Promise.allSettled([
    // Salesforce
    syncToSalesforce(user),
    // Intercom
    syncToIntercom(user),
    // 内部数据仓库
    syncToDataWarehouse(user)
  ]);
  
  // 记录同步结果
  results.forEach((result, index) => {
    if (result.status === 'rejected') {
      console.error(`Sync ${index} failed:`, result.reason);
    }
  });
  
  return {
    synced: results.filter(r => r.status === 'fulfilled').length,
    failed: results.filter(r => r.status === 'rejected').length
  };
}

async function syncToSalesforce(user) {
  // Salesforce API 调用
}

async function syncToIntercom(user) {
  // Intercom API 调用
}

async function syncToDataWarehouse(user) {
  // 数据仓库 API 调用
}
```

## 限制和注意事项

### 执行限制

- **超时时间**: 30 秒（超时后自动终止）
- **内存限制**: 128MB
- **并发执行**: 最多 100 个 Action 同时执行
- **日志大小**: 单次执行最多 10KB 日志

### 触发器限制

- 每个触发器最多绑定 10 个 Action
- Action 按优先级顺序执行（未来支持）
- 失败的 Action 不会阻止后续 Action 执行

### API 速率限制

- 外部 API 调用需遵守对方的速率限制
- 建议实现重试逻辑和指数退避

## 故障排查

### Action 未执行

**可能原因**：
1. Action 未启用 - 检查 Enabled 开关
2. 触发器配置错误 - 验证触发器类型
3. 代码有语法错误 - 查看错误日志

**解决方法**：
- 使用 Test 功能验证代码
- 检查执行日志中的错误信息
- 确认触发器事件已正确触发

### 外部 API 调用失败

**可能原因**：
1. 密钥配置错误
2. 网络问题
3. API 端点不可达
4. 速率限制

**解决方法**：
```javascript
async function handler(context) {
  try {
    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${context.secrets.API_KEY}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(data)
    });
    
    if (!response.ok) {
      const error = await response.text();
      console.error('API Error:', {
        status: response.status,
        statusText: response.statusText,
        body: error
      });
      throw new Error(`API returned ${response.status}`);
    }
    
    return await response.json();
  } catch (error) {
    console.error('Request failed:', error.message);
    
    // 实现重试逻辑
    if (shouldRetry(error)) {
      await new Promise(resolve => setTimeout(resolve, 1000));
      // 重试...
    }
    
    throw error;
  }
}

function shouldRetry(error) {
  // 网络错误或 5xx 错误可以重试
  return error.message.includes('network') || 
         error.message.includes('timeout') ||
         error.message.includes('500');
}
```

### 性能问题

**症状**：Action 执行时间过长

**排查步骤**：
1. 查看执行日志，定位耗时操作
2. 检查是否有不必要的顺序 API 调用
3. 使用 `console.time()` 和 `console.timeEnd()` 测量

```javascript
async function handler(context) {
  console.time('total');
  
  console.time('fetch-user-data');
  const userData = await fetchUserData();
  console.timeEnd('fetch-user-data');
  
  console.time('process-data');
  const processed = processData(userData);
  console.timeEnd('process-data');
  
  console.timeEnd('total');
  
  return processed;
}
```

## 相关文档

- [架构设计](架构设计.md) - Action Engine 技术架构
- [REST API](REST-API.md) - Action API 端点参考
- [Webhook 集成](Webhook集成.md) - 与 Webhook 配合使用
- [最佳实践](最佳实践.md) - Action 开发最佳实践

---

**最后更新**: 2026-02-16
**适用版本**: Auth9 v0.1.0+
