# OIDC 测试文档规范

本文件定义 `docs/oidc` 的统一治理规则。继承 `docs/qa/_standards.md` 的所有基础规范，并追加 OIDC 特定约束。

## 1. 继承规则

以下规则与 `docs/qa/_standards.md` 一致，不再重复：

- 每个文档最多 5 个编号场景
- 每篇文档必须包含「检查清单」
- 涉及数据变更的场景应包含「预期数据状态」与 SQL 验证
- 测试数据仅使用 `@example.com`、`@test.com`、`@auth9.local` 域名
- 描述性文本使用中文，技术术语（SQL、API path、字段名）使用英文

## 2. OIDC 特定规则

### 2.1 环境重置

所有 OIDC 测试文档的「前置条件」必须包含：

```bash
./scripts/reset-docker.sh --conformance
```

此命令会启动包含 OIDC Conformance Suite 的完整环境。

### 2.2 Issuer 地址

- Docker 内部 issuer: `http://auth9-core:8080`（Conformance Suite 使用）
- Host 端 issuer: `http://localhost:8080`（本地脚本测试使用）
- 测试文档中需明确标注使用哪个 issuer

### 2.3 OIDC 客户端

测试用 OIDC Client 通过 `scripts/oidc-conformance-setup.sh` 预置。文档中引用客户端时使用占位符：
- `{client_id}` — 测试客户端 ID
- `{client_secret}` — 测试客户端密钥
- `{redirect_uri}` — 已注册的回调地址

### 2.4 不支持的流程

以下 OIDC 流程 Auth9 当前不支持，不应编写测试文档：
- Implicit Flow
- Hybrid Flow
- Device Authorization Flow
- Dynamic Client Registration

### 2.5 Conformance Suite 场景

涉及 Conformance Suite UI 操作的场景，操作步骤以 `https://localhost:9443` 为入口，需标注为「手动验证」。

## 3. 索引与统计

1. `docs/oidc/_manifest.yaml` 是 OIDC 文档清单的事实来源。
2. `docs/oidc/README.md` 的索引与统计必须与 manifest 一致。
3. 新增文档必须同步更新 manifest 与 README。
