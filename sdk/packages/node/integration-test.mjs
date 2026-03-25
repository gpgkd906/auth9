#!/usr/bin/env node
import express from "express";
import { auth9Middleware, requirePermission, requireRole } from "./dist/middleware/express.js";

const PORT = 3001;
const DOMAIN = "http://localhost:8080";

const app = express();
app.use(express.json());

app.use(auth9Middleware({
  domain: DOMAIN,
  audience: "auth9-portal",
}));

app.get("/test", (req, res) => {
  res.json({
    userId: req.auth?.userId,
    email: req.auth?.email,
    tokenType: req.auth?.tokenType,
    tenantId: req.auth?.tenantId,
    roles: req.auth?.roles,
    permissions: req.auth?.permissions,
  });
});

const optionalApp = express();
optionalApp.use(express.json());
optionalApp.use(auth9Middleware({
  domain: DOMAIN,
  optional: true,
}));

optionalApp.get("/public-or-private", (req, res) => {
  if (req.auth) {
    res.json({ message: "Authenticated", user: req.auth.email });
  } else {
    res.json({ message: "Anonymous" });
  }
});

optionalApp.get("/check-helpers", (req, res) => {
  res.json({
    hasReadPerm: req.auth?.hasPermission("user:read"),
    hasDeletePerm: req.auth?.hasPermission("user:delete"),
    isAdmin: req.auth?.hasRole("admin"),
    isSuperAdmin: req.auth?.hasRole("superadmin"),
    hasAnyWritePerm: req.auth?.hasAnyPermission(["user:write", "user:admin"]),
    hasAllPerms: req.auth?.hasAllPermissions(["user:read", "user:write"]),
    hasAllPermsIncDelete: req.auth?.hasAllPermissions(["user:read", "user:delete"]),
  });
});

const permApp = express();
permApp.use(express.json());
permApp.use(auth9Middleware({
  domain: DOMAIN,
  audience: "auth9-portal",
}));

permApp.get("/users", requirePermission("user:read"), (req, res) => {
  res.json({ ok: true, path: "/users" });
});

permApp.post("/users", requirePermission(["user:read", "user:write"]), (req, res) => {
  res.json({ ok: true, path: "/users (POST)" });
});

permApp.delete("/users/:id", requirePermission("user:delete"), (req, res) => {
  res.json({ ok: true, path: "/users/:id (DELETE)" });
});

permApp.patch("/users/:id",
  requirePermission(["user:write", "user:admin"], { mode: "any" }),
  (req, res) => {
    res.json({ ok: true, path: "/users/:id (PATCH)" });
  }
);

const roleApp = express();
roleApp.use(express.json());
roleApp.use(auth9Middleware({
  domain: DOMAIN,
  audience: "auth9-portal",
}));

roleApp.get("/admin", requireRole("admin"), (req, res) => {
  res.json({ ok: true, path: "/admin" });
});

roleApp.get("/superadmin", requireRole("superadmin"), (req, res) => {
  res.json({ ok: true, path: "/superadmin" });
});

roleApp.get("/any-admin",
  requireRole(["admin", "superadmin"], { mode: "any" }),
  (req, res) => {
    res.json({ ok: true, path: "/any-admin" });
  }
);

function startServer(app, port) {
  return new Promise((resolve) => {
    const server = app.listen(port, () => {
      console.log(`Server started on port ${port}`);
      resolve(server);
    });
  });
}

async function runTests() {
  const server1 = await startServer(app, PORT);
  const server2 = await startServer(optionalApp, PORT + 1);
  const server3 = await startServer(permApp, PORT + 2);
  const server4 = await startServer(roleApp, PORT + 3);

  const results = [];
  
  const adminToken = await getAdminToken();
  
  console.log("\n=== SCENARIO 1: Successful Auth - req.auth injection ===");
  try {
    const res = await fetch(`http://localhost:${PORT}/test`, {
      headers: { Authorization: `Bearer ${adminToken}` }
    });
    const data = await res.json();
    console.log(`Status: ${res.status}`);
    console.log("Response:", JSON.stringify(data, null, 2));
    
    const pass = res.status === 200 && 
      data.userId && data.email && data.tokenType === "tenantAccess" &&
      data.tenantId && Array.isArray(data.roles) && Array.isArray(data.permissions);
    results.push({ scenario: 1, status: pass ? "PASS" : "FAIL", data });
    console.log(pass ? "✅ PASS" : "❌ FAIL");
  } catch (e) {
    results.push({ scenario: 1, status: "FAIL", error: e.message });
    console.log("❌ FAIL:", e.message);
  }

  console.log("\n=== SCENARIO 2: Auth Failure - No Token / Invalid Token ===");
  
  // No token
  try {
    const res = await fetch(`http://localhost:${PORT}/test`);
    const data = await res.json();
    console.log(`No Token - Status: ${res.status}, Error: ${data.message}`);
    const pass1 = res.status === 401 && data.message === "Missing authorization token";
    results.push({ scenario: "2a", status: pass1 ? "PASS" : "FAIL" });
    console.log(pass1 ? "✅ PASS" : "❌ FAIL");
  } catch (e) {
    console.log("❌ FAIL:", e.message);
  }

  // Invalid token
  try {
    const res = await fetch(`http://localhost:${PORT}/test`, {
      headers: { Authorization: "Bearer invalid-token" }
    });
    const data = await res.json();
    console.log(`Invalid Token - Status: ${res.status}, Error: ${data.message}`);
    const pass2 = res.status === 401 && data.message === "Invalid or expired token";
    results.push({ scenario: "2b", status: pass2 ? "PASS" : "FAIL" });
    console.log(pass2 ? "✅ PASS" : "❌ FAIL");
  } catch (e) {
    console.log("❌ FAIL:", e.message);
  }

  // Basic auth
  try {
    const res = await fetch(`http://localhost:${PORT}/test`, {
      headers: { Authorization: "Basic dXNlcjpwYXNz" }
    });
    const data = await res.json();
    console.log(`Basic Auth - Status: ${res.status}, Error: ${data.message}`);
    const pass3 = res.status === 401;
    results.push({ scenario: "2c", status: pass3 ? "PASS" : "FAIL" });
    console.log(pass3 ? "✅ PASS" : "❌ FAIL");
  } catch (e) {
    console.log("❌ FAIL:", e.message);
  }

  console.log("\n=== SCENARIO 3: Optional Mode ===");
  
  // No token with optional
  try {
    const res = await fetch(`http://localhost:${PORT+1}/public-or-private`);
    const data = await res.json();
    console.log(`No Token - Status: ${res.status}, Body: ${JSON.stringify(data)}`);
    const pass1 = res.status === 200 && data.message === "Anonymous";
    results.push({ scenario: "3a", status: pass1 ? "PASS" : "FAIL" });
    console.log(pass1 ? "✅ PASS" : "❌ FAIL");
  } catch (e) {
    console.log("❌ FAIL:", e.message);
  }

  // Valid token with optional
  try {
    const res = await fetch(`http://localhost:${PORT+1}/public-or-private`, {
      headers: { Authorization: `Bearer ${adminToken}` }
    });
    const data = await res.json();
    console.log(`Valid Token - Status: ${res.status}, Body: ${JSON.stringify(data)}`);
    const pass2 = res.status === 200 && data.message === "Authenticated" && data.user;
    results.push({ scenario: "3b", status: pass2 ? "PASS" : "FAIL" });
    console.log(pass2 ? "✅ PASS" : "❌ FAIL");
  } catch (e) {
    console.log("❌ FAIL:", e.message);
  }

  // Invalid token with optional
  try {
    const res = await fetch(`http://localhost:${PORT+1}/public-or-private`, {
      headers: { Authorization: "Bearer invalid" }
    });
    const data = await res.json();
    console.log(`Invalid Token - Status: ${res.status}, Body: ${JSON.stringify(data)}`);
    const pass3 = res.status === 200 && data.message === "Anonymous";
    results.push({ scenario: "3c", status: pass3 ? "PASS" : "FAIL" });
    console.log(pass3 ? "✅ PASS" : "❌ FAIL");
  } catch (e) {
    console.log("❌ FAIL:", e.message);
  }

  console.log("\n=== SCENARIO 4: requirePermission ===");
  
  // GET /users with user:read
  try {
    const res = await fetch(`http://localhost:${PORT+2}/users`, {
      headers: { Authorization: `Bearer ${adminToken}` }
    });
    console.log(`GET /users - Status: ${res.status}`);
    const pass1 = res.status === 200;
    results.push({ scenario: "4a", status: pass1 ? "PASS" : "FAIL" });
    console.log(pass1 ? "✅ PASS" : "❌ FAIL");
  } catch (e) {
    console.log("❌ FAIL:", e.message);
  }

  // POST /users with user:read + user:write
  try {
    const res = await fetch(`http://localhost:${PORT+2}/users`, {
      method: "POST",
      headers: { Authorization: `Bearer ${adminToken}` }
    });
    console.log(`POST /users - Status: ${res.status}`);
    const pass2 = res.status === 200;
    results.push({ scenario: "4b", status: pass2 ? "PASS" : "FAIL" });
    console.log(pass2 ? "✅ PASS" : "❌ FAIL");
  } catch (e) {
    console.log("❌ FAIL:", e.message);
  }

  // DELETE /users/1 without user:delete permission
  try {
    const res = await fetch(`http://localhost:${PORT+2}/users/1`, {
      method: "DELETE",
      headers: { Authorization: `Bearer ${adminToken}` }
    });
    const data = await res.json();
    console.log(`DELETE /users/1 - Status: ${res.status}, Error: ${data.message}`);
    const pass3 = res.status === 403 && data.message?.includes("Missing required permission");
    results.push({ scenario: "4c", status: pass3 ? "PASS" : "FAIL" });
    console.log(pass3 ? "✅ PASS" : "❌ FAIL");
  } catch (e) {
    console.log("❌ FAIL:", e.message);
  }

  // PATCH /users/1 with any mode
  try {
    const res = await fetch(`http://localhost:${PORT+2}/users/1`, {
      method: "PATCH",
      headers: { Authorization: `Bearer ${adminToken}` }
    });
    console.log(`PATCH /users/1 - Status: ${res.status}`);
    const pass4 = res.status === 200;
    results.push({ scenario: "4d", status: pass4 ? "PASS" : "FAIL" });
    console.log(pass4 ? "✅ PASS" : "❌ FAIL");
  } catch (e) {
    console.log("❌ FAIL:", e.message);
  }

  console.log("\n=== SCENARIO 5: requireRole & AuthInfo Helpers ===");
  
  // GET /admin with admin role
  try {
    const res = await fetch(`http://localhost:${PORT+3}/admin`, {
      headers: { Authorization: `Bearer ${adminToken}` }
    });
    console.log(`GET /admin - Status: ${res.status}`);
    const pass1 = res.status === 200;
    results.push({ scenario: "5a", status: pass1 ? "PASS" : "FAIL" });
    console.log(pass1 ? "✅ PASS" : "❌ FAIL");
  } catch (e) {
    console.log("❌ FAIL:", e.message);
  }

  // GET /superadmin without superadmin role
  try {
    const res = await fetch(`http://localhost:${PORT+3}/superadmin`, {
      headers: { Authorization: `Bearer ${adminToken}` }
    });
    const data = await res.json();
    console.log(`GET /superadmin - Status: ${res.status}, Error: ${data.message}`);
    const pass2 = res.status === 403 && data.message?.includes("Missing required role");
    results.push({ scenario: "5b", status: pass2 ? "PASS" : "FAIL" });
    console.log(pass2 ? "✅ PASS" : "❌ FAIL");
  } catch (e) {
    console.log("❌ FAIL:", e.message);
  }

  // GET /any-admin with any mode
  try {
    const res = await fetch(`http://localhost:${PORT+3}/any-admin`, {
      headers: { Authorization: `Bearer ${adminToken}` }
    });
    console.log(`GET /any-admin - Status: ${res.status}`);
    const pass3 = res.status === 200;
    results.push({ scenario: "5c", status: pass3 ? "PASS" : "FAIL" });
    console.log(pass3 ? "✅ PASS" : "❌ FAIL");
  } catch (e) {
    console.log("❌ FAIL:", e.message);
  }

  // AuthInfo helpers
  try {
    const res = await fetch(`http://localhost:${PORT+1}/check-helpers`, {
      headers: { Authorization: `Bearer ${adminToken}` }
    });
    const data = await res.json();
    console.log(`AuthInfo helpers - Status: ${res.status}`);
    console.log("Helpers:", JSON.stringify(data, null, 2));
    
    const pass = res.status === 200;
    if (pass) {
      results.push({ scenario: "5d", status: "PASS", helpers: data });
      console.log("✅ PASS");
    } else {
      results.push({ scenario: "5d", status: "FAIL" });
      console.log("❌ FAIL");
    }
  } catch (e) {
    console.log("❌ FAIL:", e.message);
  }

  // Summary
  console.log("\n=== SUMMARY ===");
  const passed = results.filter(r => r.status === "PASS").length;
  const failed = results.filter(r => r.status === "FAIL").length;
  console.log(`Total: ${results.length} | Passed: ${passed} | Failed: ${failed}`);
  
  if (failed > 0) {
    console.log("\nFailed scenarios:");
    results.filter(r => r.status === "FAIL").forEach(r => {
      console.log(`  - Scenario ${r.scenario}`);
    });
  }

  // Cleanup
  server1.close();
  server2.close();
  server3.close();
  server4.close();
  
  process.exit(failed > 0 ? 1 : 0);
}

async function getAdminToken() {
  const { execSync } = await import("child_process");
  try {
    const scriptPath = "/Volumes/Yotta/auth9/.claude/skills/tools/gen-admin-token.sh";
    return execSync(`bash ${scriptPath} 2>/dev/null`, { encoding: "utf8" }).trim();
  } catch (e) {
    console.error("Failed to get admin token:", e.message);
    process.exit(1);
  }
}

runTests();