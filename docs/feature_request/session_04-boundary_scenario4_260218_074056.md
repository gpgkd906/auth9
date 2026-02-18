# Feature Request: 可疑 IP 黑名单

**Created**: 2026-02-18 07:40:56
**Source**: QA Document `docs/qa/session/04-boundary.md` Scenario #4

---

## 需求描述
实现已知恶意 IP 黑名单功能，在可疑 IP 尝试登录时触发告警。

## 当前行为
- security_alerts 表有 'suspicious_ip' alert_type
- 该告警仅在密码喷洒攻击检测（check_password_spray）时触发
- 没有独立的 IP 黑名单检查逻辑
- 无 IP 黑名单配置表

## 期望行为
- 管理员可配置恶意 IP 黑名单
- 从黑名单 IP 尝试登录时触发 suspicious_ip 告警
- 告警 severity = 'critical'

## 相关组件
- Backend: security detection service
- Database: 新增 IP 黑名单表

## Severity
Medium
