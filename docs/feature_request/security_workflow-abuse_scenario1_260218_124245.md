# Feature Request: gRPC Token Exchange 速率限制

**Created**: 2026-02-18 12:42:45
**Source**: QA Document `docs/security/business-logic/01-workflow-abuse.md` Scenario #1

---

## 需求描述
为 gRPC Token Exchange 服务添加应用层速率限制。

## 当前行为
- gRPC Token Exchange 服务没有应用层速率限制
- mTLS 认证已启用，限制了可访问服务的客户端
- API Key 认证也提供了一层保护

## 期望行为
在应用层添加速率限制，防止 Token Exchange 流程被滥用。

## 建议方案
1. 在生产环境中通过 Kubernetes Ingress 或 API Gateway 配置速率限制
2. 如果需要应用层保护，添加 gRPC 拦截器实现速率限制

## 相关代码位置
- gRPC 服务配置: `auth9-core/src/server/mod.rs:789-879`
- HTTP 速率限制实现: `auth9-core/src/middleware/rate_limit.rs`
- 配置: `auth9-core/src/config/mod.rs` (RateLimitConfig)

## Severity
Low (Infrastructure Level)
