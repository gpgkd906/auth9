# 基础设施安全 - 依赖漏洞审计

**模块**: 基础设施安全
**测试范围**: 第三方依赖安全
**场景数**: 4
**风险等级**: 🟠 高
**ASVS 5.0 矩阵ID**: M-INFRA-03
**OWASP ASVS 5.0**: V13.1,V15.1,V15.2
**回归任务映射**: Backlog #14, #20


---

## 背景知识

Auth9 依赖生态：
- **Rust (auth9-core)**: Cargo.toml 管理
- **TypeScript (auth9-portal)**: package.json 管理
- **Docker 镜像**: 基础镜像和运行时
- **系统依赖**: 操作系统包

风险来源：
- 已知漏洞 (CVE)
- 恶意包 (Supply Chain)
- 过时版本

---

## 场景 1：Rust 依赖审计

### 前置条件
- auth9-core 源代码
- cargo-audit 工具

### 攻击目标
检测 Rust 依赖中的已知漏洞

### 攻击步骤
1. 安装 cargo-audit
2. 扫描 Cargo.lock
3. 分析漏洞报告
4. 评估影响和优先级

### 预期安全行为
- 无高危/严重漏洞
- 定期更新依赖
- CI/CD 自动扫描

### 验证方法
```bash
# 安装 cargo-audit
cargo install cargo-audit

# 进入项目目录
cd auth9-core

# 运行审计
cargo audit

# 输出格式化 JSON
cargo audit --json > audit-report.json

# 检查特定 advisory
cargo audit --ignore RUSTSEC-2022-0001

# 检查过时依赖
cargo outdated

# 示例输出:
# Crate:     tokio
# Version:   1.25.0
# Warning:   unmaintained
# Advisory:  RUSTSEC-2023-XXXX
# Severity:  high
```

### 已知传递依赖漏洞

| Crate | 版本 | 原因 | 状态 |
|-------|------|------|------|
| `rustls-webpki` | 0.101.7 | 被 AWS SDK 的 `hyper-rustls 0.24.2` 间接依赖 | 无法直接升级，需等待 AWS SDK 更新依赖链 |

> **说明**: 其他 2 个高危漏洞（`aws-lc-sys`）已通过 `cargo update` 修复。
> `rustls-webpki 0.101.7` 是 AWS SDK 传递依赖锁定的版本，auth9-core 无法单独升级。
> 此漏洞的实际风险较低，因为 TLS 连接仅用于出站 AWS 服务调用。

### 修复建议
- 升级有漏洞的依赖
- 锁定版本 (Cargo.lock)
- CI 集成 cargo-audit
- 定期运行 cargo update

---

## 场景 2：Node.js 依赖审计

### 前置条件
- auth9-portal 源代码
- npm/yarn

### 攻击目标
检测 Node.js 依赖中的已知漏洞

### 攻击步骤
1. 运行 npm audit
2. 分析漏洞报告
3. 检查 dev 和 prod 依赖
4. 评估传递依赖

### 预期安全行为
- 无高危/严重漏洞
- prod 依赖优先修复
- dev 依赖适时更新

### 验证方法
```bash
# 进入项目目录
cd auth9-portal

# npm 审计
npm audit

# 详细报告
npm audit --json > audit-report.json

# 仅生产依赖
npm audit --omit=dev

# 尝试自动修复
npm audit fix

# 强制修复 (可能有破坏性)
npm audit fix --force

# 使用 Snyk (更全面)
npx snyk test

# 检查过时依赖
npm outdated
```

### 修复建议
- 定期运行 npm audit
- CI 集成审计检查
- 使用 dependabot 自动 PR
- 审查依赖树减少传递依赖

---

## 场景 3：Docker 镜像扫描

### 前置条件
- Docker 镜像
- 镜像扫描工具

### 攻击目标
检测 Docker 镜像中的漏洞

### 攻击步骤
1. 扫描基础镜像
2. 扫描应用镜像
3. 检查镜像层
4. 分析操作系统包漏洞

### 预期安全行为
- 使用最小化基础镜像
- 无高危 OS 漏洞
- 定期重建镜像

### 验证方法
```bash
# 使用 Trivy
# 安装
brew install aquasecurity/trivy/trivy

# 扫描镜像
trivy image auth9-core:latest
trivy image auth9-portal:latest

# JSON 输出
trivy image --format json -o report.json auth9-core:latest

# 仅高危漏洞
trivy image --severity HIGH,CRITICAL auth9-core:latest

# 扫描 Dockerfile
trivy config Dockerfile

# 使用 Docker Scout (Docker Desktop)
docker scout cves auth9-core:latest

# 使用 Snyk
snyk container test auth9-core:latest
```

### 修复建议
- 使用 `distroless` 或 `alpine` 基础镜像
- 定期更新基础镜像
- 多阶段构建减少攻击面
- CI 集成镜像扫描

### 已知基础镜像漏洞（2026-02-25 评估）

以下漏洞存在于 `debian:bookworm-slim` 基础镜像中，**无可用补丁**，属于已知接受风险：

| 库 | CVE | 严重性 | 状态 | 影响评估 |
|----|-----|--------|------|----------|
| zlib1g | CVE-2023-45853 | CRITICAL | will_not_fix | 影响 minizip 组件（`zipOpenNewFileInZip4_6`），auth9-core 不创建 ZIP 文件，实际风险低 |
| libc-bin/libc6 | CVE-2026-0861 | HIGH | affected | glibc memalign 整数溢出，需特定内存分配模式触发 |
| libldap-2.5-0 | CVE-2023-2953 | HIGH | affected | openldap 空指针解引用，auth9-core 不直接使用 LDAP |

**缓解措施**:
- Dockerfile 已包含 `apt-get upgrade -y` 确保可用补丁已应用
- 运行时以非 root 用户（`auth9`）执行
- auth9-portal 和 auth9-demo 已使用 `node:20-alpine` 基础镜像（更少 CVE）

**后续追踪**: 考虑将 auth9-core 运行时切换到 `gcr.io/distroless/cc-debian12` 以消除非必要系统包。

---

## 场景 4：供应链安全

### 前置条件
- 包管理配置
- CI/CD 访问

### 攻击目标
评估供应链攻击风险

### 攻击步骤
1. 检查依赖来源
2. 验证包完整性
3. 检查 CI/CD 安全
4. 评估 typosquatting 风险

### 预期安全行为
- 使用官方注册表
- 验证包签名/校验和
- 锁定依赖版本
- 审计新依赖

### 验证方法
```bash
# 检查 npm registry 配置
npm config get registry
# 预期: https://registry.npmjs.org/

# 检查 Cargo registry
cat ~/.cargo/config.toml | grep registry

# 验证包完整性
# npm 使用 package-lock.json 的 integrity 字段
grep "integrity" package-lock.json | head -5

# 检查可疑依赖名称
# 搜索类似知名包的名称 (typosquatting)
npm ls | grep -E "loadsh|reqeusts|colros"

# 检查依赖许可证
npx license-checker --summary

# 使用 Socket.dev 检查
npx socket npm info <package-name>
```

### 修复建议
- 使用官方注册表
- 锁定依赖版本
- 审计新依赖添加
- 使用私有镜像仓库
- 启用 2FA for npm publish

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | Rust 依赖审计 | ✅ | 2026-03-24 | QA | 1个已知漏洞(RUSTSEC-2026-0049)，3个警告(已维护) |
| 2 | Node.js 依赖审计 | ✅ | 2026-03-24 | QA | 0漏洞 |
| 3 | Docker 镜像扫描 | ⚠️ | 2026-03-24 | QA | 4个已知OS漏洞(已在文档记录)，6个node-tar/esbuild漏洞 |
| 4 | 供应链安全 | ✅ | 2026-03-24 | QA | npm registry官方，package-lock.json有integrity字段 |

---

## 自动化工具集成

### GitHub Actions 示例

```yaml
name: Security Audit

on:
  push:
    branches: [main]
  schedule:
    - cron: '0 0 * * 1'  # 每周一

jobs:
  rust-audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Rust Audit
        run: |
          cargo install cargo-audit
          cd auth9-core && cargo audit

  npm-audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: NPM Audit
        run: |
          cd auth9-portal && npm ci
          npm audit --audit-level=high

  trivy-scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build Image
        run: docker build -t auth9-core:test ./auth9-core
      - name: Trivy Scan
        uses: aquasecurity/trivy-action@master
        with:
          image-ref: 'auth9-core:test'
          severity: 'CRITICAL,HIGH'
          exit-code: '1'
```

### Dependabot 配置

```yaml
# .github/dependabot.yml
version: 2
updates:
  - package-ecosystem: "cargo"
    directory: "/auth9-core"
    schedule:
      interval: "weekly"
    open-pull-requests-limit: 5

  - package-ecosystem: "npm"
    directory: "/auth9-portal"
    schedule:
      interval: "weekly"
    open-pull-requests-limit: 5

  - package-ecosystem: "docker"
    directory: "/"
    schedule:
      interval: "weekly"
```

---

## 漏洞响应流程

1. **检测**: 自动扫描发现漏洞
2. **评估**: 判断严重性和影响范围
3. **优先级**:
   - CRITICAL: 24 小时内修复
   - HIGH: 7 天内修复
   - MEDIUM: 30 天内修复
   - LOW: 下个版本修复
4. **修复**: 更新依赖或实施缓解措施
5. **验证**: 重新扫描确认修复
6. **部署**: 发布修复版本

---

## 参考资料

- [OWASP Dependency Check](https://owasp.org/www-project-dependency-check/)
- [Snyk Vulnerability Database](https://snyk.io/vuln/)
- [RustSec Advisory Database](https://rustsec.org/)
- [npm Advisory Database](https://www.npmjs.com/advisories)
- [CWE-1104: Use of Unmaintained Third Party Components](https://cwe.mitre.org/data/definitions/1104.html)

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-INFRA-03  
**适用控制**: V13.1,V15.1,V15.2  
**关联任务**: Backlog #14, #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 4

### 执行清单
- [ ] M-INFRA-03-C01 | 控制: V13.1 | 任务: #14, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-INFRA-03-C02 | 控制: V15.1 | 任务: #14, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-INFRA-03-C03 | 控制: V15.2 | 任务: #14, #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
