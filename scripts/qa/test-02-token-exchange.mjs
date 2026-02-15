import { execSync } from "child_process";
import jwt from "jsonwebtoken";
import fs from "fs";
import path from "path";

const GREEN = "\x1b[32m";
const RED = "\x1b[31m";
const RESET = "\x1b[0m";
const YELLOW = "\x1b[33m";

function pass(msg) {
  console.log(`${GREEN}✓${RESET} ${msg}`);
}

function fail(msg, err) {
  console.log(`${RED}✗${RESET} ${msg}`);
  if (err) {
    if (err.stdout) console.error(`  Stdout: ${err.stdout.toString()}`);
    if (err.stderr) console.error(`  Stderr: ${err.stderr.toString()}`);
    if (err.message) console.error(`  Message: ${err.message}`);
  }
}

function section(title) {
  console.log(`\n${YELLOW}=== ${title} ===${RESET}`);
}

const PROJECT_ROOT = "/Volumes/Yotta/auth9";
const PRIVATE_KEY_PATH = path.join(PROJECT_ROOT, ".claude/skills/tools/jwt_private_clean.key");
const GRPCURL_SH = path.join(PROJECT_ROOT, ".claude/skills/tools/grpcurl-docker.sh");
const GEN_ADMIN_TOKEN_SH = path.join(PROJECT_ROOT, ".claude/skills/tools/gen-admin-token.sh");

// Helper to run grpcurl
function grpcurl(method, data) {
  const cmd = `${GRPCURL_SH} -cacert /certs/ca.crt -cert /certs/client.crt -key /certs/client.key -import-path /proto -proto auth9.proto -H "x-api-key: dev-grpc-api-key" -d '${JSON.stringify(data)}' auth9-grpc-tls:50051 auth9.TokenExchange/${method}`;
  return execSync(cmd, { cwd: PROJECT_ROOT, stdio: 'pipe' }).toString();
}

async function runTests() {
  console.log("Starting Token Exchange QA Tests...");

  // Get test data
  const demoTenantId = execSync("mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e \"SELECT id FROM tenants WHERE slug = 'demo';\"").toString().trim();
  const platformTenantId = execSync("mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e \"SELECT id FROM tenants WHERE slug = 'auth9-platform';\"").toString().trim();
  const emptyTenantId = execSync("mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e \"SELECT id FROM tenants WHERE slug = 'audit-test-tenant';\"").toString().trim();
  const adminId = execSync("mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e \"SELECT id FROM users WHERE email = 'admin@auth9.local';\"").toString().trim();
  const identityToken = execSync(GEN_ADMIN_TOKEN_SH).toString().trim();

  let tenantAccessToken = "";

  // --------------------------------------------------------------------------
  section("场景 1: Token Exchange - 获取租户访问令牌 (成功)");
  try {
    const res = JSON.parse(grpcurl("ExchangeToken", {
      identity_token: identityToken,
      tenant_id: platformTenantId,
      service_id: "auth9-portal"
    }));
    
    if (res.accessToken) {
      tenantAccessToken = res.accessToken;
      pass("Successfully exchanged token");
      console.log(`  Token starts with: ${tenantAccessToken.substring(0, 20)}...`);
    } else {
      fail("Response missing accessToken");
    }
  } catch (err) {
    fail("Scenario 1 failed", err);
  }

  // --------------------------------------------------------------------------
  section("场景 2: Token Exchange - 用户不是租户成员");
  try {
    try {
      grpcurl("ExchangeToken", {
        identity_token: identityToken,
        tenant_id: emptyTenantId,
        service_id: "auth9-portal"
      });
      fail("Should have failed for non-member tenant");
    } catch (err) {
      const stderr = err.stderr ? err.stderr.toString() : err.message;
      if (stderr.includes("not a member") || stderr.includes("PermissionDenied") || stderr.includes("not a member of tenant")) {
        pass("Correctly rejected non-member exchange");
        console.log(`  Error: ${stderr.trim()}`);
      } else {
        fail("Failed with unexpected error", err);
      }
    }
  } catch (err) {
    fail("Scenario 2 failed", err);
  }

  // --------------------------------------------------------------------------
  section("场景 3: Token 验证");
  try {
    const res = JSON.parse(grpcurl("ValidateToken", {
      access_token: tenantAccessToken
    }));
    
    if (res.valid === true) {
      pass("Token validated successfully");
      if (res.userId === adminId) pass("User ID matches");
      if (res.tenantId === platformTenantId) pass("Tenant ID matches");
    } else {
      fail("Token validation failed", { message: res.error });
    }
  } catch (err) {
    fail("Scenario 3 failed", err);
  }

  // --------------------------------------------------------------------------
  section("场景 4: Token 过期验证");
  try {
    // Generate an expired token
    const privateKey = fs.readFileSync(PRIVATE_KEY_PATH);
    const expiredToken = jwt.sign({
      sub: adminId,
      email: "admin@auth9.local",
      tenant_id: platformTenantId,
      roles: ["admin"],
      permissions: ["*"],
      aud: "auth9-portal",
      iss: "http://localhost:8080",
      iat: Math.floor(Date.now() / 1000) - 7200,
      exp: Math.floor(Date.now() / 1000) - 3600
    }, privateKey, { algorithm: 'RS256' });

    try {
      const res = JSON.parse(grpcurl("ValidateToken", {
        access_token: expiredToken
      }));
      
      // Protobuf JSON mapping might omit 'valid: false' as it's the default
      if (!res.valid && (res.error?.includes("expired") || res.error?.includes("Expired"))) {
        pass("Correctly rejected expired token");
        console.log(`  Error: ${res.error}`);
      } else {
        fail("Expired token should be invalid", { message: JSON.stringify(res) });
      }
    } catch (err) {
      const stderr = err.stderr ? err.stderr.toString() : err.message;
      if (stderr.includes("expired") || stderr.includes("Expired")) {
        pass("Correctly rejected expired token via gRPC error");
      } else {
        fail("Scenario 4 failed with unexpected error", err);
      }
    }
  } catch (err) {
    fail("Scenario 4 failed", err);
  }

  // --------------------------------------------------------------------------
  section("场景 5: Token 内省");
  try {
    const res = JSON.parse(grpcurl("IntrospectToken", {
      token: tenantAccessToken
    }));
    
    if (res.active === true) {
      pass("Token is active");
      if (res.sub === adminId) pass("Subject matches");
      if (res.tenantId === platformTenantId) pass("Tenant ID matches");
      if (res.roles && res.roles.length > 0) pass(`Found roles: ${res.roles.join(", ")}`);
      if (res.permissions && res.permissions.length > 0) pass(`Found ${res.permissions.length} permissions`);
    } else {
      fail("Introspection failed - token inactive");
    }
  } catch (err) {
    fail("Scenario 5 failed", err);
  }

  console.log("\nToken Exchange QA Tests Completed.");
}

runTests().catch(err => {
  console.error("Test execution failed:", err);
});
