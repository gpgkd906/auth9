import express from "express";
import session from "express-session";
import path from "path";
import { fileURLToPath } from "url";
import crypto from "node:crypto";
import { Auth9 } from "@auth9/node";
import { Auth9Error, Auth9HttpClient } from "@auth9/core";
import * as jose from "jose";
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
    auth9PublicUrl: process.env.AUTH9_PUBLIC_URL || "http://localhost:8080",
    auth9GrpcAddress: process.env.AUTH9_GRPC_ADDRESS || "localhost:50051",
    auth9GrpcApiKey: process.env.AUTH9_GRPC_API_KEY || "dev-grpc-api-key",
    auth9Audience: process.env.AUTH9_AUDIENCE || "demo-service",
    auth9AdminToken: process.env.AUTH9_ADMIN_TOKEN || "",
    clientId: process.env.AUTH9_CLIENT_ID || "auth9-demo",
    defaultTenantId: process.env.AUTH9_DEFAULT_TENANT_ID || "demo",
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

app.set("view engine", "ejs");
// Fix views path: at runtime __dirname is dist/, but EJS templates are in src/views.
// We need to check if we are running from dist or src.
// Since we copy views in Dockerfile, checking '../src/views' might be wrong in prod if not copied to expected place relative to dist.
// In Dockerfile we do: COPY src/views ./dist/views (if we change tsconfig to include views? No, we copy manually or use clean structure)
// Let's assume standard structure: src/views.
// In ts-node dev: src/server.ts -> views in src/views
// In dist: dist/server.js -> views in dist/views
// Let's try to detect or just use relative path that works for both if structure is preserved.
app.set("views", path.join(__dirname, "../src/views"));

app.use(express.static("public"));
app.use(express.json());
app.use(express.urlencoded({ extended: true }));

// Session middleware
app.use(
    session({
        secret: "demo-secret-key-change-in-prod",
        resave: false,
        saveUninitialized: false,
        cookie: { secure: false }, // set to true if using https
    })
);

// Extend Express Request type to include session
declare module "express-session" {
    interface SessionData {
        user?: any;
        identityToken?: string;
    }
}

// ─── Routes ─────────────────────────────────────────────────────────────────

// 1. Home Page
app.get("/", (req, res) => {
    res.render("index", { user: req.session.user, config });
});

// 1.1 Health check
app.get("/health", (_req, res) => {
    res.json({ status: "ok", service: "auth9-demo" });
});

// 2. Login - Redirect to Auth9 Core Authorize Endpoint
app.get("/login", (req, res) => {
    const authUrl = new URL(`${config.auth9PublicUrl}/api/v1/auth/authorize`);
    authUrl.searchParams.append("client_id", config.clientId);
    authUrl.searchParams.append(
        "redirect_uri",
        `http://localhost:${config.port}/auth/callback`
    );
    authUrl.searchParams.append("response_type", "code");
    authUrl.searchParams.append("scope", "openid profile email");
    authUrl.searchParams.append("state", "random-state-string"); // Should be random
    authUrl.searchParams.append("nonce", "random-nonce-string");

    res.redirect(authUrl.toString());
});

// 2.1 Enterprise SSO Login (domain discovery -> redirect)
app.post("/enterprise/login", async (req, res) => {
    const email = String(req.body.email || "").trim().toLowerCase();
    if (!email) {
        return res.status(400).send("Email is required");
    }

    const state = crypto.randomUUID();
    const nonce = crypto.randomUUID();
    // Use internal core URL for server-to-server discovery call.
    const discoveryUrl = new URL(
        `${config.auth9Domain}/api/v1/enterprise-sso/discovery`
    );
    discoveryUrl.searchParams.set("response_type", "code");
    discoveryUrl.searchParams.set("client_id", config.clientId);
    discoveryUrl.searchParams.set(
        "redirect_uri",
        `http://localhost:${config.port}/auth/callback`
    );
    discoveryUrl.searchParams.set("scope", "openid profile email");
    discoveryUrl.searchParams.set("state", state);
    discoveryUrl.searchParams.set("nonce", nonce);

    try {
        const response = await fetch(discoveryUrl.toString(), {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ email }),
        });

        if (!response.ok) {
            const text = await response.text();
            return res.status(response.status).send(text);
        }

        const payload = (await response.json()) as {
            data?: { authorize_url?: string };
        };
        const authorizeUrl = payload?.data?.authorize_url;
        if (!authorizeUrl) {
            return res
                .status(502)
                .send("Discovery succeeded but authorize_url is missing");
        }

        return res.redirect(authorizeUrl);
    } catch (err: any) {
        console.error("Enterprise discovery error:", err);
        return res.status(500).send(`Enterprise discovery failed: ${err.message}`);
    }
});

// 3. Callback - Exchange Code for Token
app.get("/auth/callback", async (req, res) => {
    const { code, state, error, error_description } = req.query;

    if (error) {
        res.status(400).send(`Auth Error: ${error} - ${error_description}`);
        return;
    }

    if (!code) {
        res.status(400).send("Missing code");
        return;
    }

    try {
        // Exchange code for tokens via Auth9 Core
        const tokenResponse = await fetch(
            `${config.auth9Domain}/api/v1/auth/token`,
            {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({
                    grant_type: "authorization_code",
                    client_id: config.clientId,
                    code: code.toString(),
                    redirect_uri: `http://localhost:${config.port}/auth/callback`,
                }),
            }
        );

        if (!tokenResponse.ok) {
            const errText = await tokenResponse.text();
            throw new Error(`Token exchange failed: ${errText}`);
        }

        const tokens = await tokenResponse.json();
        // access_token is the Auth9-signed Identity Token (used for gRPC token exchange)
        // id_token is the Keycloak-signed OIDC token (for display/OIDC purposes only)
        const identityToken = tokens.access_token;

        // Decode token to get user info
        const claims = jose.decodeJwt(identityToken);

        // Store in session
        req.session.user = {
            sub: claims.sub,
            email: claims.email,
            name: claims.name || claims.preferred_username,
        };
        req.session.identityToken = identityToken;

        res.redirect("/dashboard");
    } catch (err: any) {
        console.error("Callback error:", err);
        res.status(500).send(`Authentication failed: ${err.message}`);
    }
});

// 4. Dashboard (Protected)
app.get("/dashboard", (req, res) => {
    if (!req.session.user) {
        return res.redirect("/");
    }

    res.render("dashboard", {
        user: req.session.user,
        identityToken: req.session.identityToken,
        config,
    });
});

// 5. Logout
app.get("/logout", (req, res) => {
    req.session.destroy(() => {
        // Redirect to Auth9 logout logic if needed, or just home
        res.redirect("/");
    });
});

// ─── API Routes (Middleware Protected) ──────────────────────────────────────

// These routes require a Tenant Access Token via Bearer header
// The client (dashboard) must obtain this token via /demo/exchange-token first

const apiRouter = express.Router();

// Apply Auth9 middleware to all /api routes
// Note: Middleware expects Tenant Access Token, not Identity Token
apiRouter.use(auth9Middleware({
    domain: config.auth9Domain,
    audience: config.auth9Audience,
}));

apiRouter.get("/me", (req, res) => {
    res.json({
        message: "You are authenticated",
        user: req.auth,
    });
});

apiRouter.get("/admin", requireRole("admin"), (req, res) => {
    res.json({
        message: "Welcome Admin",
        user: req.auth,
    });
});

apiRouter.get("/resources", requirePermission("resource:read"), (req, res) => {
    res.json({
        data: [
            { id: "1", name: "Resource A", type: "document" },
            { id: "2", name: "Resource B", type: "image" },
        ],
        user: req.auth,
    });
});

app.use("/api", apiRouter);

// ─── Demo Routes (Public / Direct SDK Calls) ────────────────────────────────

// These routes demonstrate direct SDK usage (gRPC, Management API)
// They do NOT use the global auth9Middleware because we want to handle
// raw Identity Tokens or specific flows manually.

const demoRouter = express.Router();

// Parse JSON for these routes
demoRouter.use(express.json());

function resolveAdminToken(req: express.Request, res: express.Response): string | null {
    if (config.auth9AdminToken) {
        return config.auth9AdminToken;
    }

    const authHeader = req.headers.authorization;
    if (authHeader?.startsWith("Bearer ")) {
        return authHeader.slice("Bearer ".length);
    }

    const rawHeaderToken = req.headers["x-admin-token"];
    const headerToken = Array.isArray(rawHeaderToken)
        ? rawHeaderToken[0]
        : rawHeaderToken;
    if (headerToken?.trim()) {
        return headerToken.trim();
    }

    const bodyToken = String(req.body?.adminToken || "").trim();
    if (bodyToken) {
        return bodyToken;
    }

    const queryToken = String(req.query.adminToken || "").trim();
    if (queryToken) {
        return queryToken;
    }

    res.status(400).json({
        error: "missing_admin_token",
        message: "Provide AUTH9_ADMIN_TOKEN or pass admin token via Authorization/x-admin-token",
    });
    return null;
}

// Token Exchange: Identity Token -> Tenant Access Token
demoRouter.post("/exchange-token", async (req, res) => {
    const authHeader = req.headers.authorization;
    if (!authHeader?.startsWith("Bearer ")) {
        res.status(401).json({ error: "Missing Bearer token" });
        return;
    }
    const identityToken = authHeader.split(" ")[1];
    const { tenantId } = req.body;

    if (!tenantId) {
        res.status(400).json({ error: "Missing tenantId" });
        return;
    }

    console.log(`[Demo] Exchanging token for tenant: ${tenantId}`);

    try {
        // Use gRPC to exchange token
        const grpc = auth9.grpc({
            address: config.auth9GrpcAddress,
            auth: { apiKey: config.auth9GrpcApiKey },
        });

        const result = await grpc.exchangeToken({
            identityToken,
            tenantId,
            serviceId: config.auth9Audience,
        });

        grpc.close();
        res.json(result);
    } catch (err: any) {
        console.error("[Demo] Token exchange error:", err);
        if (err instanceof Auth9Error) {
            res.status(err.statusCode).json({
                error: err.code,
                message: err.message,
            });
            return;
        }
        res.status(500).json({ error: "exchange_failed", message: err.message });
    }
});

// Introspection: Check token status via gRPC
demoRouter.post("/introspect", async (req, res) => {
    const authHeader = req.headers.authorization;
    if (!authHeader?.startsWith("Bearer ")) {
        res.status(401).json({ error: "Missing Bearer token" });
        return;
    }
    const token = authHeader.split(" ")[1];

    try {
        const grpc = auth9.grpc({
            address: config.auth9GrpcAddress,
            auth: { apiKey: config.auth9GrpcApiKey },
        });

        const result = await grpc.introspectToken({ token });
        grpc.close();
        res.json(result);
    } catch (err: any) {
        console.error("[Demo] Token introspection error:", err);
        if (err instanceof Auth9Error) {
            res.status(err.statusCode).json({
                error: err.code,
                message: err.message,
            });
            return;
        }
        res.status(500).json({ error: "introspection_failed", message: err.message });
    }
});

// Management API: List Tenants
demoRouter.get("/tenants", async (_req, res) => {
    try {
        const response = await httpClient.get("/api/v1/tenants");
        res.json(response);
    } catch (err: any) {
        console.error("[Demo] List tenants error:", err);
        if (err instanceof Auth9Error) {
            res.status(err.statusCode).json({
                error: err.code,
                message: err.message,
            });
            return;
        }
        res.status(500).json({ error: "list_tenants_failed", message: err.message });
    }
});

// Management API: List Users
demoRouter.get("/users", async (req, res) => {
    try {
        // Extract tenantId from query if provided
        const tenantId = req.query.tenantId as string | undefined;
        // Mock query for now or pass if supported
        // The SDK might not expose filters yet, assuming simple list
        const response = await httpClient.get("/api/v1/users");
        res.json(response);
    } catch (err: any) {
        console.error("[Demo] List users error:", err);
        if (err instanceof Auth9Error) {
            res.status(err.statusCode).json({
                error: err.code,
                message: err.message,
            });
            return;
        }
        res.status(500).json({ error: "list_users_failed", message: err.message });
    }
});

// Enterprise SSO: Domain discovery (for QA scripts)
demoRouter.post("/enterprise/discovery", async (req, res) => {
    const email = String(req.body?.email || "").trim().toLowerCase();
    if (!email) {
        return res
            .status(400)
            .json({ error: "missing_email", message: "email is required" });
    }

    const state = crypto.randomUUID();
    const nonce = crypto.randomUUID();
    const discoveryUrl = new URL(
        `${config.auth9Domain}/api/v1/enterprise-sso/discovery`
    );
    discoveryUrl.searchParams.set("response_type", "code");
    discoveryUrl.searchParams.set("client_id", config.clientId);
    discoveryUrl.searchParams.set(
        "redirect_uri",
        `http://localhost:${config.port}/auth/callback`
    );
    discoveryUrl.searchParams.set("scope", "openid profile email");
    discoveryUrl.searchParams.set("state", state);
    discoveryUrl.searchParams.set("nonce", nonce);

    try {
        const response = await fetch(discoveryUrl.toString(), {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ email }),
        });
        const text = await response.text();
        res.status(response.status).type("application/json").send(text);
    } catch (err: any) {
        res.status(500).json({
            error: "enterprise_discovery_failed",
            message: err.message,
        });
    }
});

// Enterprise SSO: List tenant connectors
demoRouter.get("/enterprise/connectors", async (req, res) => {
    const tenantId = String(req.query.tenantId || config.defaultTenantId);
    const adminToken = resolveAdminToken(req, res);
    if (!adminToken) return;

    try {
        const response = await fetch(
            `${config.auth9Domain}/api/v1/tenants/${tenantId}/sso/connectors`,
            {
                headers: {
                    Authorization: `Bearer ${adminToken}`,
                    "Content-Type": "application/json",
                },
            }
        );
        const text = await response.text();
        res.status(response.status).type("application/json").send(text);
    } catch (err: any) {
        res.status(500).json({ error: "list_connectors_failed", message: err.message });
    }
});

// Enterprise SSO: Create connector
demoRouter.post("/enterprise/connectors", async (req, res) => {
    const tenantId = String(req.body?.tenantId || config.defaultTenantId);
    const adminToken = resolveAdminToken(req, res);
    if (!adminToken) return;

    try {
        const response = await fetch(
            `${config.auth9Domain}/api/v1/tenants/${tenantId}/sso/connectors`,
            {
                method: "POST",
                headers: {
                    Authorization: `Bearer ${adminToken}`,
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(req.body),
            }
        );
        const text = await response.text();
        res.status(response.status).type("application/json").send(text);
    } catch (err: any) {
        res.status(500).json({ error: "create_connector_failed", message: err.message });
    }
});

// Enterprise SSO: Update connector
demoRouter.put("/enterprise/connectors/:connectorId", async (req, res) => {
    const tenantId = String(req.body?.tenantId || req.query.tenantId || config.defaultTenantId);
    const connectorId = String(req.params.connectorId);
    const adminToken = resolveAdminToken(req, res);
    if (!adminToken) return;

    try {
        const response = await fetch(
            `${config.auth9Domain}/api/v1/tenants/${tenantId}/sso/connectors/${connectorId}`,
            {
                method: "PUT",
                headers: {
                    Authorization: `Bearer ${adminToken}`,
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(req.body),
            }
        );
        const text = await response.text();
        res.status(response.status).type("application/json").send(text);
    } catch (err: any) {
        res.status(500).json({ error: "update_connector_failed", message: err.message });
    }
});

// Enterprise SSO: Delete connector
demoRouter.delete("/enterprise/connectors/:connectorId", async (req, res) => {
    const tenantId = String(req.query.tenantId || config.defaultTenantId);
    const connectorId = String(req.params.connectorId);
    const adminToken = resolveAdminToken(req, res);
    if (!adminToken) return;

    try {
        const response = await fetch(
            `${config.auth9Domain}/api/v1/tenants/${tenantId}/sso/connectors/${connectorId}`,
            {
                method: "DELETE",
                headers: {
                    Authorization: `Bearer ${adminToken}`,
                    "Content-Type": "application/json",
                },
            }
        );
        const text = await response.text();
        res.status(response.status).type("application/json").send(text);
    } catch (err: any) {
        res.status(500).json({ error: "delete_connector_failed", message: err.message });
    }
});

// Enterprise SSO: Test connector
demoRouter.post("/enterprise/connectors/:connectorId/test", async (req, res) => {
    const tenantId = String(req.body?.tenantId || req.query.tenantId || config.defaultTenantId);
    const connectorId = String(req.params.connectorId);
    const adminToken = resolveAdminToken(req, res);
    if (!adminToken) return;

    try {
        const response = await fetch(
            `${config.auth9Domain}/api/v1/tenants/${tenantId}/sso/connectors/${connectorId}/test`,
            {
                method: "POST",
                headers: {
                    Authorization: `Bearer ${adminToken}`,
                    "Content-Type": "application/json",
                },
            }
        );
        const text = await response.text();
        res.status(response.status).type("application/json").send(text);
    } catch (err: any) {
        res.status(500).json({ error: "test_connector_failed", message: err.message });
    }
});

app.use("/demo", demoRouter);

// ─── Error Handling & Startup ───────────────────────────────────────────────

app.use((err: any, _req: express.Request, res: express.Response, _next: express.NextFunction) => {
    console.error("[auth9-demo] Unhandled error:", err);
    res.status(500).json({ error: "internal_error", message: err.message });
});

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
