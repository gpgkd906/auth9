# Supply Chain: path-to-regexp ReDoS 漏洞修复

**类型**: 供应链安全
**严重程度**: High (Dependabot)，实际可利用性 Low
**影响范围**: auth9-portal (Frontend)
**前置依赖**: @react-router/serve 或 express 上游更新
**被依赖**: 无

---

## 背景

GitHub Dependabot 检测到 2 个 HIGH 级别的 path-to-regexp ReDoS 安全漏洞。该漏洞来自传递依赖链：

```
auth9-portal → @react-router/serve@7.13.1 → express@4.22.1 → path-to-regexp@0.1.13
```

### 为什么不能直接修复

- `npm overrides` 将 path-to-regexp 升级到 v8+ 会破坏 express v4（v8 API 完全不兼容 v0.1.x）
- express v4 内部使用 `pathRegexp(path, keys, opts)` 函数签名，v8 已移除该接口
- 修复需要 express 升级到 v5（使用 path-to-regexp v8+），或 @react-router/serve 更新其 express 依赖

### 实际风险评估

- ReDoS 需要构造特定的多参数路由请求（如 `/:a{-:b}*` 模式）
- auth9-portal 使用 @react-router/serve 作为 SSR 服务容器，路由由 React Router 管理，不暴露用户可控的 express 多参数路由
- 实际可利用性: **Low**

---

## 期望行为

- R1: 消除 Dependabot HIGH severity alerts
- R2: 不引入 express runtime 兼容性问题
- R3: Portal SSR 服务正常运行

---

## 涉及文件

| 文件 | 变更类型 |
|------|---------|
| `auth9-portal/package.json` | 更新 @react-router/serve 或 express 版本 |
| `auth9-portal/package-lock.json` | 自动更新 |

---

## 验证方法

1. `npm audit` 无 HIGH/CRITICAL 漏洞
2. `gh api repos/:owner/:repo/dependabot/alerts` 无 open path-to-regexp 警报
3. Portal SSR 正常启动和运行
4. E2E 测试通过

---

## 建议修复时机

等待以下任一条件满足后执行：
1. @react-router/serve 更新 express 依赖到 v5+
2. express v4 发布包含安全修复的 path-to-regexp 版本
3. React Router 团队提供替代的 SSR serve 方案

---
*Created from ticket: supply-chain_path-to-regexp_20260329_143000.md*
