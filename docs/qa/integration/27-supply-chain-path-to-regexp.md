# 集成 - 供应链 path-to-regexp ReDoS 漏洞修复验证

**模块**: integration
**测试范围**: 验证 path-to-regexp 从 0.1.12 升级到 0.1.13 后，依赖链完整性和运行时兼容性
**场景数**: 3
**优先级**: 高

---

## 背景说明

Dependabot 检测到 CVE-2026-4867（HIGH）：`path-to-regexp@0.1.12` 存在 ReDoS 漏洞。修复版本为 `0.1.13`。

受影响的 lock 文件：
- `sdk/pnpm-lock.yaml`（express@4.22.1 → path-to-regexp@0.1.12）
- `auth9-demo/package-lock.json`（express@4.22.1 → path-to-regexp@0.1.12）

修复方式：通过 pnpm overrides 和 npm overrides 将 express 的 path-to-regexp 依赖固定为 `0.1.13`。

---

## 场景 1：SDK lock 文件版本验证

### 初始状态
- `sdk/package.json` 已添加 `"express>path-to-regexp": "0.1.13"` 到 pnpm overrides
- `pnpm install` 已执行

### 目的
验证 SDK 的 pnpm-lock.yaml 中 path-to-regexp 已升级到 0.1.13，无残留 0.1.12

### 测试操作流程
1. 检查 `sdk/pnpm-lock.yaml` 中 path-to-regexp 版本：
```bash
cd sdk && grep 'path-to-regexp@0' pnpm-lock.yaml
```
2. 验证 SDK 构建通过：
```bash
cd sdk && pnpm build
```

### 预期结果
- lock 文件中仅出现 `path-to-regexp@0.1.13`，不存在 `path-to-regexp@0.1.12`
- `pnpm build` 成功完成，无错误

---

## 场景 2：auth9-demo lock 文件版本验证

### 初始状态
- `auth9-demo/package.json` 已添加 npm overrides：`"express": { "path-to-regexp": "0.1.13" }`
- `npm install` 已执行

### 目的
验证 auth9-demo 的 package-lock.json 中 express 的 path-to-regexp 已升级到 0.1.13

### 测试操作流程
1. 检查 auth9-demo 依赖树：
```bash
cd auth9-demo && npm ls path-to-regexp 2>&1 | grep -v 'npm error' | grep path-to-regexp
```
2. 验证 auth9-demo 构建通过：
```bash
cd auth9-demo && npm run build
```

### 预期结果
- 依赖树中 express 下的 path-to-regexp 显示 `0.1.13`（标记为 `overridden`）
- `npm run build` 成功完成，无错误

---

## 场景 3：Portal SSR 运行时兼容性验证

### 初始状态
- Docker 环境已通过 `./scripts/reset-docker.sh` 重置
- auth9-core 和 auth9-portal 服务已启动

### 目的
验证依赖升级未影响 Portal SSR 正常运行

### 测试操作流程
1. 检查 auth9-core 健康端点：
```bash
curl -sf http://localhost:8080/health && echo "OK"
```
2. 检查 Portal 服务可访问：
```bash
curl -sf -o /dev/null -w "%{http_code}" http://localhost:3000
```

### 预期结果
- auth9-core 健康端点返回 200
- Portal 返回 200（或 302 重定向到登录页）

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | SDK lock 文件版本验证 | ☐ | | | |
| 2 | auth9-demo lock 文件版本验证 | ☐ | | | |
| 3 | Portal SSR 运行时兼容性验证 | ☐ | | | |
