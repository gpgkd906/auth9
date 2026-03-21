# 基础设施安全 - TLS 配置安全测试

**模块**: 基础设施安全
**测试范围**: TLS/SSL 配置
**场景数**: 5
**风险等级**: 🟠 高
**ASVS 5.0 矩阵ID**: M-INFRA-01
**OWASP ASVS 5.0**: V12.1,V12.2,V13.1
**回归任务映射**: Backlog #3, #13, #20


---

## 背景知识

Auth9 TLS 终端点：
- **反向代理**: Nginx/Cloudflare Tunnel (TLS 终止)
- **Auth9 OIDC Engine**: 内部通信
- **gRPC**: 可选 mTLS

安全要求：
- TLS 1.2+ (推荐 1.3)
- 强加密套件
- 有效证书链

---

## 场景 1：TLS 版本安全

### 前置条件
- HTTPS 端点可访问
- TLS 测试工具

### 攻击目标
验证是否支持不安全的 TLS 版本

### 攻击步骤
1. 测试各 TLS 版本支持：
   - SSL 3.0 (不安全)
   - TLS 1.0 (不安全)
   - TLS 1.1 (不推荐)
   - TLS 1.2 (安全)
   - TLS 1.3 (最安全)
2. 检查降级攻击防护

### 预期安全行为
- 仅支持 TLS 1.2 和 1.3
- 禁用 SSL 3.0, TLS 1.0, 1.1
- 支持 TLS_FALLBACK_SCSV

### 验证方法
```bash
# 使用 nmap
nmap --script ssl-enum-ciphers -p 443 auth9.example.com

# 使用 openssl 测试各版本
openssl s_client -connect auth9.example.com:443 -ssl3
# 预期: handshake failure

openssl s_client -connect auth9.example.com:443 -tls1
# 预期: handshake failure

openssl s_client -connect auth9.example.com:443 -tls1_1
# 预期: handshake failure

openssl s_client -connect auth9.example.com:443 -tls1_2
# 预期: 成功

openssl s_client -connect auth9.example.com:443 -tls1_3
# 预期: 成功

# testssl.sh 全面测试
./testssl.sh auth9.example.com
```

### 常见误报

| 症状 | 原因 | 解决方法 |
|------|------|---------|
| TLS 1.0/1.1 显示为 ACCEPTED | 测试对象不是 nginx TLS 终端（连接了内部 plaintext 端口） | Docker 环境中 gRPC 通过 `auth9-grpc-tls` nginx 容器暴露 50051 端口，确认测试 `localhost:50051`（nginx 代理），而非直接连接容器内部端口 |
| TLS 1.0/1.1 显示为 ACCEPTED | 从 Docker 容器内部测试，绕过了 nginx 代理 | 从宿主机测试 `localhost:50051`，不要从 Docker 内部直连 `auth9-core:50051` |
| 本地 openssl 不支持 `-tls1` 选项 | macOS/Linux 新版 OpenSSL 已移除 TLS 1.0 支持 | 使用 `nmap --script ssl-enum-ciphers` 或 `testssl.sh` 替代 |

> **注意**: Docker 部署中 auth9-core 的 gRPC 端口（50051）为 plaintext（`expose` 仅供内部），TLS 终止由 `auth9-grpc-tls` nginx 容器完成。nginx 配置文件 `deploy/nginx/grpc-tls-nginx.conf` 中已设置 `ssl_protocols TLSv1.2 TLSv1.3;`。

### 修复建议
- Nginx: `ssl_protocols TLSv1.2 TLSv1.3;`
- 禁用所有旧版本
- 启用 SCSV 降级防护
- 定期更新配置

---

## 场景 2：加密套件安全

### 前置条件
- HTTPS 端点可访问

### 攻击目标
验证加密套件配置

### 攻击步骤
1. 列举支持的加密套件
2. 检查弱加密：
   - NULL 加密
   - 出口级加密 (EXPORT)
   - RC4, DES, 3DES
   - MD5 哈希
3. 验证前向保密 (PFS)

### 预期安全行为
- 使用强加密套件
- 支持 ECDHE 密钥交换
- 禁用所有弱加密

### 验证方法
```bash
# 列出支持的加密套件
nmap --script ssl-enum-ciphers -p 443 auth9.example.com | grep -A 50 "cipher"

# 检查弱加密
openssl s_client -connect auth9.example.com:443 -cipher NULL
# 预期: no ciphers available

openssl s_client -connect auth9.example.com:443 -cipher EXPORT
# 预期: no ciphers available

openssl s_client -connect auth9.example.com:443 -cipher RC4
# 预期: no ciphers available

# 验证 PFS
openssl s_client -connect auth9.example.com:443 -cipher ECDHE
# 预期: 成功
```

### 修复建议
```nginx
# Nginx 推荐配置
ssl_ciphers 'ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384:ECDHE-ECDSA-CHACHA20-POLY1305:ECDHE-RSA-CHACHA20-POLY1305';
ssl_prefer_server_ciphers on;
```

---

## 场景 3：证书安全

### 前置条件
- HTTPS 端点可访问

### 攻击目标
验证 SSL 证书配置

### 攻击步骤
1. 检查证书有效性：
   - 是否过期
   - 域名是否匹配
   - 证书链是否完整
2. 检查证书强度：
   - 密钥长度
   - 签名算法
3. 检查证书透明度

### 预期安全行为
- 有效期内
- 域名匹配
- RSA >= 2048 位或 ECDSA >= 256 位
- SHA-256 签名

### 验证方法
```bash
# 获取证书信息
openssl s_client -connect auth9.example.com:443 </dev/null 2>/dev/null | \
  openssl x509 -text -noout

# 检查有效期
openssl s_client -connect auth9.example.com:443 </dev/null 2>/dev/null | \
  openssl x509 -dates -noout

# 检查域名
openssl s_client -connect auth9.example.com:443 </dev/null 2>/dev/null | \
  openssl x509 -subject -noout

# 检查证书链
openssl s_client -connect auth9.example.com:443 -showcerts

# 在线检查 (SSL Labs)
# https://www.ssllabs.com/ssltest/

# 证书透明度
# https://crt.sh/?q=auth9.example.com
```

### 修复建议
- 使用受信任 CA
- RSA 2048+ 或 ECDSA P-256+
- 设置证书到期提醒
- 使用证书透明度 (CT)

---

## 场景 4：HSTS 配置

### 前置条件
- HTTPS 端点可访问

### 攻击目标
验证 HSTS (HTTP Strict Transport Security) 配置

### 攻击步骤
1. 检查 HSTS 头
2. 验证各参数：
   - max-age
   - includeSubDomains
   - preload
3. 测试 HTTP 到 HTTPS 重定向

### 预期安全行为
- HSTS 头存在
- max-age >= 31536000 (1 年)
- 包含 includeSubDomains

### 验证方法
```bash
# 检查 HSTS 头
curl -I https://auth9.example.com | grep -i strict-transport-security
# 预期: Strict-Transport-Security: max-age=31536000; includeSubDomains; preload

# 检查 HTTP 重定向
curl -I http://auth9.example.com
# 预期: 301/302 重定向到 HTTPS

# 检查 HSTS Preload 状态
# https://hstspreload.org/?domain=auth9.example.com
```

### 修复建议
```nginx
# Nginx HSTS 配置
add_header Strict-Transport-Security "max-age=31536000; includeSubDomains; preload" always;
```

### 常见误报

| 症状 | 原因 | 解决方法 |
|------|------|---------|
| 内部服务端口返回 HSTS 头 | 某些服务在所有响应中添加 HSTS 头，与协议无关 | 浏览器会忽略 HTTP 上的 HSTS 头，无安全影响。本场景应在生产 HTTPS 端点上验证 |
| 本地 Docker 环境无 HTTPS 端点 | 本地开发使用 HTTP | HSTS 测试仅适用于**生产环境**（有 TLS 终止的 Nginx/Cloudflare），本地 Docker 环境跳过此场景 |

> **注意**: Auth9 Core 的 HSTS 实现（`security_headers.rs`）默认 `hsts_https_only=true`，仅在 HTTPS 请求上返回 HSTS 头。

---

## 场景 5：内部服务通信安全

> **注意：本场景仅适用于生产/Kubernetes 环境。** 本地 Docker 开发环境有意使用明文连接（Redis `redis://`、TiDB `mysql://`），这是预期行为，不构成安全缺陷。docker-compose.yml 中已标注生产环境应启用 TLS。

### 前置条件
- **生产或预发布环境**（非本地 Docker 开发环境）
- 集群内部访问

### 攻击目标
验证内部服务间通信安全

### 攻击步骤
1. 检查服务间通信：
   - Portal → Core (HTTP?)
   - Core → Auth9 OIDC Engine (内部通信)
   - Core → TiDB (加密?)
   - Core → Redis (加密?)
2. 测试 mTLS (如果启用)

### 预期安全行为
- 内部通信至少使用 TLS
- 敏感服务使用 mTLS
- 数据库连接加密

### 验证方法
```bash
# 检查服务连接配置
# 在 Pod 内部测试
kubectl exec -it auth9-core-xxx -- sh

# 检查内部服务通信安全
# Auth9 OIDC Engine 为内置组件，无需独立 TLS 配置

# 检查到 TiDB 的连接
# 查看连接字符串是否使用 TLS
cat /app/config.yaml | grep database

# 检查 Redis 连接
redis-cli -h redis -p 6379 info server
# 检查 TLS 配置

# gRPC mTLS 测试
grpcurl -cacert ca.crt -cert client.crt -key client.key \
  localhost:50051 grpc.health.v1.Health/Check
```

### 修复建议
- 服务间强制 TLS
- 生产环境使用 mTLS
- 数据库启用 TLS
- 使用 Service Mesh (Istio)

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | TLS 版本安全 | ☐ | | | |
| 2 | 加密套件安全 | ☐ | | | |
| 3 | 证书安全 | ☐ | | | |
| 4 | HSTS 配置 | ☐ | | | |
| 5 | 内部服务通信安全 | ☐ | | | |

---

## 推荐 TLS 配置 (Nginx)

```nginx
# TLS 版本
ssl_protocols TLSv1.2 TLSv1.3;

# 加密套件
ssl_ciphers 'ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384:ECDHE-ECDSA-CHACHA20-POLY1305:ECDHE-RSA-CHACHA20-POLY1305';
ssl_prefer_server_ciphers off;

# 会话缓存
ssl_session_cache shared:SSL:10m;
ssl_session_timeout 1d;
ssl_session_tickets off;

# OCSP Stapling
ssl_stapling on;
ssl_stapling_verify on;
resolver 8.8.8.8 8.8.4.4 valid=300s;
resolver_timeout 5s;

# HSTS
add_header Strict-Transport-Security "max-age=31536000; includeSubDomains; preload" always;
```

---

## 参考资料

- [Mozilla SSL Configuration Generator](https://ssl-config.mozilla.org/)
- [SSL Labs Best Practices](https://github.com/ssllabs/research/wiki/SSL-and-TLS-Deployment-Best-Practices)
- [OWASP TLS Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Transport_Layer_Protection_Cheat_Sheet.html)
- [CWE-326: Inadequate Encryption Strength](https://cwe.mitre.org/data/definitions/326.html)

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-INFRA-01  
**适用控制**: V12.1,V12.2,V13.1  
**关联任务**: Backlog #3, #13, #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 5

### 执行清单
- [ ] M-INFRA-01-C01 | 控制: V12.1 | 任务: #3, #13, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-INFRA-01-C02 | 控制: V12.2 | 任务: #3, #13, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-INFRA-01-C03 | 控制: V13.1 | 任务: #3, #13, #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
