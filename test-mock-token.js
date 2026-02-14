// 测试createMockToken功能
import { createMockToken, createMockAuth9 } from "./sdk/packages/node/dist/testing.js";

async function testCreateMockToken() {
  console.log("=== 场景3：createMockToken测试 ===\n");
  
  // 1. 生成默认mock token
  console.log("1. 生成默认mock token:");
  const defaultToken = createMockToken();
  console.log(`   生成的token: ${defaultToken.substring(0, 50)}...`);
  
  // 验证token格式
  const parts = defaultToken.split(".");
  console.log(`   token段数: ${parts.length} (应为3)`);
  console.log(`   ${parts.length === 3 ? '✅' : '❌'} 三段式JWT格式`);
  
  // 解码payload检查默认值
  try {
    const payload = JSON.parse(Buffer.from(parts[1], "base64url").toString());
    console.log("\n   默认claims:");
    console.log(`     sub: ${payload.sub} (应为"test-user-id")`);
    console.log(`     email: ${payload.email} (应为"test@example.com")`);
    console.log(`     tenantId: ${payload.tenantId} (应为"test-tenant-id")`);
    console.log(`     roles: ${JSON.stringify(payload.roles)} (应为["user"])`);
    console.log(`     exp > iat: ${payload.exp > payload.iat ? '✅' : '❌'}`);
  } catch (error) {
    console.log(`   ❌ 解码失败: ${error.message}`);
  }
  
  // 2. 使用自定义claims生成token
  console.log("\n2. 使用自定义claims生成token:");
  const adminToken = createMockToken({
    sub: "admin-user-id",
    email: "admin@example.com",
    roles: ["admin", "user"],
    permissions: ["user:read", "user:write", "user:delete"],
    tenantId: "custom-tenant",
  });
  
  console.log(`   生成的admin token: ${adminToken.substring(0, 50)}...`);
  
  try {
    const adminParts = adminToken.split(".");
    const adminPayload = JSON.parse(Buffer.from(adminParts[1], "base64url").toString());
    
    console.log("\n   自定义claims验证:");
    console.log(`     sub: ${adminPayload.sub} ${adminPayload.sub === "admin-user-id" ? '✅' : '❌'}`);
    console.log(`     email: ${adminPayload.email} ${adminPayload.email === "admin@example.com" ? '✅' : '❌'}`);
    console.log(`     roles: ${JSON.stringify(adminPayload.roles)} ${JSON.stringify(adminPayload.roles) === JSON.stringify(["admin", "user"]) ? '✅' : '❌'}`);
    console.log(`     permissions: ${JSON.stringify(adminPayload.permissions)} ${JSON.stringify(adminPayload.permissions) === JSON.stringify(["user:read", "user:write", "user:delete"]) ? '✅' : '❌'}`);
    console.log(`     tenantId: ${adminPayload.tenantId} ${adminPayload.tenantId === "custom-tenant" ? '✅' : '❌'}`);
  } catch (error) {
    console.log(`   ❌ 解码失败: ${error.message}`);
  }
  
  // 3. 测试createMockAuth9
  console.log("\n3. 测试createMockAuth9:");
  const mockAuth9 = createMockAuth9({
    defaultUser: {
      sub: "test-user",
      email: "test@example.com",
      roles: ["admin"],
      permissions: ["user:read", "user:write"],
    },
  });
  
  // 测试verifyToken
  console.log("   a) verifyToken测试:");
  const customToken = createMockToken({ sub: "other-user", email: "other@test.com" });
  const claims = mockAuth9.verifyToken(customToken);
  console.log(`     解析claims.sub: ${claims.sub} ${claims.sub === "other-user" ? '✅' : '❌'}`);
  
  // 测试middleware无Token
  console.log("   b) middleware无Token测试:");
  const req1 = { headers: {}, auth: undefined };
  const res1 = {};
  let nextCalled1 = false;
  
  mockAuth9.middleware()(req1, res1, () => {
    nextCalled1 = true;
  });
  
  console.log(`     next调用: ${nextCalled1 ? '✅' : '❌'}`);
  console.log(`     req.auth.userId: ${req1.auth?.userId} ${req1.auth?.userId === "test-user" ? '✅' : '❌'}`);
  console.log(`     req.auth.roles: ${JSON.stringify(req1.auth?.roles)} ${JSON.stringify(req1.auth?.roles) === JSON.stringify(["admin"]) ? '✅' : '❌'}`);
  
  // 测试middleware有Token
  console.log("   c) middleware有Token测试:");
  const req2 = { headers: { authorization: `Bearer ${customToken}` }, auth: undefined };
  const res2 = {};
  let nextCalled2 = false;
  
  mockAuth9.middleware()(req2, res2, () => {
    nextCalled2 = true;
  });
  
  console.log(`     next调用: ${nextCalled2 ? '✅' : '❌'}`);
  console.log(`     req.auth.userId: ${req2.auth?.userId} ${req2.auth?.userId === "other-user" ? '✅' : '❌'}`);
  
  // 测试helper方法
  console.log("   d) helper方法测试:");
  if (req1.auth) {
    console.log(`     hasPermission("user:read"): ${req1.auth.hasPermission("user:read") ? '✅' : '❌'}`);
    console.log(`     hasPermission("user:delete"): ${req1.auth.hasPermission("user:delete") ? '❌' : '✅'} (应为false)`);
    console.log(`     hasRole("admin"): ${req1.auth.hasRole("admin") ? '✅' : '❌'}`);
    console.log(`     hasRole("user"): ${req1.auth.hasRole("user") ? '❌' : '✅'} (应为false)`);
  }
  
  console.log("\n=== 测试总结 ===");
  console.log("createMockToken和createMockAuth9功能测试完成。");
}

testCreateMockToken().catch(console.error);