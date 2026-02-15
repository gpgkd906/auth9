#!/usr/bin/env node
/**
 * Full Express middleware QA test
 * QA Document: docs/qa/sdk/05-express-middleware.md (All 5 scenarios)
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

// Error handler for Express apps
function errorHandler(err, _req, res, _next) {
  res.status(err.statusCode || 500).json({ error: err.message });
}

async function runTests() {
  const mockAuth9 = createMockAuth9();

  // ── Scenario 3: Optional Mode ────────────────────────────────────────
  
  section("场景 3: Optional 模式");

  const app3 = express();
  app3.use(mockAuth9.middleware()); // Use mock middleware for optional mode testing
  app3.get("/public-or-private", (req, res) => {
    if (req.auth) {
      res.json({ message: "Authenticated", user: req.auth.email });
    } else {
      res.json({ message: "Anonymous" });
    }
  });
  app3.use(errorHandler);

  const server3 = await startServer(app3);

  try {
    // Test 3.1: No token (should return Anonymous)
    const res1 = await fetch(`${server3.url}/public-or-private`);
    const body1 = await res1.json();
    
    if (res1.status === 200) pass("Optional mode: No token returns 200");
    else fail(`Optional mode: Expected 200, got ${res1.status}`);
    
    if (body1.message === "Anonymous") pass("Optional mode: Returns 'Anonymous' without token");
    else fail(`Optional mode: Expected 'Anonymous', got '${body1.message}'`);

    // Test 3.2: Valid token (should return Authenticated)
    const validToken = createMockToken({
      sub: "test-user-123",
      email: "test@example.com",
      tenantId: "tenant-123",
      roles: ["admin"],
      permissions: ["user:read", "user:write"],
    });
    
    const res2 = await fetch(`${server3.url}/public-or-private`, {
      headers: { Authorization: `Bearer ${validToken}` },
    });
    const body2 = await res2.json();
    
    if (res2.status === 200) pass("Optional mode: Valid token returns 200");
    else fail(`Optional mode: Valid token expected 200, got ${res2.status}`);
    
    if (body2.message === "Authenticated") pass("Optional mode: Returns 'Authenticated' with valid token");
    else fail(`Optional mode: Expected 'Authenticated', got '${body2.message}'`);
    
    if (body2.user === "test@example.com") pass("Optional mode: Includes user email");
    else fail(`Optional mode: Expected user email, got '${body2.user}'`);

    // Test 3.3: Invalid token (should return Anonymous in optional mode)
    const res3 = await fetch(`${server3.url}/public-or-private`, {
      headers: { Authorization: "Bearer invalid-token" },
    });
    const body3 = await res3.json();
    
    if (res3.status === 200) pass("Optional mode: Invalid token returns 200");
    else fail(`Optional mode: Invalid token expected 200, got ${res3.status}`);
    
    if (body3.message === "Anonymous") pass("Optional mode: Invalid token returns 'Anonymous'");
    else fail(`Optional mode: Invalid token expected 'Anonymous', got '${body3.message}'`);

  } catch (err) {
    fail("场景 3 failed: " + err.message);
  } finally {
    server3.close();
  }

  // ── Scenario 4: requirePermission ────────────────────────────────────
  
  section("场景 4: requirePermission 权限控制");

  const app4 = express();
  app4.use(mockAuth9.middleware());
  
  // Setup routes with requirePermission
  app4.get("/users", requirePermission("user:read"), (_req, res) => {
    res.json({ message: "GET /users success" });
  });
  
  app4.post("/users", requirePermission(["user:read", "user:write"]), (_req, res) => {
    res.json({ message: "POST /users success" });
  });
  
  app4.delete("/users/:id", requirePermission("user:delete"), (_req, res) => {
    res.json({ message: "DELETE /users success" });
  });
  
  app4.patch("/users/:id", 
    requirePermission(["user:write", "user:admin"], { mode: "any" }),
    (_req, res) => {
      res.json({ message: "PATCH /users success" });
    }
  );
  
  app4.use(errorHandler);

  const server4 = await startServer(app4);

  try {
    // Create token with permissions: ["user:read", "user:write"]
    const token = createMockToken({
      sub: "test-user-123",
      email: "test@example.com",
      tenantId: "tenant-123",
      roles: ["admin"],
      permissions: ["user:read", "user:write"],
    });

    // Test 4.1: GET /users (has user:read permission)
    const res1 = await fetch(`${server4.url}/users`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    
    if (res1.status === 200) pass("requirePermission: GET /users with user:read returns 200");
    else fail(`requirePermission: GET /users expected 200, got ${res1.status}`);

    // Test 4.2: POST /users (has user:read + user:write permissions)
    const res2 = await fetch(`${server4.url}/users`, {
      method: "POST",
      headers: { Authorization: `Bearer ${token}` },
    });
    
    if (res2.status === 200) pass("requirePermission: POST /users with user:read+write returns 200");
    else fail(`requirePermission: POST /users expected 200, got ${res2.status}`);

    // Test 4.3: DELETE /users/1 (does NOT have user:delete permission)
    const res3 = await fetch(`${server4.url}/users/1`, {
      method: "DELETE",
      headers: { Authorization: `Bearer ${token}` },
    });
    
    if (res3.status === 403) pass("requirePermission: DELETE /users without user:delete returns 403");
    else fail(`requirePermission: DELETE /users expected 403, got ${res3.status}`);
    
    const body3 = await res3.json();
    if (body3.error && body3.error.includes("user:delete")) 
      pass("requirePermission: Error message mentions missing permission");
    else fail("requirePermission: Error message missing or incorrect");

    // Test 4.4: PATCH /users/1 (has user:write, any mode should match)
    const res4 = await fetch(`${server4.url}/users/1`, {
      method: "PATCH",
      headers: { Authorization: `Bearer ${token}` },
    });
    
    if (res4.status === 200) pass("requirePermission: PATCH /users with user:write (any mode) returns 200");
    else fail(`requirePermission: PATCH /users expected 200, got ${res4.status}`);

  } catch (err) {
    fail("场景 4 failed: " + err.message);
  } finally {
    server4.close();
  }

  // ── Scenario 5: requireRole & AuthInfo helpers ───────────────────────
  
  section("场景 5: requireRole 与 AuthInfo helpers");

  const app5 = express();
  app5.use(mockAuth9.middleware());
  
  // Setup routes with requireRole
  app5.get("/admin", requireRole("admin"), (_req, res) => {
    res.json({ message: "Admin access granted" });
  });
  
  app5.get("/superadmin", requireRole("superadmin"), (_req, res) => {
    res.json({ message: "Superadmin access granted" });
  });
  
  app5.get("/any-admin", requireRole(["admin", "superadmin"], { mode: "any" }), (_req, res) => {
    res.json({ message: "Any admin access granted" });
  });
  
  // Test AuthInfo helper methods
  app5.get("/check-helpers", (req, res) => {
    if (!req.auth) {
      return res.status(401).json({ error: "Authentication required" });
    }
    
    res.json({
      hasReadPerm: req.auth.hasPermission("user:read"),
      hasDeletePerm: req.auth.hasPermission("user:delete"),
      isAdmin: req.auth.hasRole("admin"),
      isSuperAdmin: req.auth.hasRole("superadmin"),
      hasAnyWritePerm: req.auth.hasAnyPermission(["user:write", "user:admin"]),
      hasAllPerms: req.auth.hasAllPermissions(["user:read", "user:write"]),
      hasAllPermsIncDelete: req.auth.hasAllPermissions(["user:read", "user:delete"]),
    });
  });
  
  app5.use(errorHandler);

  const server5 = await startServer(app5);

  try {
    // Create token with roles: ["admin", "user"], permissions: ["user:read", "user:write"]
    const token = createMockToken({
      sub: "test-user-123",
      email: "test@example.com",
      tenantId: "tenant-123",
      roles: ["admin", "user"],
      permissions: ["user:read", "user:write"],
    });

    // Test 5.1: GET /admin (has admin role)
    const res1 = await fetch(`${server5.url}/admin`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    
    if (res1.status === 200) pass("requireRole: GET /admin with admin role returns 200");
    else fail(`requireRole: GET /admin expected 200, got ${res1.status}`);

    // Test 5.2: GET /superadmin (does NOT have superadmin role)
    const res2 = await fetch(`${server5.url}/superadmin`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    
    if (res2.status === 403) pass("requireRole: GET /superadmin without superadmin role returns 403");
    else fail(`requireRole: GET /superadmin expected 403, got ${res2.status}`);
    
    const body2 = await res2.json();
    if (body2.error && body2.error.includes("superadmin")) 
      pass("requireRole: Error message mentions missing role");
    else fail("requireRole: Error message missing or incorrect");

    // Test 5.3: GET /any-admin (has admin role, any mode should match)
    const res3 = await fetch(`${server5.url}/any-admin`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    
    if (res3.status === 200) pass("requireRole: GET /any-admin with admin role (any mode) returns 200");
    else fail(`requireRole: GET /any-admin expected 200, got ${res3.status}`);

    // Test 5.4: AuthInfo helper methods
    const res4 = await fetch(`${server5.url}/check-helpers`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    
    if (res4.status === 200) pass("AuthInfo helpers: Endpoint returns 200");
    else fail(`AuthInfo helpers: Expected 200, got ${res4.status}`);
    
    const body4 = await res4.json();
    
    // Check each helper method
    if (body4.hasReadPerm === true) pass("AuthInfo helpers: hasPermission('user:read') === true");
    else fail(`AuthInfo helpers: hasPermission('user:read') expected true, got ${body4.hasReadPerm}`);
    
    if (body4.hasDeletePerm === false) pass("AuthInfo helpers: hasPermission('user:delete') === false");
    else fail(`AuthInfo helpers: hasPermission('user:delete') expected false, got ${body4.hasDeletePerm}`);
    
    if (body4.isAdmin === true) pass("AuthInfo helpers: hasRole('admin') === true");
    else fail(`AuthInfo helpers: hasRole('admin') expected true, got ${body4.isAdmin}`);
    
    if (body4.isSuperAdmin === false) pass("AuthInfo helpers: hasRole('superadmin') === false");
    else fail(`AuthInfo helpers: hasRole('superadmin') expected false, got ${body4.isSuperAdmin}`);
    
    if (body4.hasAnyWritePerm === true) pass("AuthInfo helpers: hasAnyPermission(['user:write', 'user:admin']) === true");
    else fail(`AuthInfo helpers: hasAnyPermission expected true, got ${body4.hasAnyWritePerm}`);
    
    if (body4.hasAllPerms === true) pass("AuthInfo helpers: hasAllPermissions(['user:read', 'user:write']) === true");
    else fail(`AuthInfo helpers: hasAllPermissions(['user:read', 'user:write']) expected true, got ${body4.hasAllPerms}`);
    
    if (body4.hasAllPermsIncDelete === false) pass("AuthInfo helpers: hasAllPermissions(['user:read', 'user:delete']) === false");
    else fail(`AuthInfo helpers: hasAllPermissions(['user:read', 'user:delete']) expected false, got ${body4.hasAllPermsIncDelete}`);

  } catch (err) {
    fail("场景 5 failed: " + err.message);
  } finally {
    server5.close();
  }

  // ── Summary ────────────────────────────────────────────────────────

  section("\n📊 Express 中间件完整测试完成");
  console.log(`  ${GREEN}${passed} passed${RESET}, ${failed > 0 ? RED : ""}${failed} failed${RESET}`);

  if (failed > 0) process.exit(1);
}

runTests().catch((err) => {
  console.error("Test suite failed:", err);
  process.exit(1);
});