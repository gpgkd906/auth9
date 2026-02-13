import express from "express";
import session from "express-session";
import path from "path";
import { fileURLToPath } from "url";
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
