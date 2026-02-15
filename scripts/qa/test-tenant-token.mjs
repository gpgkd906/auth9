#!/usr/bin/env node
/**
 * Test Tenant Access Token verification
 * QA Document: docs/qa/sdk/03-token-verification.md
 */

import { TokenVerifier } from "./packages/node/dist/index.js";
import { execSync } from "child_process";

const GREEN = "\x1b[32m";
const RED = "\x1b[31m";
const RESET = "\x1b[0m";
const YELLOW = "\x1b[33m";

function pass(msg) {
  console.log(`${GREEN}✓${RESET} ${msg}`);
}

function fail(msg, err) {
  console.log(`${RED}✗${RESET} ${msg}`);
  if (err) console.error(`  Error: ${err.message}`);
}

function section(title) {
  console.log(`\n${YELLOW}${title}${RESET}`);
}

// Generate admin token
const identityToken = execSync("cd /Volumes/Yotta/auth9 && .claude/skills/tools/gen-admin-token.sh")
  .toString()
  .trim();

async function getTenantAccessToken() {
  // Get demo tenant ID
  const demoTenantId = execSync(
    `mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT id FROM tenants WHERE slug = 'demo';"`
  )
    .toString()
    .trim();

  // Get demo service client_id
  const serviceId = "auth9-demo"; // Using demo service

  // Use grpcurl to exchange token
  const grpcurlCmd = `
    cd /Volumes/Yotta/auth9 && 
    .claude/skills/tools/grpcurl-docker.sh \
      -cacert /certs/ca.crt -cert /certs/client.crt -key /certs/client.key -import-path /proto -proto auth9.proto \
      -H "x-api-key: dev-grpc-api-key" \
      -d '{"identity_token": "${identityToken}", "tenant_id": "${demoTenantId}", "service_id": "${serviceId}"}' \
      auth9-grpc-tls:50051 auth9.TokenExchange/ExchangeToken
  `;

  try {
    const result = execSync(grpcurlCmd, { shell: true }).toString();
    const jsonResult = JSON.parse(result);
    return jsonResult.accessToken;
  } catch (error) {
    console.error("Failed to get tenant access token:", error.message);
    console.error("Command output:", error.stdout?.toString());
    console.error("Command error:", error.stderr?.toString());
    throw error;
  }
}

async function runTests() {
  section("场景 2: 验证 Tenant Access Token");

  try {
    // Get tenant access token
    const tenantAccessToken = await getTenantAccessToken();
    console.log(`Got tenant access token: ${tenantAccessToken.substring(0, 50)}...`);

    // Verify with TokenVerifier
    const verifier = new TokenVerifier({
      domain: "http://localhost:8080",
      audience: "auth9-demo", // Demo service client_id
    });

    const { claims, tokenType } = await verifier.verify(tenantAccessToken);

    if (tokenType === "tenantAccess") {
      pass("tokenType === 'tenantAccess'");
    } else {
      fail(`tokenType should be 'tenantAccess', got '${tokenType}'`);
    }

    if (claims.aud === "auth9-demo") {
      pass("claims.aud === 'auth9-demo'");
    } else {
      fail(`claims.aud should be 'auth9-demo', got '${claims.aud}'`);
    }

    if (claims.tenantId && claims.tenantId.length > 0) {
      pass(`claims.tenantId is valid UUID: ${claims.tenantId}`);
    } else {
      fail("claims.tenantId is missing");
    }

    if (claims.roles && Array.isArray(claims.roles)) {
      pass(`claims.roles is array: ${JSON.stringify(claims.roles)}`);
    } else {
      fail("claims.roles is missing or not an array");
    }

    if (claims.permissions && Array.isArray(claims.permissions)) {
      pass(`claims.permissions is array: ${JSON.stringify(claims.permissions)}`);
    } else {
      fail("claims.permissions is missing or not an array");
    }

    if (claims.sub && claims.sub.length > 0) {
      pass(`claims.sub is valid UUID: ${claims.sub}`);
    } else {
      fail("claims.sub is missing");
    }

    // Verify database state
    const userId = claims.sub;
    const tenantId = claims.tenantId;
    
    const dbCheck = execSync(
      `mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "
        SELECT r.name 
        FROM user_tenant_roles utr
        JOIN roles r ON r.id = utr.role_id
        JOIN tenant_users tu ON tu.id = utr.tenant_user_id
        WHERE tu.user_id = '${userId}' AND tu.tenant_id = '${tenantId}';
      "`
    ).toString().trim();

    console.log(`Database roles: ${dbCheck}`);
    
    if (dbCheck) {
      pass("Database roles match claims");
    } else {
      fail("No roles found in database for this user/tenant");
    }

  } catch (err) {
    fail("场景 2 failed", err);
  }

  section("\n📊 Tenant Access Token 测试完成");
}

runTests().catch((err) => {
  console.error("Test suite failed:", err);
  process.exit(1);
});
