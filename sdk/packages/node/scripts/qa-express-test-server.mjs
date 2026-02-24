#!/usr/bin/env node
import express from "express";
import { auth9Middleware, requirePermission, requireRole } from "@auth9/node/middleware/express";
import { createServer } from "http";

const app = express();
const PORT = 13001;

app.use(auth9Middleware({
  domain: "http://localhost:8080",
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

app.get("/public-or-private", (req, res) => {
  if (req.auth) {
    res.json({ message: "Authenticated", user: req.auth.email });
  } else {
    res.json({ message: "Anonymous" });
  }
});

app.get("/users", requirePermission("user:read"), (req, res) => {
  res.json({ data: "users list" });
});

app.post("/users", requirePermission(["user:read", "user:write"]), (req, res) => {
  res.json({ data: "user created" });
});

app.delete("/users/:id", requirePermission("user:delete"), (req, res) => {
  res.json({ data: "user deleted" });
});

app.patch("/users/:id", requirePermission(["user:write", "user:admin"], { mode: "any" }), (req, res) => {
  res.json({ data: "user patched" });
});

app.get("/admin", requireRole("admin"), (req, res) => {
  res.json({ data: "admin area" });
});

app.get("/superadmin", requireRole("superadmin"), (req, res) => {
  res.json({ data: "superadmin area" });
});

app.get("/any-admin", requireRole(["admin", "superadmin"], { mode: "any" }), (req, res) => {
  res.json({ data: "any admin area" });
});

app.get("/check-helpers", (req, res) => {
  if (!req.auth) {
    return res.status(401).json({ error: "Unauthorized" });
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

const server = createServer(app);

server.listen(PORT, () => {
  console.log(`Test server running on http://localhost:${PORT}`);
});

process.on('SIGTERM', () => {
  server.close(() => process.exit(0));
});
