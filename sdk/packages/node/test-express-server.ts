import express from "express";
import { auth9Middleware, requirePermission, requireRole } from "./dist/middleware/express.js";

const app = express();
const PORT = 3001;

// Middleware: auth9Middleware (non-optional)
app.use(auth9Middleware({
  domain: "http://localhost:8080",
  audience: "auth9-portal",
}));

// Test route for scenario 1: req.auth injection
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

// Test route for scenario 3: optional mode
const optionalApp = express();
optionalApp.use(auth9Middleware({
  domain: "http://localhost:8080",
  optional: true,
}));

optionalApp.get("/public-or-private", (req, res) => {
  if (req.auth) {
    res.json({ message: "Authenticated", user: req.auth.email });
  } else {
    res.json({ message: "Anonymous" });
  }
});

// Test route for scenario 4: requirePermission
app.get("/users", requirePermission("user:read"), (req, res) => {
  res.json({ route: "GET /users" });
});

app.post("/users", requirePermission(["user:read", "user:write"]), (req, res) => {
  res.json({ route: "POST /users" });
});

app.delete("/users/:id", requirePermission("user:delete"), (req, res) => {
  res.json({ route: "DELETE /users/:id" });
});

app.patch("/users/:id", requirePermission(["user:write", "user:admin"], { mode: "any" }), (req, res) => {
  res.json({ route: "PATCH /users/:id" });
});

// Test route for scenario 5: requireRole
app.get("/admin", requireRole("admin"), (req, res) => {
  res.json({ route: "GET /admin" });
});

app.get("/superadmin", requireRole("superadmin"), (req, res) => {
  res.json({ route: "GET /superadmin" });
});

app.get("/any-admin", requireRole(["admin", "superadmin"], { mode: "any" }), (req, res) => {
  res.json({ route: "GET /any-admin" });
});

// Test route for AuthInfo helpers
app.get("/check-helpers", (req, res) => {
  res.json({
    hasReadPerm: req.auth!.hasPermission("user:read"),
    hasDeletePerm: req.auth!.hasPermission("user:delete"),
    isAdmin: req.auth!.hasRole("admin"),
    isSuperAdmin: req.auth!.hasRole("superadmin"),
    hasAnyWritePerm: req.auth!.hasAnyPermission(["user:write", "user:admin"]),
    hasAllPerms: req.auth!.hasAllPermissions(["user:read", "user:write"]),
    hasAllPermsIncDelete: req.auth!.hasAllPermissions(["user:read", "user:delete"]),
  });
});

// Error handler
app.use((err: any, req: express.Request, res: express.Response, next: express.NextFunction) => {
  const status = err.statusCode || err.status || 500;
  res.status(status).json({
    error: err.name || "Error",
    message: err.message,
  });
});

optionalApp.use((err: any, req: express.Request, res: express.Response, next: express.NextFunction) => {
  const status = err.statusCode || err.status || 500;
  res.status(status).json({
    error: err.name || "Error",
    message: err.message,
  });
});

// Start servers
app.listen(PORT, () => {
  console.log(`Server running on port ${PORT}`);
});

optionalApp.listen(PORT + 1, () => {
  console.log(`Optional server running on port ${PORT + 1}`);
});
