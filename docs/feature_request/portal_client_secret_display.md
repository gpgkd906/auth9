# Portal Client Secret 展示 (Service Detail)

**类型**: UI 增强
**严重程度**: Low
**影响范围**: auth9-portal (services detail component)
**前置依赖**: 无

---

## 背景

Services 详情页当前显示了 Client ID，但未提供 Client Secret 的展示区域。QA 测试期望 Client Secret 以默认遮蔽的方式展示，并支持复制按钮和长文本自动换行。

---

## 期望行为

### R1: Service 详情页展示 Client Secret

在 Service 详情页中新增 Client Secret 显示区域，默认以遮蔽形式呈现（如 `••••••••••••`），提供 "显示/隐藏" 切换按钮。

**涉及文件**:
- `auth9-portal/app/routes/` — Service 详情页组件

### R2: 复制到剪贴板按钮

Client Secret 旁提供复制按钮，点击后将完整 secret 值复制到系统剪贴板，并显示短暂的 "已复制" 反馈。

**涉及文件**:
- `auth9-portal/app/components/` — 可复用的 CopyButton 组件

### R3: 长 Secret 自动换行

当 Client Secret 值较长时，显示区域应正确换行，避免溢出或被截断。使用 `word-break: break-all` 或等宽字体确保可读性。

**涉及文件**:
- `auth9-portal/app/routes/` — Service 详情页样式

---

## 验证方法

### 手动验证

1. 导航到 Service 详情页
2. 确认 Client Secret 区域默认遮蔽显示
3. 点击 "显示" 按钮，确认 Secret 完整展示
4. 点击 "复制" 按钮，确认剪贴板中有正确值
5. 使用较长的 Secret 值，确认换行正常

### 代码验证

```bash
grep -r "client_secret\|clientSecret\|CopyButton" auth9-portal/app/
cd auth9-portal && npm run test
```
