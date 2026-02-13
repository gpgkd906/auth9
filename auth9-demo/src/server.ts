import express from "express";
import path from "path";
import { fileURLToPath } from "url";
import { Auth9 } from "@auth9/node";
import { Auth9Error, Auth9HttpClient } from "@auth9/core";
import {
    auth9Middleware,
    requirePermission,
    requireRole,
} from "@auth9/node/middleware/express";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// ─── Configuration ───────────────────────────────────────────────────────────

const config = {
    port: parseInt(process.env.PORT || "3002"),
    auth9Domain: process.env.AUTH9_DOMAIN || "http://localhost:8080",
    auth9GrpcAddress: process.env.AUTH9_GRPC_ADDRESS || "localhost:50051",
    auth9GrpcApiKey: process.env.AUTH9_GRPC_API_KEY || "dev-grpc-api-key",
    auth9Audience: process.env.AUTH9_AUDIENCE || "demo-service",
    auth9AdminToken: process.env.AUTH9_ADMIN_TOKEN || "",
};

// ─── SDK Initialization ─────────────────────────────────────────────────────

// 1. Auth9 main client (token verification + gRPC)
const auth9 = new Auth9({
    domain: config.auth9Domain,
    audience: config.auth9Audience,
});

// 2. HTTP client for Management API calls
const httpClient = new Auth9HttpClient({
    baseUrl: config.auth9Domain,
    accessToken: config.auth9AdminToken || undefined,
});

// ─── Express App Setup ──────────────────────────────────────────────────────

const app = express();
app.use(express.json());
app.set("view engine", "ejs");
app.set("views", path.join(__dirname, "..", "src", "views"));

// ─── Public Routes ──────────────────────────────────────────────────────────

// Health check - no authentication required
app.get("/health", (_req, res) => {
    res.json({ status: "ok", service: "auth9-demo", timestamp: new Date().toISOString() });
});

// Home page - shows all available endpoints and integration guide
app.get("/", (_req, res) => {
    res.render("index", { config });
});

// ─── Protected Routes (Token Verification via Middleware) ───────────────────
// These endpoints require a Tenant Access Token (issued by auth9-core after Token Exchange)

const authMiddleware = auth9Middleware({
    domain: config.auth9Domain,
    audience: config.auth9Audience,
});

// Scenario 1: Get current user info from verified token
app.get("/api/me", authMiddleware, (req, res) => {
    res.json({
        message: "Token verified successfully",
        user: {
            userId: req.auth!.userId,
            email: req.auth!.email,
            tokenType: req.auth!.tokenType,
            tenantId: req.auth!.tenantId,
            roles: req.auth!.roles,
            permissions: req.auth!.permissions,
        },
    });
});

// Scenario 2: RBAC - Require specific role
app.get(
    "/api/admin",
    authMiddleware,
    requireRole("admin"),
    (_req, res) => {
        res.json({
            message: "Admin access granted",
            hint: "This endpoint requires the 'admin' role in your Tenant Access Token",
        });
    },
);

// Scenario 3: RBAC - Require specific permission
app.get(
    "/api/resources",
    authMiddleware,
    requirePermission("resource:read"),
    (_req, res) => {
        res.json({
            message: "Permission check passed",
            hint: "This endpoint requires the 'resource:read' permission",
            data: [
                { id: "1", name: "Resource A", type: "document" },
                { id: "2", name: "Resource B", type: "image" },
            ],
        });
    },
);

// ─── Demo Routes (Direct SDK calls, no middleware) ──────────────────────────
// These endpoints accept a Keycloak Identity Token directly in the Authorization header

// Scenario 4: Token Exchange via gRPC
app.post("/demo/exchange-token", async (req, res, next) => {
    try {
        const authHeader = req.headers.authorization;
        if (!authHeader?.startsWith("Bearer ")) {
            res.status(401).json({ error: "unauthorized", message: "Bearer token required" });
            return;
        }

        const { tenantId, serviceId } = req.body;
        if (!tenantId || !serviceId) {
            res.status(400).json({
                error: "bad_request",
                message: "tenantId and serviceId are required in request body",
            });
            return;
        }

        const grpc = auth9.grpc({
            address: config.auth9GrpcAddress,
            auth: { apiKey: config.auth9GrpcApiKey },
        });

        const result = await grpc.exchangeToken({
            identityToken: authHeader.slice(7),
            tenantId,
            serviceId,
        });

        grpc.close();
        res.json({
            message: "Token exchanged successfully",
            hint: "Identity Token → Tenant Access Token with roles/permissions",
            result,
        });
    } catch (err) {
        next(err);
    }
});

// Scenario 5: Token Introspection via gRPC
app.post("/demo/introspect", async (req, res, next) => {
    try {
        const authHeader = req.headers.authorization;
        if (!authHeader?.startsWith("Bearer ")) {
            res.status(401).json({ error: "unauthorized", message: "Bearer token required" });
            return;
        }

        const grpc = auth9.grpc({
            address: config.auth9GrpcAddress,
            auth: { apiKey: config.auth9GrpcApiKey },
        });

        const result = await grpc.introspectToken({ token: authHeader.slice(7) });

        grpc.close();
        res.json({
            message: "Token introspected successfully",
            result,
        });
    } catch (err) {
        next(err);
    }
});

// Scenario 6: Management API - List tenants
app.get("/demo/tenants", async (_req, res, next) => {
    try {
        const tenants = await httpClient.get("/api/v1/tenants");
        res.json({
            message: "Tenants fetched via Auth9 Management API",
            hint: "Uses Auth9HttpClient from @auth9/core",
            data: tenants,
        });
    } catch (err) {
        next(err);
    }
});

// Scenario 7: Management API - List users
app.get("/demo/users", async (_req, res, next) => {
    try {
        const users = await httpClient.get("/api/v1/users");
        res.json({
            message: "Users fetched via Auth9 Management API",
            hint: "Uses Auth9HttpClient from @auth9/core",
            data: users,
        });
    } catch (err) {
        next(err);
    }
});

// ─── Error Handler ──────────────────────────────────────────────────────────

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
        console.error("[auth9-demo] Unhandled error:", err);
        res.status(500).json({ error: "internal_error", message: err.message });
    },
);

// ─── Start Server ───────────────────────────────────────────────────────────

app.listen(config.port, () => {
    console.log(`\n🚀 Auth9 Demo running on http://localhost:${config.port}`);
    console.log(`\n📖 Integration scenarios:`);
    console.log(`  GET  /              - Home page (integration guide)`);
    console.log(`  GET  /health        - Health check (public)`);
    console.log(`  GET  /api/me        - Current user info (requires token)`);
    console.log(`  GET  /api/admin     - Admin-only endpoint (requires 'admin' role)`);
    console.log(`  GET  /api/resources - Protected resource (requires 'resource:read')`);
    console.log(`  POST /demo/exchange-token  - Token Exchange via gRPC`);
    console.log(`  POST /demo/introspect      - Token Introspection via gRPC`);
    console.log(`  GET  /demo/tenants   - List tenants (Management API)`);
    console.log(`  GET  /demo/users     - List users (Management API)`);
    console.log(`\n🔧 Configuration:`);
    console.log(`  AUTH9_DOMAIN:       ${config.auth9Domain}`);
    console.log(`  AUTH9_GRPC_ADDRESS: ${config.auth9GrpcAddress}`);
    console.log(`  AUTH9_AUDIENCE:     ${config.auth9Audience}`);
    console.log();
});
