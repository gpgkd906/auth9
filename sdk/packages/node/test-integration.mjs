import { Auth9GrpcClient } from "./dist/index.js";
import { ClientCredentials } from "./dist/index.js";

const ADMIN_TOKEN = process.env.ADMIN_TOKEN || "";
const TENANT_ID = process.env.TENANT_ID || "6e71e4d8-7752-4c7d-aafc-66c02a77ca00";
const SERVICE_ID = process.env.SERVICE_ID || "auth9-portal";
const USER_ID = process.env.USER_ID || "0ed10582-aab3-4525-b735-8a354e3f1401";
const CLIENT_ID = process.env.CLIENT_ID || "auth9-portal";
const CLIENT_SECRET = process.env.CLIENT_SECRET || "";
const GRPC_ADDRESS = process.env.GRPC_ADDRESS || "localhost:50051";
const DOMAIN = process.env.DOMAIN || "http://localhost:8080";

async function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function testScenario1() {
  console.log("\n=== Scenario 1: gRPC Token Exchange ===");
  const client = new Auth9GrpcClient({
    address: GRPC_ADDRESS,
    auth: { apiKey: "dev-grpc-api-key" }, // pragma: allowlist secret
  });

  try {
    const result = await client.exchangeToken({
      identityToken: ADMIN_TOKEN,
      tenantId: TENANT_ID,
      serviceId: SERVICE_ID,
    });

    console.log("Result:", result);
    
    // Validate response
    const hasAccessToken = result.accessToken && result.accessToken.split(".").length === 3;
    const hasTokenType = result.tokenType === "Bearer";
    const hasExpiresIn = result.expiresIn > 0;
    
    console.log("✓ accessToken is JWT:", hasAccessToken);
    console.log("✓ tokenType is Bearer:", hasTokenType);
    console.log("✓ expiresIn > 0:", hasExpiresIn);
    
    if (hasAccessToken && hasTokenType && hasExpiresIn) {
      console.log("✅ Scenario 1 PASS");
      return { success: true, accessToken: result.accessToken };
    } else {
      console.log("❌ Scenario 1 FAIL");
      return { success: false };
    }
  } catch (err) {
    console.error("❌ Scenario 1 FAIL:", err.message);
    return { success: false, error: err.message };
  } finally {
    client.close();
  }
}

async function testScenario2(accessToken) {
  console.log("\n=== Scenario 2: gRPC ValidateToken & IntrospectToken ===");
  const client = new Auth9GrpcClient({
    address: GRPC_ADDRESS,
    auth: { apiKey: "dev-grpc-api-key" }, // pragma: allowlist secret
  });

  try {
    // Test validateToken with valid token
    const validateResult = await client.validateToken({ accessToken });
    console.log("Validate Result:", validateResult);
    
    const validPass = validateResult.valid === true && validateResult.userId && validateResult.tenantId;
    console.log("✓ Valid token validation:", validPass);
    
    // Test validateToken with invalid token
    const invalidResult = await client.validateToken({ accessToken: "invalid-token" });
    console.log("Invalid Validate Result:", invalidResult);
    
    const invalidPass = invalidResult.valid === false;
    console.log("✓ Invalid token validation:", invalidPass);
    
    // Test introspectToken
    const introspectResult = await client.introspectToken({ token: accessToken });
    console.log("Introspect Result:", {
      active: introspectResult.active,
      sub: introspectResult.sub,
      email: introspectResult.email,
      tenantId: introspectResult.tenantId,
      roles: introspectResult.roles,
      permissions: introspectResult.permissions,
    });
    
    const introspectPass = introspectResult.active === true && introspectResult.sub && introspectResult.roles;
    console.log("✓ Token introspection:", introspectPass);
    
    if (validPass && invalidPass && introspectPass) {
      console.log("✅ Scenario 2 PASS");
      return { success: true };
    } else {
      console.log("❌ Scenario 2 FAIL");
      return { success: false };
    }
  } catch (err) {
    console.error("❌ Scenario 2 FAIL:", err.message);
    return { success: false, error: err.message };
  } finally {
    client.close();
  }
}

async function testScenario3() {
  console.log("\n=== Scenario 3: gRPC GetUserRoles ===");
  const client = new Auth9GrpcClient({
    address: GRPC_ADDRESS,
    auth: { apiKey: "dev-grpc-api-key" }, // pragma: allowlist secret
  });

  try {
    const result = await client.getUserRoles({
      userId: USER_ID,
      tenantId: TENANT_ID,
    });

    console.log("GetUserRoles Result:", result);
    
    // Validate response
    const hasRoles = Array.isArray(result.roles);
    const hasPermissions = Array.isArray(result.permissions);
    
    console.log("✓ Has roles array:", hasRoles);
    console.log("✓ Has permissions array:", hasPermissions);
    console.log("✓ Roles:", result.roles);
    console.log("✓ Permissions:", result.permissions);
    
    if (hasRoles && hasPermissions) {
      console.log("✅ Scenario 3 PASS");
      return { success: true };
    } else {
      console.log("❌ Scenario 3 FAIL");
      return { success: false };
    }
  } catch (err) {
    console.error("❌ Scenario 3 FAIL:", err.message);
    return { success: false, error: err.message };
  } finally {
    client.close();
  }
}

async function testScenario4() {
  console.log("\n=== Scenario 4: Client Credentials Token & Caching ===");
  
  const creds = new ClientCredentials({
    domain: DOMAIN,
    clientId: CLIENT_ID,
    clientSecret: CLIENT_SECRET,
  });

  try {
    // First call - should fetch new token
    const token1 = await creds.getToken();
    console.log("Token 1:", token1 ? `${token1.substring(0, 30)}...` : "null");
    
    const token1Valid = token1 && token1.split(".").length === 3;
    console.log("✓ First call returns JWT:", token1Valid);
    
    // Second call - should use cache
    const token2 = await creds.getToken();
    console.log("Token 2 (cached):", token2 ? `${token2.substring(0, 30)}...` : "null");
    
    const cachedPass = token1 === token2;
    console.log("✓ Second call uses cache:", cachedPass);
    
    // Clear cache - should fetch new token
    creds.clearCache();
    await sleep(100);
    const token3 = await creds.getToken();
    console.log("Token 3 (after clear):", token3 ? `${token3.substring(0, 30)}...` : "null");
    
    const newTokenPass = token1 !== token3;
    console.log("✓ After clearCache, new token:", newTokenPass);
    
    if (token1Valid && cachedPass && newTokenPass) {
      console.log("✅ Scenario 4 PASS");
      return { success: true };
    } else {
      console.log("❌ Scenario 4 FAIL");
      return { success: false };
    }
  } catch (err) {
    console.error("❌ Scenario 4 FAIL:", err.message);
    return { success: false, error: err.message };
  }
}

async function testScenario5() {
  console.log("\n=== Scenario 5: Client Credentials Error Handling ===");
  
  try {
    // Test wrong client_secret
    const badCreds = new ClientCredentials({
      domain: DOMAIN,
      clientId: CLIENT_ID,
      clientSecret: "wrong-secret", // pragma: allowlist secret
    });

    try {
      await badCreds.getToken();
      console.log("❌ Should have thrown on wrong secret");
      return { success: false };
    } catch (err) {
      console.log("✓ Wrong secret throws error:", err.message);
      const wrongSecretPass = err.statusCode === 401;
      console.log("✓ Status code is 401:", wrongSecretPass);
    }
    
    // Test non-existent client
    const noCreds = new ClientCredentials({
      domain: DOMAIN,
      clientId: "non-existent-client",
      clientSecret: "any", // pragma: allowlist secret
    });

    try {
      await noCreds.getToken();
      console.log("❌ Should have thrown on non-existent client");
      return { success: false };
    } catch (err) {
      console.log("✓ Non-existent client throws error:", err.message);
      const notFoundPass = err.statusCode === 401 || err.statusCode === 404;
      console.log("✓ Status code is 401/404:", notFoundPass);
    }
    
    // Test wrong domain
    const wrongDomain = new ClientCredentials({
      domain: "http://localhost:9999",
      clientId: "any",
      clientSecret: "any", // pragma: allowlist secret
    });

    try {
      await wrongDomain.getToken();
      console.log("❌ Should have thrown on wrong domain");
      return { success: false };
    } catch (err) {
      console.log("✓ Wrong domain throws error:", err.message);
    }
    
    console.log("✅ Scenario 5 PASS");
    return { success: true };
  } catch (err) {
    console.error("❌ Scenario 5 FAIL:", err.message);
    return { success: false, error: err.message };
  }
}

async function main() {
  console.log("Starting QA Integration Tests for SDK");
  console.log("=====================================");
  
  const results = [];
  
  // Scenario 1
  const s1 = await testScenario1();
  results.push(s1);
  
  // Scenario 2 needs access token from scenario 1
  if (s1.success && s1.accessToken) {
    const s2 = await testScenario2(s1.accessToken);
    results.push(s2);
  } else {
    console.log("\n⚠️ Skipping Scenario 2 (no valid access token)");
    results.push({ success: false, error: "Skipped due to Scenario 1 failure" });
  }
  
  // Scenario 3
  const s3 = await testScenario3();
  results.push(s3);
  
  // Scenario 4
  const s4 = await testScenario4();
  results.push(s4);
  
  // Scenario 5
  const s5 = await testScenario5();
  results.push(s5);
  
  // Summary
  console.log("\n=====================================");
  console.log("Test Summary");
  console.log("=====================================");
  
  const passed = results.filter(r => r.success).length;
  const total = results.length;
  
  console.log(`Passed: ${passed}/${total} (${Math.round(passed/total*100)}%)`);
  
  results.forEach((r, i) => {
    console.log(`Scenario ${i+1}: ${r.success ? "✅ PASS" : "❌ FAIL"}`);
    if (r.error) console.log(`  Error: ${r.error}`);
  });
  
  process.exit(passed === total ? 0 : 1);
}

main().catch(console.error);
