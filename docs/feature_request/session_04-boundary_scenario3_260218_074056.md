# Feature Request: 社交登录配置

**Created**: 2026-02-18 07:40:56
**Source**: QA Document `docs/qa/session/04-boundary.md` Scenario #3

---

## 需求描述
配置社交登录提供商（Google/GitHub），允许用户使用社交账号登录。

## 当前行为
- 登录页面只显示：Email 输入框、Sign in with password、Sign in with passkey
- 没有社交登录按钮（Google/GitHub）
- 数据库 login_events 表支持 'social' event_type
- Keycloak identity provider 未配置

## 期望行为
- 登录页面显示「使用 Google 登录」/「使用 GitHub 登录」按钮
- 点击后完成 OAuth 授权流程
- 登录成功后事件类型记录为 'social'

## 相关组件
- Keycloak: Identity Provider 配置
- Frontend: /login 页面

## Severity
Medium
