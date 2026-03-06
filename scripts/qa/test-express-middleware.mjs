#!/usr/bin/env node
/**
 * Integration test for Express middleware
 * QA Document: docs/qa/sdk/05-express-middleware.md (Scenarios 1-5)
 *
 * Uses a real Express server with HTTP requests to avoid mock object issues.
 */

import express from "express";
import http from "http";
import { auth9Middleware, requirePermission, requireRole } from "./packages/node/dist/middleware/express.js";
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

/** Create a standard error handler */
function errorHandler(err, req, res, next) {
  const status = err.statusCode || err.status || 500;
  res.status(status).json({ error: err.message, code: err.code });
}

async function runTests() {
  const mockAuth9 = createMockAuth9();

  // ── Scenario 1: Successful Authentication ──────────────────────────

  section("场景 1: 成功认证 — req.auth 注入");

  const app1 = express();
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

  section("场景 2: 认证失败 — 无 Token / 无效 Token");

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
  app2.use(errorHandler);

  const server2 = await startServer(app2);

  try {
    // Test 2a: No token
    let res = await fetch(`${server2.url}/test`);
    let body = await res.json();

    if (res.status === 401) pass("Missing token returns 401");
    else fail(`Expected 401, got ${res.status}`);

    if (body.error && body.error.toLowerCase().includes("token"))
      pass("Error message mentions 'token'");
    else fail(`Unexpected error message: "${body.error}"`);

    // Test 2b: Invalid token
    res = await fetch(`${server2.url}/test`, {
      headers: { Authorization: "Bearer invalid-token" },
    });
    body = await res.json();

    if (res.status === 401) pass("Invalid token returns 401");
    else fail(`Expected 401 for invalid token, got ${res.status}`);

    // Test 2c: Basic Auth (wrong scheme)
    res = await fetch(`${server2.url}/test`, {
      headers: { Authorization: "Basic dXNlcjpwYXNz" },
    });
    if (res.status === 401) pass("Basic Auth returns 401 (not Bearer)");
    else fail(`Expected 401 for Basic Auth, got ${res.status}`);
  } catch (err) {
    fail("场景 2 unexpected error: " + err.message);
  } finally {
    server2.close();
  }

  // ── Scenario 3: Optional Mode ─────────────────────────────────────
  // Note: Testing optional mode requires either a real valid token or mock behavior.
  // We'll test different aspects separately.

  section("场景 3: Optional 模式");

  // Test 3a & 3c: Real middleware with optional mode (no token / invalid token)
  const app3a = express();
  app3a.use(
    auth9Middleware({
      domain: "http://localhost:8080",
      optional: true,
    })
  );
  app3a.get("/public-or-private", (req, res) => {
    if (req.auth) {
      res.json({ message: "Authenticated", user: req.auth.email });
    } else {
      res.json({ message: "Anonymous" });
    }
  });
  app3a.use(errorHandler);

  const server3a = await startServer(app3a);

  try {
    // Test 3a: No token - should be allowed
    let res = await fetch(`${server3a.url}/public-or-private`);
    let body = await res.json();

    if (res.status === 200 && body.message === "Anonymous")
      pass("3a: No token returns 200 with Anonymous");
    else fail(`3a: Expected 200 Anonymous, got ${res.status}: ${body.message}`);

    // Test 3c: Invalid token - should be allowed (optional mode doesn't reject)
    res = await fetch(`${server3a.url}/public-or-private`, {
      headers: { Authorization: "Bearer invalid-token" },
    });
    body = await res.json();

    if (res.status === 200 && body.message === "Anonymous")
      pass("3c: Invalid token in optional mode returns Anonymous");
    else fail(`3c: Expected 200 Anonymous for invalid token, got ${res.status}: ${body.message}`);
  } catch (err) {
    fail("场景 3a/3c unexpected error: " + err.message);
  } finally {
    server3a.close();
  }

  // Test 3b: Mock middleware for valid token in optional mode
  const app3b = express();
  app3b.use(mockAuth9.middleware());
  app3b.get("/public-or-private", (req, res) => {
    if (req.auth) {
      res.json({ message: "Authenticated", user: req.auth.email });
    } else {
      res.json({ message: "Anonymous" });
    }
  });
  app3b.use(errorHandler);

  const server3b = await startServer(app3b);

  try {
    // Test 3b: Valid token - should be authenticated (mock always sets auth)
    const validToken = createMockToken({
      sub: "user-456",
      email: "valid@example.com",
      tenantId: "tenant-456",
      roles: ["user"],
      permissions: ["user:read"],
    });
    const res = await fetch(`${server3b.url}/public-or-private`, {
      headers: { Authorization: `Bearer ${validToken}` },
    });
    const body = await res.json();

    if (res.status === 200 && body.message === "Authenticated" && body.user === "valid@example.com")
      pass("3b: Valid token returns Authenticated");
    else fail(`3b: Expected Authenticated, got ${res.status}: ${JSON.stringify(body)}`);
  } catch (err) {
    fail("场景 3b unexpected error: " + err.message);
  } finally {
    server3b.close();
  }

  // ── Scenario 4: requirePermission ─────────────────────────────────

  section("场景 4: requirePermission 权限控制");

  const app4 = express();
  app4.use(mockAuth9.middleware());
  app4.get("/users", requirePermission("user:read"), (_req, res) => {
    res.json({ route: "GET /users", ok: true });
  });
  app4.post("/users", requirePermission(["user:read", "user:write"]), (_req, res) => {
    res.json({ route: "POST /users", ok: true });
  });
  app4.delete("/users/:id", requirePermission("user:delete"), (_req, res) => {
    res.json({ route: "DELETE /users/:id", ok: true });
  });
  app4.patch("/users/:id", requirePermission(["user:write", "user:admin"], { mode: "any" }), (_req, res) => {
    res.json({ route: "PATCH /users/:id", ok: true });
  });
  app4.use(errorHandler);

  const server4 = await startServer(app4);

  try {
    // User has: user:read, user:write
    const tokenWithPerms = createMockToken({
      sub: "perm-user",
      email: "perm@example.com",
      tenantId: "tenant-perm",
      roles: ["editor"],
      permissions: ["user:read", "user:write"],
    });
    const authHeader = { Authorization: `Bearer ${tokenWithPerms}` };

    // Test 4a: GET /users - has user:read
    let res = await fetch(`${server4.url}/users`, { headers: authHeader });
    if (res.status === 200) pass("GET /users: 200 (has user:read)");
    else fail(`GET /users expected 200, got ${res.status}`);

    // Test 4b: POST /users - has user:read + user:write (all mode)
    res = await fetch(`${server4.url}/users`, { method: "POST", headers: authHeader });
    if (res.status === 200) pass("POST /users: 200 (has user:read + user:write)");
    else fail(`POST /users expected 200, got ${res.status}`);

    // Test 4c: DELETE /users/1 - missing user:delete
    res = await fetch(`${server4.url}/users/1`, { method: "DELETE", headers: authHeader });
    const body = await res.json();
    if (res.status === 403 && body.error && body.error.includes("user:delete"))
      pass("DELETE /users/:id: 403 (missing user:delete)");
    else fail(`DELETE expected 403, got ${res.status}: ${body.error}`);

    // Test 4d: PATCH /users/1 - has user:write (any mode)
    res = await fetch(`${server4.url}/users/1`, { method: "PATCH", headers: authHeader });
    if (res.status === 200) pass("PATCH /users/:id: 200 (any mode, user:write matches)");
    else fail(`PATCH expected 200, got ${res.status}`);
  } catch (err) {
    fail("场景 4 unexpected error: " + err.message);
  } finally {
    server4.close();
  }

  // ── Scenario 5: requireRole and AuthInfo helpers ───────────────────

  section("场景 5: requireRole 角色控制与 AuthInfo helpers");

  const app5 = express();
  app5.use(mockAuth9.middleware());
  app5.get("/admin", requireRole("admin"), (_req, res) => {
    res.json({ route: "/admin", ok: true });
  });
  app5.get("/superadmin", requireRole("superadmin"), (_req, res) => {
    res.json({ route: "/superadmin", ok: true });
  });
  app5.get("/any-admin", requireRole(["admin", "superadmin"], { mode: "any" }), (_req, res) => {
    res.json({ route: "/any-admin", ok: true });
  });
  app5.get("/check-helpers", (req, res) => {
    const auth = req.auth;
    res.json({
      hasReadPerm: auth ? auth.hasPermission("user:read") : false,
      hasDeletePerm: auth ? auth.hasPermission("user:delete") : false,
      isAdmin: auth ? auth.hasRole("admin") : false,
      isSuperAdmin: auth ? auth.hasRole("superadmin") : false,
      hasAnyWritePerm: auth ? auth.hasAnyPermission(["user:write", "user:admin"]) : false,
      hasAllPerms: auth ? auth.hasAllPermissions(["user:read", "user:write"]) : false,
      hasAllPermsIncDelete: auth ? auth.hasAllPermissions(["user:read", "user:delete"]) : false,
    });
  });
  app5.use(errorHandler);

  const server5 = await startServer(app5);

  try {
    // User has: roles: ["admin", "user"], permissions: ["user:read", "user:write"]
    const tokenWithRoles = createMockToken({
      sub: "role-user",
      email: "role@example.com",
      tenantId: "tenant-role",
      roles: ["admin", "user"],
      permissions: ["user:read", "user:write"],
    });
    const authHeader = { Authorization: `Bearer ${tokenWithRoles}` };

    // Test 5a: /admin - has admin role
    let res = await fetch(`${server5.url}/admin`, { headers: authHeader });
    if (res.status === 200) pass("GET /admin: 200 (has admin role)");
    else fail(`GET /admin expected 200, got ${res.status}`);

    // Test 5b: /superadmin - missing superadmin role
    res = await fetch(`${server5.url}/superadmin`, { headers: authHeader });
    const body = await res.json();
    if (res.status === 403 && body.error && body.error.includes("superadmin"))
      pass("GET /superadmin: 403 (missing superadmin role)");
    else fail(`GET /superadmin expected 403, got ${res.status}: ${body.error}`);

    // Test 5c: /any-admin - any mode, admin matches
    res = await fetch(`${server5.url}/any-admin`, { headers: authHeader });
    if (res.status === 200) pass("GET /any-admin: 200 (any mode, admin matches)");
    else fail(`GET /any-admin expected 200, got ${res.status}`);

    // Test 5d: AuthInfo helper methods
    res = await fetch(`${server5.url}/check-helpers`, { headers: authHeader });
    const helpers = await res.json();

    if (helpers.hasReadPerm === true) pass("hasPermission('user:read') === true");
    else fail(`hasPermission expected true, got ${helpers.hasReadPerm}`);

    if (helpers.hasDeletePerm === false) pass("hasPermission('user:delete') === false");
    else fail(`hasPermission expected false, got ${helpers.hasDeletePerm}`);

    if (helpers.isAdmin === true) pass("hasRole('admin') === true");
    else fail(`hasRole expected true, got ${helpers.isAdmin}`);

    if (helpers.isSuperAdmin === false) pass("hasRole('superadmin') === false");
    else fail(`hasRole expected false, got ${helpers.isSuperAdmin}`);

    if (helpers.hasAnyWritePerm === true) pass("hasAnyPermission(['user:write', 'user:admin']) === true");
    else fail(`hasAnyPermission expected true, got ${helpers.hasAnyWritePerm}`);

    if (helpers.hasAllPerms === true) pass("hasAllPermissions(['user:read', 'user:write']) === true");
    else fail(`hasAllPermissions expected true, got ${helpers.hasAllPerms}`);

    if (helpers.hasAllPermsIncDelete === false) pass("hasAllPermissions(['user:read', 'user:delete']) === false");
    else fail(`hasAllPermissions expected false, got ${helpers.hasAllPermsIncDelete}`);
  } catch (err) {
    fail("场景 5 unexpected error: " + err.message);
  } finally {
    server5.close();
  }

  // ── Summary ───────────────────────────────────────────────────────

  section("\n📊 Express 中间件集成测试完成");
  console.log(`  ${GREEN}${passed} passed${RESET}, ${failed > 0 ? RED : ""}${failed} failed${RESET}`);

  if (failed > 0) process.exit(1);
}

runTests().catch((err) => {
  console.error("Test suite failed:", err);
  process.exit(1);
});
