# OIDC Token 验证 - Claims 与签名

| 项目 | 值 |
|------|-----|
| 模块 | Token Validation |
| 场景数 | 5 |
| 最后更新 | 2026-03-27 |

## 前置条件

1. 执行 `./scripts/reset-docker.sh --conformance` 重置环境至一致性测试状态
2. 确认 Auth9 Core 服务运行于 `http://localhost:8080`
3. 确认测试用 OAuth Client 已注册，持有有效的 `{client_id}` 和 `{client_secret}`
4. 已获取有效的 id_token（通过 Authorization Code Flow 或其他支持的 flow）
5. 安装 `jq` 和 `openssl` 命令行工具

---

## 场景 1：id_token 是有效的 RS256 签名 JWT

**目的**：验证 id_token 是结构正确的 JWT，使用 RS256 算法签名。

**步骤**：

1. 获取 id_token（此处以 client_credentials 获取 access_token 为例，实际 id_token 需通过 auth code flow）：

```bash
TOKEN="<your_id_token>"
```

2. 解码 JWT header：

```bash
echo "$TOKEN" | cut -d. -f1 | base64 -d 2>/dev/null | jq .
```

3. 验证 JWT 结构（三段式，点分隔）：

```bash
PARTS=$(echo "$TOKEN" | tr '.' '\n' | wc -l)
echo "JWT parts: $PARTS"
```

**预期结果**：

- JWT 由三部分组成（header.payload.signature）
- Header 的 `alg` 字段值为 `RS256`
- Header 的 `kid` 字段值为 `auth9-current`
- Header 的 `typ` 字段值为 `JWT`

---

## 场景 2：id_token 签名可通过 JWKS 公钥验证

**目的**：验证 id_token 的签名可以使用 JWKS endpoint 提供的公钥进行验证。

**步骤**：

1. 获取 JWKS 公钥：

```bash
JWKS=$(curl -s http://localhost:8080/.well-known/jwks.json)
echo "$JWKS" | jq .
```

2. 提取与 id_token kid 匹配的公钥：

```bash
KID=$(echo "$TOKEN" | cut -d. -f1 | base64 -d 2>/dev/null | jq -r '.kid')
echo "$JWKS" | jq --arg kid "$KID" '.keys[] | select(.kid == $kid)'
```

3. 验证公钥参数完整性：

```bash
echo "$JWKS" | jq --arg kid "$KID" '.keys[] | select(.kid == $kid) | {kty, alg, use, n, e}'
```

**预期结果**：

- JWKS endpoint 返回有效的 JSON，包含 `keys` 数组
- 存在 `kid` 为 `auth9-current` 的密钥
- 密钥的 `kty` 为 `RSA`，`alg` 为 `RS256`，`use` 为 `sig`
- 密钥包含 `n`（modulus）和 `e`（exponent）参数
- 使用该公钥可成功验证 id_token 签名

---

## 场景 3：必需 claims 存在 - iss, sub, aud, exp, iat

**目的**：验证 id_token 包含 OIDC 规范要求的所有必需 claims。

**步骤**：

1. 解码 id_token payload：

```bash
PAYLOAD=$(echo "$TOKEN" | cut -d. -f2 | base64 -d 2>/dev/null)
echo "$PAYLOAD" | jq .
```

2. 逐一检查必需 claims：

```bash
for claim in iss sub aud exp iat; do
  VALUE=$(echo "$PAYLOAD" | jq -r ".$claim // \"MISSING\"")
  echo "$claim: $VALUE"
done
```

3. 检查 id_token 特有 claims（如通过含 nonce 的请求获取）：

```bash
for claim in nonce sid email name; do
  VALUE=$(echo "$PAYLOAD" | jq -r ".$claim // \"NOT_PRESENT\"")
  echo "$claim: $VALUE"
done
```

**预期结果**：

- `iss`：非空字符串，值为 issuer URL
- `sub`：非空字符串，标识用户
- `aud`：非空，包含 `{client_id}`
- `exp`：数值类型，Unix 时间戳
- `iat`：数值类型，Unix 时间戳
- 可选 claims（`nonce`、`sid`、`email`、`name`）在对应 flow 中存在

---

## 场景 4：iss claim 与 Discovery issuer 一致

**目的**：验证 id_token 的 `iss` claim 与 OpenID Connect Discovery 文档中声明的 `issuer` 完全一致。

**步骤**：

1. 获取 Discovery 文档中的 issuer：

```bash
DISCOVERY_ISSUER=$(curl -s http://localhost:8080/.well-known/openid-configuration | jq -r '.issuer')
echo "Discovery issuer: $DISCOVERY_ISSUER"
```

2. 获取 id_token 中的 iss claim：

```bash
TOKEN_ISS=$(echo "$TOKEN" | cut -d. -f2 | base64 -d 2>/dev/null | jq -r '.iss')
echo "Token iss: $TOKEN_ISS"
```

3. 比较两者是否完全一致：

```bash
if [ "$DISCOVERY_ISSUER" = "$TOKEN_ISS" ]; then
  echo "PASS: issuer 一致"
else
  echo "FAIL: issuer 不一致 - Discovery='$DISCOVERY_ISSUER' Token='$TOKEN_ISS'"
fi
```

**预期结果**：

- Discovery 文档的 `issuer` 值为 `http://localhost:8080`
- id_token 的 `iss` claim 值为 `http://localhost:8080`
- 两者完全一致（包括协议、主机、端口、路径，无尾部斜杠差异）

---

## 场景 5：exp 在未来，iat 在过去

**目的**：验证 token 的时间类 claims 符合逻辑约束，确保签发时间和过期时间合理。

**步骤**：

1. 提取时间 claims 并与当前时间比较：

```bash
PAYLOAD=$(echo "$TOKEN" | cut -d. -f2 | base64 -d 2>/dev/null)
NOW=$(date +%s)
IAT=$(echo "$PAYLOAD" | jq -r '.iat')
EXP=$(echo "$PAYLOAD" | jq -r '.exp')

echo "当前时间 (now): $NOW"
echo "签发时间 (iat): $IAT"
echo "过期时间 (exp): $EXP"
```

2. 验证时间关系：

```bash
if [ "$IAT" -le "$NOW" ]; then
  echo "PASS: iat ($IAT) <= now ($NOW)"
else
  echo "FAIL: iat ($IAT) > now ($NOW) - 签发时间在未来"
fi

if [ "$EXP" -gt "$NOW" ]; then
  echo "PASS: exp ($EXP) > now ($NOW)"
else
  echo "FAIL: exp ($EXP) <= now ($NOW) - token 已过期"
fi
```

3. 验证 token 有效期合理：

```bash
LIFETIME=$((EXP - IAT))
echo "Token 有效期: ${LIFETIME} 秒 ($((LIFETIME / 60)) 分钟)"
```

**预期结果**：

- `iat` <= 当前时间（签发时间在过去或刚好是当前）
- `exp` > 当前时间（token 尚未过期）
- `exp` > `iat`（过期时间在签发时间之后）
- token 有效期在合理范围内（通常数分钟到数小时）

---

## 检查清单

| # | 检查项 | 场景 | 预期 | 通过 |
|---|--------|------|------|------|
| 1 | JWT 为三段式结构 | 1 | header.payload.signature | [ ] |
| 2 | JWT header alg 为 RS256 | 1 | alg=RS256 | [ ] |
| 3 | JWT header kid 为 auth9-current | 1 | kid=auth9-current | [ ] |
| 4 | JWKS endpoint 返回有效公钥 | 2 | keys 数组含匹配 kid 的 RSA 密钥 | [ ] |
| 5 | 公钥可验证 id_token 签名 | 2 | 签名验证通过 | [ ] |
| 6 | iss claim 存在且非空 | 3 | iss=http://localhost:8080 | [ ] |
| 7 | sub claim 存在且非空 | 3 | sub 标识用户 | [ ] |
| 8 | aud claim 包含 client_id | 3 | aud 含 {client_id} | [ ] |
| 9 | exp 和 iat 为数值类型 | 3 | Unix 时间戳 | [ ] |
| 10 | iss 与 Discovery issuer 完全一致 | 4 | 字符串完全匹配 | [ ] |
| 11 | exp 在未来 | 5 | exp > now | [ ] |
| 12 | iat 在过去 | 5 | iat <= now | [ ] |
| 13 | exp > iat | 5 | 有效期为正值 | [ ] |
