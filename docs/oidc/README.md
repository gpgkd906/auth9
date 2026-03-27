# Auth9 OIDC Conformance 测试文档

本目录包含 Auth9 OIDC 引擎的 Conformance 测试用例，基于 OpenID Foundation Conformance Suite 和 OIDC Core 规范设计。

## 环境准备

```bash
# 启动含 OIDC Conformance Suite 的环境
./scripts/reset-docker.sh --conformance

# 预置 OIDC 测试客户端
./scripts/oidc-conformance-setup.sh
```

## 支持的 OIDC 流程

| 流程 | 状态 | 说明 |
|------|------|------|
| Authorization Code | 支持 | 含 PKCE (S256) |
| Client Credentials | 支持 | |
| Refresh Token | 支持 | 含 replay protection |
| Implicit | 不支持 | |
| Hybrid | 不支持 | |
| Device Authorization | 不支持 | |

## 测试用例索引

### 环境搭建 (1 个文档, 3 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [setup/01-conformance-suite.md](./setup/01-conformance-suite.md) | 环境重置、Conformance Suite 就绪验证、OIDC Client 预置 | 3 |

### Discovery & JWKS (1 个文档, 5 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [discovery/01-discovery-jwks.md](./discovery/01-discovery-jwks.md) | Discovery JSON 完整性、端点可达性、JWKS 格式验证 | 5 |

### Authorization Code Flow (2 个文档, 10 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [authz-code/01-basic-flow.md](./authz-code/01-basic-flow.md) | Authorization Code 基本流程、Token Exchange、错误处理 | 5 |
| [authz-code/02-pkce-flow.md](./authz-code/02-pkce-flow.md) | PKCE S256 流程、Verifier 验证、公开客户端约束 | 5 |

### Client Credentials Flow (1 个文档, 4 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [client-credentials/01-basic-flow.md](./client-credentials/01-basic-flow.md) | Client Secret Basic/Post、无效凭证、Token 内容 | 4 |

### Refresh Token Flow (1 个文档, 4 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [refresh-token/01-refresh-flow.md](./refresh-token/01-refresh-flow.md) | Token 刷新、Replay Protection、错误处理 | 4 |

### Token 验证 (1 个文档, 5 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [token-validation/01-claims-signing.md](./token-validation/01-claims-signing.md) | JWT RS256 验签、Claims 完整性、时间戳 | 5 |

### UserInfo 端点 (1 个文档, 4 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [userinfo/01-userinfo-endpoint.md](./userinfo/01-userinfo-endpoint.md) | UserInfo 响应、Scope 映射、认证错误 | 4 |

### Logout 流程 (1 个文档, 4 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [logout/01-logout-flow.md](./logout/01-logout-flow.md) | GET/POST Logout、Session 撤销、参数验证 | 4 |

### 错误处理 (1 个文档, 5 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [error-handling/01-invalid-requests.md](./error-handling/01-invalid-requests.md) | 缺少参数、无效 Client、不支持 Grant Type | 5 |

### 安全参数 (1 个文档, 5 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [security/01-state-nonce-redirect.md](./security/01-state-nonce-redirect.md) | State、Redirect URI、Code Replay/Expiry | 5 |

## 总计

- **文档数**: 11
- **场景数**: 45
- **可脚本化**: 6 个文档（Fast Path）
- **需浏览器**: 4 个文档（Agent Path）

## 文档治理

- 规范文件: [docs/oidc/_standards.md](./_standards.md)
- 清单真值: [docs/oidc/_manifest.yaml](./_manifest.yaml)
- 执行命令: `./scripts/run-qa-tests.sh --only-oidc`
