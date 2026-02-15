#!/usr/bin/env node
/**
 * Test Audience verification
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

// Generate admin token (aud="auth9")
const identityToken = execSync("cd /Volumes/Yotta/auth9 && .claude/skills/tools/gen-admin-token.sh")
  .toString()
  .trim();

async function runTests() {
  section("场景 4: Audience 验证");

  try {
    // Test 1: Verifier with specific audience should reject token with different aud
    const verifierWithAudience = new TokenVerifier({
      domain: "http://localhost:8080",
      audience: "other-service",  // Different from token's aud="auth9"
    });

    try {
      await verifierWithAudience.verify(identityToken);
      fail("Token with aud='auth9' should be rejected by verifier with audience='other-service'");
    } catch (err) {
      if (err.message.includes("audience") || err.message.includes("aud")) {
        pass("Token correctly rejected due to audience mismatch");
      } else {
        fail(`Unexpected error: ${err.message}`);
      }
    }

    // Test 2: Verifier without audience should accept all valid tokens
    const permissiveVerifier = new TokenVerifier({
      domain: "http://localhost:8080",
      // No audience specified
    });

    try {
      const result = await permissiveVerifier.verify(identityToken);
      if (result.claims && result.claims.sub) {
        pass("Permissive verifier (no audience) accepts token with aud='auth9'");
        pass(`  Verified claims: sub=${result.claims.sub}, aud=${result.claims.aud}`);
      } else {
        fail("Permissive verifier returned invalid result");
      }
    } catch (err) {
      fail("Permissive verifier should accept token without audience check", err);
    }

    // Test 3: Verifier with matching audience should accept token
    const matchingVerifier = new TokenVerifier({
      domain: "http://localhost:8080",
      audience: "auth9",  // Matches token's aud
    });

    try {
      const result = await matchingVerifier.verify(identityToken);
      if (result.claims && result.claims.aud === "auth9") {
        pass("Verifier with audience='auth9' accepts token with matching aud");
      } else {
        fail("Matching verifier returned unexpected result");
      }
    } catch (err) {
      fail("Verifier with matching audience should accept token", err);
    }

    // Test 4: Test with tenant access token
    // First get a tenant access token
    const demoTenantId = execSync(
      `mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT id FROM tenants WHERE slug = 'demo';"`
    ).toString().trim();

    const grpcurlCmd = `
      cd /Volumes/Yotta/auth9 && 
      .claude/skills/tools/grpcurl-docker.sh \
        -insecure -import-path /proto -proto auth9.proto \
        -H "x-api-key: dev-grpc-api-key" \
        -d '{"identity_token": "${identityToken}", "tenant_id": "${demoTenantId}", "service_id": "auth9-demo"}' \
        auth9-grpc-tls:50051 auth9.TokenExchange/ExchangeToken
    `;

    const result = execSync(grpcurlCmd, { shell: true }).toString();
    const jsonResult = JSON.parse(result);
    const tenantAccessToken = jsonResult.accessToken;

    // Verifier with correct audience for tenant access token
    const tenantVerifier = new TokenVerifier({
      domain: "http://localhost:8080",
      audience: "auth9-demo",  // Matches tenant token's aud
    });

    try {
      const tenantResult = await tenantVerifier.verify(tenantAccessToken);
      if (tenantResult.claims && tenantResult.claims.aud === "auth9-demo") {
        pass("Tenant access token verified with correct audience");
      } else {
        fail("Tenant token verification failed");
      }
    } catch (err) {
      fail("Tenant token should be accepted with correct audience", err);
    }

    // Verifier with wrong audience for tenant access token
    const wrongTenantVerifier = new TokenVerifier({
      domain: "http://localhost:8080",
      audience: "wrong-service",  // Doesn't match tenant token's aud
    });

    try {
      await wrongTenantVerifier.verify(tenantAccessToken);
      fail("Tenant access token should be rejected with wrong audience");
    } catch (err) {
      if (err.message.includes("audience") || err.message.includes("aud")) {
        pass("Tenant access token correctly rejected due to audience mismatch");
      } else {
        fail(`Unexpected error for tenant token: ${err.message}`);
      }
    }

  } catch (err) {
    fail("场景 4 failed", err);
  }

  section("\n📊 Audience 验证测试完成");
}

runTests().catch((err) => {
  console.error("Test suite failed:", err);
  process.exit(1);
});