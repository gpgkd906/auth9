# Feature Request: 租户级 IP 黑名单隔离

**Created**: 2026-03-13 00:00:01
**Source**: Follow-up to QA Document `docs/qa/session/04-boundary.md` Scenario #4

---

## 需求描述
在现有平台级恶意 IP 黑名单之外，补充租户级 IP 黑名单隔离能力，使各租户可独立维护自己的封禁 IP 列表。

## 当前行为
- 计划中的平台级黑名单会对全局登录流量生效
- 不支持按租户维护差异化黑名单
- 同一个 IP 无法在不同租户上配置不同封禁策略

## 期望行为
- 每个租户可独立配置自己的 IP 黑名单
- 登录事件仅匹配当前租户的黑名单，不影响其他租户
- 平台级黑名单与租户级黑名单可并存，平台级优先级高于租户级

## 相关组件
- Backend: security detection service
- Backend: tenant-scoped security settings API / policy
- Database: 新增 tenant_id 维度的 IP 黑名单表或扩展现有黑名单模型
- Frontend: tenant security settings

## Severity
Medium
