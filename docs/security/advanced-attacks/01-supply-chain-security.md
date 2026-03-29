# 高级攻击 - 供应链与依赖安全测试

**模块**: 高级攻击
**测试范围**: 依赖漏洞、供应链攻击、构建安全
**场景数**: 6
**风险等级**: 🔴 极高
**ASVS 5.0 矩阵ID**: M-ADV-01
**OWASP ASVS 5.0**: V13.1,V13.2,V15.1,V15.2
**回归任务映射**: Backlog #14, #20


---

## 背景

供应链攻击是现代应用安全的重大威胁。Auth9 使用 Rust（Cargo）和 TypeScript（npm），需要验证依赖安全性。

**相关标准**:
- OWASP Top 10 2021 A06: Vulnerable and Outdated Components
- OWASP API Security Top 10: API7 - Security Misconfiguration
- SLSA Framework (Supply Chain Levels for Software Artifacts)

---

## 场景 1：已知漏洞依赖检测

### 前置条件
- Auth9 项目已构建
- 安装依赖扫描工具：
  - Rust: `cargo audit`
  - Node.js: `npm audit`

### 攻击目标
验证项目不使用已知漏洞的依赖版本

### 攻击步骤
1. 扫描 Rust 依赖漏洞：
   ```bash
   cd auth9-core
   cargo audit --json > audit-rust.json
   cat audit-rust.json | jq '.vulnerabilities.list'
   ```

2. 扫描 Node.js 依赖漏洞：
   ```bash
   cd auth9-portal
   npm audit --json > audit-npm.json
   cat audit-npm.json | jq '.vulnerabilities'
   ```

3. 检查关键依赖的版本：
   - `axum` - Web 框架
   - `jsonwebtoken` - JWT 处理
   - `sqlx` - 数据库
   - `react`, `react-router` - 前端框架

### 预期安全行为
- `cargo audit` 报告 0 高危/极高漏洞
- `npm audit` 报告 0 高危/极高漏洞
- 所有关键依赖使用最新的稳定版本
- Cargo.lock 和 package-lock.json 已提交到版本控制

### 验证方法
```bash
# 检查 Rust 依赖
cargo audit --deny warnings

# 检查 npm 依赖
npm audit --audit-level=high

# 验证锁文件存在
ls -la auth9-core/Cargo.lock
ls -la auth9-portal/package-lock.json
```

### 修复建议
- 设置 CI 自动化依赖扫描（GitHub Dependabot, Snyk）
- 定期更新依赖：`cargo update`, `npm update`
- 使用 `cargo deny` 检查许可证和安全策略
- 设置依赖更新策略（每月/每季度）

---

## 场景 2：传递依赖漏洞（Transitive Dependencies）

### 前置条件
- 项目依赖树已分析

### 攻击目标
验证间接依赖不引入安全漏洞

### 攻击步骤
1. 列出所有传递依赖：
   ```bash
   cd auth9-core
   cargo tree --edges normal --depth 10 > rust-deps-tree.txt
   
   cd auth9-portal
   npm list --all > npm-deps-tree.txt
   ```

2. 查找已知有漏洞的传递依赖：
   ```bash
   # 示例：查找 tokio 的旧版本
   cargo tree | grep -i "tokio v0"
   
   # 示例：查找 axios 的旧版本
   npm list axios
   ```

3. 尝试利用已知的传递依赖漏洞

### 预期安全行为
- 所有传递依赖版本无已知高危漏洞
- 使用 `cargo audit` 和 `npm audit` 可检测传递依赖问题
- 锁文件（Cargo.lock, package-lock.json）防止意外降级

### 验证方法
```bash
# 使用 cargo-outdated 检查过时依赖
cargo install cargo-outdated
cargo outdated

# 使用 npm-check-updates 检查过时依赖
npx npm-check-updates

# 生成依赖图（可选）
cargo tree --format "{p} {f}" | dot -Tpng > deps-graph.png
```

### 修复建议
- 定期运行 `cargo update --workspace` 和 `npm update`
- 使用 `cargo tree -d` 查找重复依赖
- 考虑使用 `cargo-minimal-versions` 测试最小依赖版本

---

## 场景 3：Typosquatting 攻击（包名劫持）

### 前置条件
- 项目依赖列表

### 攻击目标
验证依赖包名称正确，未被 typosquatting 攻击

### 攻击步骤
1. 检查 Cargo.toml 和 package.json 中的包名：
   ```bash
   # 常见的拼写错误包名
   cd auth9-core
   grep -i "toklo\|serde_jsno\|reqwuest" Cargo.toml
   
   cd auth9-portal
   grep -i "reacct\|expres\|loadash" package.json
   ```

2. 验证包的官方来源：
   ```bash
   # Rust: 检查 crates.io 官方包
   cargo search axum | head -1
   
   # Node.js: 检查 npm 官方包
   npm view react version
   ```

3. 检查是否有重复或相似的包名

### 预期安全行为
- 所有包名拼写正确
- 包来自官方 crates.io 和 npmjs.com
- 无可疑的包名相似项

### 验证方法
```bash
# 使用 typo 检测工具（如果有）
# 手动审查 Cargo.toml 和 package.json

# 检查包的下载量和维护者
cargo info axum
npm info react

# 验证包的哈希值（lockfile）
grep "checksum" Cargo.lock | head -5
grep "integrity" package-lock.json | head -5
```

### 修复建议
- 使用 `cargo deny` 配置可信依赖列表
- 审查所有新增依赖的来源
- 使用 GitHub Code Scanning 检测可疑依赖
- 启用 package-lock.json 的 SHA512 完整性检查

---

## 场景 4：构建时攻击（Build-Time Compromise）

### 前置条件
- CI/CD 环境
- Docker 构建流程

### 攻击目标
验证构建流程的安全性，防止构建时注入恶意代码

### 攻击步骤
1. 检查 Dockerfile 安全性：
   ```bash
   # 查找 Dockerfile 中的安全问题
   cd auth9-core
   cat Dockerfile
   
   # 检查是否运行为 root 用户
   grep "USER" Dockerfile
   
   # 检查基础镜像来源
   grep "FROM" Dockerfile
   ```

2. 检查构建脚本安全性：
   ```bash
   # 查找 build.rs 或自定义构建脚本
   find . -name "build.rs" -o -name "build.sh"
   
   # 检查是否执行外部命令
   grep -r "std::process::Command" auth9-core/
   ```

3. 验证构建产物的完整性：
   ```bash
   # 检查二进制文件签名（如果有）
   # 验证 Docker 镜像层完整性
   docker inspect auth9-core:latest | jq '.[0].RootFS.Layers'
   ```

### 预期安全行为
- Dockerfile 不运行为 root 用户（使用 USER 指令）
- 基础镜像来自官方仓库（如 rust:1.75-alpine）
- 无可疑的构建时网络请求
- 构建脚本不执行不受信任的外部命令
- 构建产物可重现（reproducible builds）

### 验证方法
```bash
# 扫描 Dockerfile 安全性
docker run --rm -v $(pwd):/project aquasec/trivy config /project/auth9-core/Dockerfile

# 检查镜像漏洞
docker run --rm -v /var/run/docker.sock:/var/run/docker.sock aquasec/trivy image auth9-core:latest

# 验证构建可重现性（两次构建结果一致）
cargo build --release
sha256sum target/release/auth9-core > hash1.txt
cargo clean
cargo build --release
sha256sum target/release/auth9-core > hash2.txt
diff hash1.txt hash2.txt  # 应该相同（Rust 默认支持）
```

### 修复建议
- 使用官方基础镜像（`rust:1.75-alpine`, `node:20-alpine`）
- 添加 USER 指令：`USER 1000:1000`
- 固定依赖版本（使用 lockfiles）
- 实施 SLSA Level 2+ 构建流程
- 使用 cosign 签名容器镜像

---

## 场景 5：容器逃逸与运行时安全

### 前置条件
- Auth9 部署在 Docker 容器中

### 攻击目标
验证容器配置安全，防止容器逃逸

### 攻击步骤
1. 检查容器权限配置：
   ```bash
   # 查看容器是否以特权模式运行
   docker inspect auth9-core | jq '.[0].HostConfig.Privileged'
   
   # 检查 capabilities
   docker inspect auth9-core | jq '.[0].HostConfig.CapAdd'
   ```

2. 检查敏感挂载点：
   ```bash
   # 检查是否挂载 Docker socket（危险）
   docker inspect auth9-core | jq '.[0].Mounts[] | select(.Source=="/var/run/docker.sock")'
   
   # 检查主机路径挂载
   docker inspect auth9-core | jq '.[0].Mounts[]'
   ```

3. 尝试容器内提权：
   ```bash
   # 进入容器
   docker exec -it auth9-core /bin/sh
   
   # 尝试执行特权操作（应失败）
   mount -t tmpfs tmpfs /mnt
   dmesg
   insmod /path/to/module.ko
   ```

4. 检查 seccomp 和 AppArmor 配置：
   ```bash
   docker inspect auth9-core | jq '.[0].HostConfig.SecurityOpt'
   ```

### 预期安全行为
- 容器不以特权模式运行（Privileged: false）
- 无不必要的 capabilities（如 SYS_ADMIN, NET_ADMIN）
- Docker socket 未挂载到容器内
- seccomp 和 AppArmor 已启用
- 容器内无法访问主机资源

### 验证方法
```bash
# 使用 Docker Bench Security 扫描
git clone https://github.com/docker/docker-bench-security.git
cd docker-bench-security
sudo sh docker-bench-security.sh

# 使用 Trivy 扫描容器配置
trivy config docker-compose.yml

# Kubernetes 环境：使用 kube-bench
kube-bench run --targets master,node
```

### 修复建议
- 使用非特权容器
- 启用 seccomp 默认配置：
  ```yaml
  security_opt:
    - no-new-privileges:true
    - seccomp:default
  ```
- 使用只读根文件系统（`read_only: true`）
- 限制容器 capabilities：
  ```yaml
  cap_drop:
    - ALL
  cap_add:
    - NET_BIND_SERVICE  # 仅需要的权限
  ```
- 在 Kubernetes 中使用 Pod Security Standards (PSS)

---

## 场景 6：GitHub Dependabot 警报审查

### 前置条件
- GitHub 仓库已启用 Dependabot alerts
- 安装 GitHub CLI (`gh`)
- 拥有仓库读取权限

### 攻击目标
验证 GitHub Dependabot 发现的已知漏洞已被及时治理，无遗留的 open 警报

### 攻击步骤
1. 拉取所有 Dependabot 警报：
   ```bash
   gh api repos/:owner/:repo/dependabot/alerts \
     --jq '.[] | {number, state, severity: .security_advisory.severity, package: .security_vulnerability.package.name, ecosystem: .security_vulnerability.package.ecosystem, summary: .security_advisory.summary, patched: .security_vulnerability.first_patched_version.identifier}'
   ```

> **注意**: Dependabot 状态不是在 push 后瞬时更新。锁文件升级刚合并或刚 push 到默认分支时，
> GitHub 依赖图可能还在重扫，短时间内仍会显示旧的 `open` 告警。只有在实时查询结果里
> `state == "open"` 仍然存在时，才应判定场景失败；如果同一批告警已经转为 `fixed`，应视为通过而不是继续开票。

2. 筛选 open 状态的警报：
   ```bash
   gh api repos/:owner/:repo/dependabot/alerts \
     --jq '[.[] | select(.state == "open")] | length'
   ```

3. 对每个 open 警报执行影响分析：
   ```bash
   # Rust: 检查当前锁定版本与修复版本
   grep -A2 '^name = "<package>"' auth9-core/Cargo.lock
   cargo tree -i <package> --depth 2

   # npm: 检查当前锁定版本
   npm ls <package>
   grep -A1 '"node_modules/<package>"' auth9-portal/package-lock.json
   ```

4. 评估每个警报的实际影响：
   - 是否为直接依赖还是传递依赖
   - 漏洞触发条件是否在 auth9 使用场景中存在
   - 修复版本是否与当前依赖树兼容

### 预期安全行为
- 无 HIGH/CRITICAL 级别的 open 警报（已评估并记录 FR 的除外）
- MEDIUM 级别警报应在 30 天内评估并处理
- 已修复（fixed/dismissed）的警报有对应的升级记录
- 所有 open 警报均已评估影响并记录处理计划

> **已知例外**: path-to-regexp ReDoS (HIGH) — 传递依赖 via express@4，npm overrides 不兼容。实际可利用性 Low（Portal 不暴露用户可控的多参数路由）。详见 `docs/feature_request/supply_chain_path_to_regexp.md`。

### 验证方法
```bash
# 获取所有 open 警报并按严重性分组
gh api repos/:owner/:repo/dependabot/alerts \
  --jq '[.[] | select(.state == "open")] | group_by(.security_advisory.severity) | map({severity: .[0].security_advisory.severity, count: length})'

# 检查 HIGH/CRITICAL 是否为 0
gh api repos/:owner/:repo/dependabot/alerts \
  --jq '[.[] | select(.state == "open" and (.security_advisory.severity == "high" or .security_advisory.severity == "critical"))] | length'
# 预期: 0

# 验证已修复警报数量
gh api repos/:owner/:repo/dependabot/alerts \
  --jq '[.[] | select(.state == "fixed")] | length'
```

### 修复建议
- 启用 Dependabot security updates（自动创建 PR）
- 对 Rust 依赖使用 `cargo update -p <package>` 升级特定包
- 对 npm 依赖使用 `npm audit fix` 或手动升级
- 对 pnpm workspace 使用 `pnpm.overrides` 强制升级传递依赖
- 设置 CI 门禁：当存在 HIGH/CRITICAL 警报时阻止合并
- 定期（每周）审查 Dependabot 警报状态

### 常见误报排查

| 症状 | 根因 | 处理方式 |
|------|------|----------|
| 本地锁文件已升级，但票据仍显示旧的 HIGH open alerts | GitHub 依赖图/Dependabot 状态刷新滞后 | 重新执行 `gh api` 查询，确认告警是否已转为 `fixed`，不要仅根据历史截图或早先输出判定失败 |

---

## 自动化检测脚本

```bash
#!/bin/bash
# supply-chain-security-check.sh

set -e

echo "=== Auth9 Supply Chain Security Check ==="

# 1. GitHub Dependabot Alerts
echo "\n[1/6] Checking GitHub Dependabot alerts..."
OPEN_CRITICAL=$(gh api repos/:owner/:repo/dependabot/alerts \
  --jq '[.[] | select(.state == "open" and (.security_advisory.severity == "high" or .security_advisory.severity == "critical"))] | length')
echo "Open HIGH/CRITICAL alerts: $OPEN_CRITICAL"
if [ "$OPEN_CRITICAL" -gt 0 ]; then
  echo "⚠️  Dependabot HIGH/CRITICAL alerts found:"
  gh api repos/:owner/:repo/dependabot/alerts \
    --jq '.[] | select(.state == "open" and (.security_advisory.severity == "high" or .security_advisory.severity == "critical")) | "#\(.number) [\(.security_advisory.severity)] \(.security_vulnerability.package.name): \(.security_advisory.summary)"'
fi

# 2. Rust Dependencies
echo "\n[2/6] Checking Rust dependencies..."
cd auth9-core
cargo audit --deny warnings || echo "⚠️  Rust vulnerabilities found"

# 3. Node.js Dependencies
echo "\n[3/6] Checking Node.js dependencies..."
cd ../auth9-portal
npm audit --audit-level=high || echo "⚠️  npm vulnerabilities found"

# 4. Container Security
echo "\n[4/6] Scanning Docker images..."
docker run --rm -v /var/run/docker.sock:/var/run/docker.sock \
  aquasec/trivy image auth9-core:latest --severity HIGH,CRITICAL

# 5. Dockerfile Security
echo "\n[5/6] Checking Dockerfile best practices..."
docker run --rm -i hadolint/hadolint < auth9-core/Dockerfile

# 6. SBOM Generation (Software Bill of Materials)
echo "\n[6/6] Generating SBOM..."
cd ../auth9-core
cargo install cargo-sbom
cargo sbom --output-format json > sbom-rust.json
echo "✅ SBOM generated: sbom-rust.json"

echo "\n=== Security Check Complete ==="
```

---

## 参考资料

- [OWASP Software Component Verification Standard](https://owasp.org/www-project-software-component-verification-standard/)
- [SLSA Framework](https://slsa.dev/)
- [Cargo Security Best Practices](https://doc.rust-lang.org/cargo/reference/security.html)
- [npm Security Best Practices](https://docs.npmjs.com/security-best-practices)
- [CIS Docker Benchmark](https://www.cisecurity.org/benchmark/docker)

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-ADV-01  
**适用控制**: V13.1,V13.2,V15.1,V15.2  
**关联任务**: Backlog #14, #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 6

### 执行清单
- [ ] M-ADV-01-C01 | 控制: V13.1 | 任务: #14, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-ADV-01-C02 | 控制: V13.2 | 任务: #14, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-ADV-01-C03 | 控制: V15.1 | 任务: #14, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-ADV-01-C04 | 控制: V15.2 | 任务: #14, #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
