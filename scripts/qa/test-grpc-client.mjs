#!/usr/bin/env node
/**
 * Integration test for Auth9GrpcClient
 * QA Document: docs/qa/sdk/04-grpc-client-credentials.md
 */

import { Auth9 } from "../../sdk/packages/node/dist/index.js";
import { execSync } from "child_process";
import { readFileSync } from "fs";

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

// Get test data from database
const tenantId = execSync(
  'mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT id FROM tenants LIMIT 1;"'
)
  .toString()
  .trim();

const serviceId = execSync(
  'mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT id FROM services LIMIT 1;"'
)
  .toString()
  .trim();

const userId = execSync(
  'mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT id FROM users WHERE email = \'admin@auth9.local\' LIMIT 1;"'
)
  .toString()
  .trim();

const certDir = "/Volumes/Yotta/auth9/deploy/dev-certs/grpc";
const grpcAuth = {
  apiKey: "dev-grpc-api-key",
  mtls: {
    ca: readFileSync(`${certDir}/ca.crt`),
    cert: readFileSync(`${certDir}/client.crt`),
    key: readFileSync(`${certDir}/client.key`),
  },
};

async function runTests() {
  section("场景 1: gRPC Token Exchange 完整流程");

  let accessToken = null;

  try {
    const auth9 = new Auth9({ domain: "http://localhost:8080" });
    const grpc = auth9.grpc({ address: "localhost:50051", auth: grpcAuth });

    const result = await grpc.exchangeToken({
      identityToken,
      tenantId,
      serviceId,
    });

    if (result.accessToken && result.accessToken.split(".").length === 3) {
      pass("result.accessToken is valid JWT (three parts)");
      accessToken = result.accessToken;
    } else {
      fail("result.accessToken is not a valid JWT");
    }

    if (result.tokenType === "Bearer") {
      pass("result.tokenType === 'Bearer'");
    } else {
      fail(`result.tokenType should be 'Bearer', got '${result.tokenType}'`);
    }

    if (result.expiresIn > 0) {
      pass(`result.expiresIn > 0 (${result.expiresIn} seconds)`);
    } else {
      fail("result.expiresIn should be > 0");
    }

    if (result.refreshToken && result.refreshToken.split(".").length === 3) {
      pass("result.refreshToken is valid JWT");
    } else {
      fail("result.refreshToken is invalid");
    }

    // Decode and check claims
    const payload = JSON.parse(
      Buffer.from(result.accessToken.split(".")[1], "base64url").toString()
    );

    if (payload.tenant_id === tenantId) {
      pass(`Access token contains tenant_id: ${tenantId}`);
    } else {
      fail("Access token tenant_id mismatch");
    }

    if (Array.isArray(payload.roles)) {
      pass(`Access token contains roles: ${JSON.stringify(payload.roles)}`);
    } else {
      fail("Access token roles is not an array");
    }

    if (Array.isArray(payload.permissions)) {
      pass(`Access token contains permissions: ${JSON.stringify(payload.permissions)}`);
    } else {
      fail("Access token permissions is not an array");
    }

    grpc.close();
  } catch (err) {
    fail("场景 1 failed", err);
  }

  section("场景 2: gRPC ValidateToken 与 IntrospectToken");

  try {
    const auth9 = new Auth9({ domain: "http://localhost:8080" });
    const grpc = auth9.grpc({ address: "localhost:50051", auth: grpcAuth });

    // ValidateToken
    const validateResult = await grpc.validateToken({
      accessToken,
    });

    if (validateResult.valid === true) {
      pass("validateResult.valid === true");
    } else {
      fail("validateResult.valid should be true");
    }

    if (validateResult.userId) {
      pass(`validateResult.userId: ${validateResult.userId}`);
    } else {
      fail("validateResult.userId is missing");
    }

    if (validateResult.tenantId === tenantId) {
      pass(`validateResult.tenantId matches: ${tenantId}`);
    } else {
      fail("validateResult.tenantId mismatch");
    }

    // IntrospectToken
    const introspectResult = await grpc.introspectToken({
      token: accessToken,
    });

    if (introspectResult.active === true) {
      pass("introspectResult.active === true");
    } else {
      fail("introspectResult.active should be true");
    }

    if (introspectResult.sub === userId) {
      pass(`introspectResult.sub matches userId: ${userId}`);
    } else {
      fail("introspectResult.sub mismatch");
    }

    if (introspectResult.email) {
      pass(`introspectResult.email: ${introspectResult.email}`);
    } else {
      fail("introspectResult.email is missing");
    }

    if (Array.isArray(introspectResult.roles)) {
      pass(`introspectResult.roles: ${JSON.stringify(introspectResult.roles)}`);
    } else {
      fail("introspectResult.roles is not an array");
    }

    if (Array.isArray(introspectResult.permissions)) {
      pass(`introspectResult.permissions: ${JSON.stringify(introspectResult.permissions)}`);
    } else {
      fail("introspectResult.permissions is not an array");
    }

    // Test invalid token
    const invalidResult = await grpc.validateToken({
      accessToken: "invalid-token",
    });

    if (invalidResult.valid === false) {
      pass("Invalid token correctly rejected (valid === false)");
    } else {
      fail("Invalid token should have valid === false");
    }

    grpc.close();
  } catch (err) {
    fail("场景 2 failed", err);
  }

  section("场景 3: gRPC GetUserRoles");

  try {
    const auth9 = new Auth9({ domain: "http://localhost:8080" });
    const grpc = auth9.grpc({ address: "localhost:50051", auth: grpcAuth });

    const result = await grpc.getUserRoles({
      userId,
      tenantId,
    });

    if (Array.isArray(result.roles)) {
      pass(`result.roles is array with ${result.roles.length} items`);

      if (result.roles.length > 0) {
        const role = result.roles[0];
        if (role.id && role.name && role.serviceId !== undefined) {
          pass(`Role structure valid: { id: ${role.id}, name: ${role.name}, serviceId: ${role.serviceId} }`);
        } else {
          fail("Role structure invalid");
        }
      }
    } else {
      fail("result.roles is not an array");
    }

    if (Array.isArray(result.permissions)) {
      pass(`result.permissions is array with ${result.permissions.length} items`);
    } else {
      fail("result.permissions is not an array");
    }

    grpc.close();
  } catch (err) {
    fail("场景 3 failed", err);
  }

  section("\n📊 gRPC 客户端集成测试完成");
}

runTests().catch((err) => {
  console.error("Test suite failed:", err);
  process.exit(1);
});
