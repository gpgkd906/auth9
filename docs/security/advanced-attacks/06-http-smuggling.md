# 高级攻击 - HTTP 请求走私测试

**模块**: 高级攻击
**测试范围**: HTTP Request Smuggling、HTTP 降级攻击
**场景数**: 2
**风险等级**: 🟠 高
**OWASP ASVS**: V13.1, V9.1

---

## 背景知识

Auth9 生产部署通常在反向代理（Nginx/Traefik）后面：
```
Client → Reverse Proxy (Nginx) → Auth9 Core (axum/hyper)
                                → Auth9 Portal (Vite/Express)
```

HTTP 请求走私利用前端代理和后端服务器对 HTTP 消息解析的差异，可能导致：
- 绕过认证/授权检查
- 缓存投毒
- 请求劫持
- 访问未授权端点

Auth9 使用 Rust axum（基于 hyper），hyper 对 HTTP 解析较严格，但仍需验证。

---

## 场景 1：CL-TE / TE-CL 走私攻击

### 前置条件
- Auth9 部署在反向代理后面
- 能够发送原始 HTTP 请求（netcat/burp）

### 攻击目标
验证 Content-Length 和 Transfer-Encoding 不一致时的处理行为

### 攻击步骤
1. 发送同时包含 Content-Length 和 Transfer-Encoding 的请求
2. CL-TE 攻击：前端用 Content-Length，后端用 Transfer-Encoding
3. TE-CL 攻击：前端用 Transfer-Encoding，后端用 Content-Length
4. TE-TE 攻击：使用混淆的 Transfer-Encoding 头
5. 检查是否可以注入第二个请求

### 预期安全行为
- 同时存在 CL 和 TE 时，优先使用 TE（RFC 7230 Section 3.3.3）
- 或直接拒绝含冲突头的请求
- hyper 默认行为：拒绝冲突头或使用 TE
- 反向代理和后端解析行为一致

### 验证方法
```bash
# CL-TE 走私尝试
# 使用 netcat 发送原始请求
printf "POST /api/v1/auth/token HTTP/1.1\r\n\
Host: localhost:8080\r\n\
Content-Length: 13\r\n\
Transfer-Encoding: chunked\r\n\
\r\n\
0\r\n\
\r\n\
G" | nc localhost 8080
# 如果存在走私，"G" 会被当作下一个请求的开始
# 预期: 400 Bad Request 或正常处理第一个请求

# TE-CL 走私尝试
printf "POST /api/v1/auth/token HTTP/1.1\r\n\
Host: localhost:8080\r\n\
Content-Length: 3\r\n\
Transfer-Encoding: chunked\r\n\
\r\n\
8\r\n\
SMUGGLED\r\n\
0\r\n\
\r\n" | nc localhost 8080
# 预期: 400 或正常处理（不走私 "SMUGGLED" 部分）

# 混淆的 Transfer-Encoding
printf "POST / HTTP/1.1\r\n\
Host: localhost:8080\r\n\
Transfer-Encoding: chunked\r\n\
Transfer-Encoding: x\r\n\
\r\n\
0\r\n\
\r\n" | nc localhost 8080
# 预期: 400 或忽略无效 TE

# 使用 smuggler 工具自动化
# https://github.com/defparam/smuggler
python3 smuggler.py -u http://localhost:8080/api/v1/auth/token

# 使用 Burp Suite HTTP Request Smuggler 扩展
# 自动检测 CL-TE, TE-CL, TE-TE 各种变体
```

### 修复建议
- 反向代理层标准化 HTTP 头（移除冲突的 CL/TE）
- Nginx: `proxy_http_version 1.1; proxy_set_header Connection "";`
- hyper 默认拒绝冲突头（Rust 生态通常安全）
- 使用 HTTP/2 到后端（避免 HTTP/1.1 解析歧义）
- 定期使用 smuggler 工具扫描

---

## 场景 2：HTTP/2 降级攻击

### 前置条件
- 反向代理支持 HTTP/2 但后端使用 HTTP/1.1
- 或直接 HTTP/2 连接到 Auth9

### 攻击目标
验证 HTTP/2 到 HTTP/1.1 降级场景下的安全性

### 攻击步骤
1. 使用 HTTP/2 发送包含 HTTP/1.1 特殊字符的头部
2. 利用 HTTP/2 的 CONTINUATION 帧攻击
3. 在 HTTP/2 pseudo-header 中注入路径遍历
4. 利用 HTTP/2 header 名大小写差异
5. 测试 HTTP/2 HPACK 头部压缩解压炸弹

### 预期安全行为
- HTTP/2 头部在降级为 HTTP/1.1 前被正确验证
- 特殊字符（`\r\n`）在头部中被拒绝
- 路径遍历在 pseudo-header 中被规范化
- HPACK 解压有内存限制
- CONTINUATION 帧有数量限制

### 验证方法
```bash
# HTTP/2 伪头部注入
curl --http2 -X POST \
  -H "Transfer-Encoding: chunked" \
  http://localhost:8080/api/v1/auth/token
# HTTP/2 中 Transfer-Encoding 通常被忽略或禁止

# HTTP/2 路径操控
curl --http2 "http://localhost:8080/api/v1/../internal/debug"
# 预期: 404 (路径遍历被规范化)

# HTTP/2 伪头部大小写
# HTTP/2 要求头部名全部小写
# 使用 h2c 等工具发送大写头部
python3 << 'PYEOF'
import h2.connection
import h2.config
import socket

# 建立 HTTP/2 明文连接
sock = socket.create_connection(('localhost', 8080))
config = h2.config.H2Configuration(client_side=True)
conn = h2.connection.H2Connection(config=config)
conn.initiate_connection()
sock.sendall(conn.data_to_send())

# 发送带有异常头部的请求
headers = [
    (':method', 'GET'),
    (':path', '/api/v1/tenants'),
    (':authority', 'localhost:8080'),
    (':scheme', 'http'),
    ('authorization', 'Bearer TOKEN'),
    # HTTP/2 不允许 Connection, Transfer-Encoding 等头
    # ('transfer-encoding', 'chunked'),  # 应被拒绝
]
conn.send_headers(1, headers, end_stream=True)
sock.sendall(conn.data_to_send())

resp = sock.recv(65535)
print(resp)
PYEOF

# HPACK bomb (大量引用相同头部)
# 使用 h2spec 工具
h2spec -h localhost -p 8080 --strict
# 检查 HTTP/2 协议合规性

# CONTINUATION 帧洪水
# 使用 httpwg/h2c 发送大量 CONTINUATION 帧
```

### 修复建议
- 反向代理使用 HTTP/2 到后端（end-to-end HTTP/2）
- 如必须降级，使用标准化的代理配置
- hyper 自带 HTTP/2 支持，避免手动降级
- 配置 HTTP/2 的 `SETTINGS_MAX_HEADER_LIST_SIZE`
- 限制 CONTINUATION 帧数量
- 定期运行 h2spec 合规性测试

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | CL-TE / TE-CL 走私攻击 | ☐ | | | |
| 2 | HTTP/2 降级攻击 | ☐ | | | |

---

## 自动化测试工具

```bash
# smuggler - HTTP 走私检测
# https://github.com/defparam/smuggler
python3 smuggler.py -u https://target.com/

# h2spec - HTTP/2 合规性测试
# https://github.com/summerwind/h2spec
h2spec -h localhost -p 8080

# Burp Suite - HTTP Request Smuggler 扩展
# https://portswigger.net/bappstore/aaaa60ef945341e8a450217a54a11646

# http-desync-guardian (AWS)
# https://github.com/aws/http-desync-guardian
```

---

## 参考资料

- [PortSwigger HTTP Request Smuggling](https://portswigger.net/web-security/request-smuggling)
- [CWE-444: Inconsistent Interpretation of HTTP Requests](https://cwe.mitre.org/data/definitions/444.html)
- [RFC 7540 - HTTP/2](https://datatracker.ietf.org/doc/html/rfc7540)
- [HTTP/2: The Sequel is Always Worse (Black Hat)](https://www.blackhat.com/us-21/briefings/schedule/#http2-the-sequel-is-always-worse-22668)
