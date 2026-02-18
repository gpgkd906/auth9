import express from "express";
import { Auth9, Auth9Error } from "@auth9/node";
import {
  auth9Middleware,
  requirePermission,
  requireRole,
} from "@auth9/node/middleware/express";

const app = express();
app.use(express.json());

// Initialize Auth9
const auth9 = new Auth9({
  domain: process.env.AUTH9_DOMAIN || "http://localhost:8080",
    audience: process.env.AUTH9_AUDIENCE || "auth9",
  clientId: process.env.AUTH9_CLIENT_ID,
  clientSecret: process.env.AUTH9_CLIENT_SECRET,
});

// Public route
app.get("/health", (_req, res) => {
  res.json({ status: "ok" });
});

// Optional auth route
app.get("/optional", auth9Middleware({
  domain: process.env.AUTH9_DOMAIN || "http://localhost:8080",
  audience: process.env.AUTH9_AUDIENCE || "auth9",
  optional: true,
}), (req, res) => {
  if (req.auth) {
    res.json({ message: "Authenticated", user: req.auth.email });
  } else {
    res.json({ message: "Anonymous" });
  }
});

// Protected routes - require valid Auth9 token
app.use(
  "/api",
  auth9Middleware({
    domain: process.env.AUTH9_DOMAIN || "http://localhost:8080",
    audience: process.env.AUTH9_AUDIENCE || "auth9",
  }),
);

// Any authenticated user
app.get("/api/me", (req, res) => {
  res.json({
    userId: req.auth!.userId,
    email: req.auth!.email,
    tokenType: req.auth!.tokenType,
    tenantId: req.auth!.tenantId,
    roles: req.auth!.roles,
  });
});

// Require specific permission
app.get(
  "/api/users",
  requirePermission("user:read"),
  (_req, res) => {
    res.json({ data: [{ id: "1", email: "user@example.com" }] });
  },
);

// Test multiple permissions (all mode)
app.post(
  "/api/users",
  requirePermission(["user:read", "user:write"]),
  (_req, res) => {
    res.json({ message: "User created" });
  },
);

// Test any mode
app.patch(
  "/api/users/:id",
  requirePermission(["user:write", "user:admin"], { mode: "any" }),
  (req, res) => {
    res.json({ message: `User ${req.params.id} updated` });
  },
);

// Test missing permission
app.delete(
  "/api/users/:id",
  requirePermission("user:delete"),
  (req, res) => {
    res.json({ message: `User ${req.params.id} deleted` });
  },
);

// Require admin role
app.delete(
  "/api/users/:id",
  requireRole("admin"),
  (req, res) => {
    res.json({ message: `User ${req.params.id} deleted` });
  },
);

// Test role checks
app.get(
  "/api/admin",
  requireRole("admin"),
  (_req, res) => {
    res.json({ message: "Admin access granted" });
  },
);

app.get(
  "/api/superadmin",
  requireRole("superadmin"),
  (_req, res) => {
    res.json({ message: "Superadmin access granted" });
  },
);

app.get(
  "/api/any-admin",
  requireRole(["admin", "superadmin"], { mode: "any" }),
  (_req, res) => {
    res.json({ message: "Any admin access granted" });
  },
);

// Test AuthInfo helper methods
app.get(
  "/api/check-helpers",
  (req, res) => {
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
      roles: req.auth.roles,
      permissions: req.auth.permissions,
    });
  },
);

// Token Exchange example via gRPC
app.post("/api/exchange-token", async (req, res, next) => {
  try {
    const grpc = auth9.grpc({
      address: process.env.AUTH9_GRPC_ADDRESS || "localhost:50051",
    });

    const result = await grpc.exchangeToken({
      identityToken: req.headers.authorization!.slice(7),
      tenantId: req.body.tenantId,
      serviceId: req.body.serviceId,
    });

    grpc.close();
    // Use result.accessToken (tenant token) for downstream RBAC-protected calls.
    res.json(result);
  } catch (err) {
    next(err);
  }
});

// M2M service token example
app.get("/api/service-info", async (_req, res, next) => {
  try {
    const token = await auth9.getServiceToken();
    res.json({ serviceToken: token.slice(0, 20) + "..." });
  } catch (err) {
    next(err);
  }
});

// Error handler
app.use(
  (
    err: Error,
    _req: express.Request,
    res: express.Response,
    _next: express.NextFunction,
  ) => {
    if (err instanceof Auth9Error) {
      res.status(err.statusCode).json({
        error: err.code,
        message: err.message,
      });
      return;
    }
    console.error(err);
    res.status(500).json({ error: "internal_error", message: "Internal server error" });
  },
);

const port = process.env.PORT || 3003;
app.listen(port, () => {
  console.log(`Example API running on http://localhost:${port}`);
  console.log(`  GET  /health         - Health check`);
  console.log(`  GET  /api/me         - Current user info`);
  console.log(`  GET  /api/users      - List users (requires user:read)`);
  console.log(`  DEL  /api/users/:id  - Delete user (requires admin role)`);
});
