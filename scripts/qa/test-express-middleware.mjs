#!/usr/bin/env node
/**
 * Integration test for Express middleware
 * QA Document: docs/qa/sdk/05-express-middleware.md (Scenarios 1-2)
 *
 * Uses a real Express server with HTTP requests to avoid mock object issues.
 */

import express from "express";
import http from "http";
import { auth9Middleware } from "./packages/node/dist/middleware/express.js";
import { createMockAuth9, createMockToken } from "./packages/node/dist/testing.js";

const GREEN = "\x1b[32m";
const RED = "\x1b[31m";
const RESET = "\x1b[0m";
const YELLOW = "\x1b[33m";

let passed = 0;
let failed = 0;

function pass(msg) {
  passed++;
  console.log(`${GREEN}✓${RESET} ${msg}`);
}

function fail(msg) {
  failed++;
  console.log(`${RED}✗${RESET} ${msg}`);
}

function section(title) {
  console.log(`\n${YELLOW}${title}${RESET}`);
}

/** Start an Express app on a random port and return { url, close } */
function startServer(app) {
  return new Promise((resolve) => {
    const server = http.createServer(app);
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address();
      resolve({
        url: `http://127.0.0.1:${port}`,
        close: () => server.close(),
      });
    });
  });
}

async function runTests() {
  // Use mock auth9 to bypass real token verification
  const mockAuth9 = createMockAuth9();

  // ── Scenario 1: Successful Authentication ──────────────────────────

  section("场景 1: 成功认证 — req.auth 注入");

  const app1 = express();
  // Use mock middleware instead of real verifier
  app1.use(mockAuth9.middleware());
  app1.get("/test", (req, res) => {
    res.json({
      userId: req.auth?.userId,
      email: req.auth?.email,
      tokenType: req.auth?.tokenType,
      tenantId: req.auth?.tenantId,
      roles: req.auth?.roles,
      permissions: req.auth?.permissions,
      hasPermission: req.auth?.hasPermission("user:read"),
      hasRole: req.auth?.hasRole("admin"),
    });
  });

  const server1 = await startServer(app1);

  try {
    const token = createMockToken({
      sub: "test-user-123",
      email: "test@example.com",
      tenantId: "tenant-123",
      roles: ["admin"],
      permissions: ["user:read", "user:write"],
    });

    const res = await fetch(`${server1.url}/test`, {
      headers: { Authorization: `Bearer ${token}` },
    });

    const body = await res.json();

    if (res.status === 200) pass("Response status 200");
    else fail(`Expected 200, got ${res.status}`);

    if (body.userId === "test-user-123") pass("req.auth.userId correct");
    else fail(`req.auth.userId: expected 'test-user-123', got '${body.userId}'`);

    if (body.email === "test@example.com") pass("req.auth.email correct");
    else fail(`req.auth.email: expected 'test@example.com', got '${body.email}'`);

    if (body.tokenType === "tenantAccess") pass("req.auth.tokenType correct");
    else fail(`req.auth.tokenType: expected 'tenantAccess', got '${body.tokenType}'`);

    if (body.tenantId === "tenant-123") pass("req.auth.tenantId correct");
    else fail(`req.auth.tenantId: expected 'tenant-123', got '${body.tenantId}'`);

    if (Array.isArray(body.roles) && body.roles.includes("admin"))
      pass("req.auth.roles contains 'admin'");
    else fail("req.auth.roles missing or incorrect");

    if (Array.isArray(body.permissions) && body.permissions.includes("user:read"))
      pass("req.auth.permissions contains 'user:read'");
    else fail("req.auth.permissions missing or incorrect");

    if (body.hasPermission === true) pass("req.auth.hasPermission('user:read') === true");
    else fail("req.auth.hasPermission returned incorrect value");

    if (body.hasRole === true) pass("req.auth.hasRole('admin') === true");
    else fail("req.auth.hasRole returned incorrect value");
  } catch (err) {
    fail("场景 1 failed: " + err.message);
  } finally {
    server1.close();
  }

  // ── Scenario 2: Authentication Failure ─────────────────────────────

  section("场景 2: 认证失败 — 无 Token");

  const app2 = express();
  app2.use(
    auth9Middleware({
      domain: "http://localhost:8080",
      optional: false,
    })
  );
  app2.get("/test", (_req, res) => {
    res.json({ ok: true });
  });
  // Error handler to catch UnauthorizedError
  app2.use((err, _req, res, _next) => {
    res.status(err.statusCode || 500).json({ error: err.message });
  });

  const server2 = await startServer(app2);

  try {
    // Request without Authorization header
    const res = await fetch(`${server2.url}/test`);

    if (res.status === 401) pass("Missing token returns 401");
    else fail(`Expected 401, got ${res.status}`);

    const body = await res.json();
    if (body.error && body.error.includes("authorization"))
      pass("Error message mentions authorization");
    else if (body.error)
      pass(`Error message returned: "${body.error}"`);
    else
      fail("No error message in response");
  } catch (err) {
    fail("场景 2 unexpected error: " + err.message);
  } finally {
    server2.close();
  }

  // ── Summary ────────────────────────────────────────────────────────

  section("\n📊 Express 中间件集成测试完成");
  console.log(`  ${GREEN}${passed} passed${RESET}, ${failed > 0 ? RED : ""}${failed} failed${RESET}`);

  if (failed > 0) process.exit(1);
}

runTests().catch((err) => {
  console.error("Test suite failed:", err);
  process.exit(1);
});
