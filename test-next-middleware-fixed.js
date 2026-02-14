// 直接测试构建后的中间件
import { auth9Middleware } from "./sdk/packages/node/dist/middleware/next.js";

// 模拟验证函数
const mockVerify = async (token) => {
  if (token === "valid_token") {
    return {
      claims: {
        sub: "746ceba8-3ddf-4a8b-b021-a1337b7a1a35",
        email: "admin@auth9.local",
        tenantId: "test-tenant-id",
        roles: ["admin", "user"],
        permissions: ["user:read", "user:write"]
      },
      tokenType: "tenantAccess"
    };
  }
  throw new Error("Invalid token");
};

// 创建中间件，但我们需要模拟TokenVerifier
// 由于构建后的代码已经编译，我们需要通过环境变量或其他方式模拟
// 这里我们直接测试逻辑

async function testScenario() {
  console.log("=== 场景1：Next.js Middleware测试 ===\n");
  
  // 测试1: 公开路径应该允许访问
  console.log("测试1: 公开路径应该允许访问（无Token）");
  const middleware1 = auth9Middleware({
    domain: "http://localhost:8080",
    audience: "my-service",
    publicPaths: ["/", "/login", "/api/health"],
  });
  
  const testCases = [
    { path: "/", token: null, expectedStatus: 200, description: "根路径" },
    { path: "/login", token: null, expectedStatus: 200, description: "登录页面" },
    { path: "/api/health", token: null, expectedStatus: 200, description: "健康检查" },
  ];
  
  for (const test of testCases) {
    const headers = {};
    if (test.token) {
      headers.authorization = `Bearer ${test.token}`;
    }
    
    const request = new Request(`http://localhost:3000${test.path}`, { headers });
    const response = await middleware1(request);
    
    console.log(`  ${test.description} (${test.path}): ${response.status === test.expectedStatus ? '✅' : '❌'} 状态码 ${response.status}`);
  }
  
  // 测试2: 保护路径需要Token
  console.log("\n测试2: 保护路径需要Token");
  const testCases2 = [
    { path: "/api/users", token: null, expectedStatus: 401, description: "无Token" },
    { path: "/api/users", token: "invalid_token", expectedStatus: 401, description: "无效Token" },
  ];
  
  for (const test of testCases2) {
    const headers = {};
    if (test.token) {
      headers.authorization = `Bearer ${test.token}`;
    }
    
    const request = new Request(`http://localhost:3000${test.path}`, { headers });
    const response = await middleware1(request);
    
    console.log(`  ${test.description} (${test.path}): ${response.status === test.expectedStatus ? '✅' : '❌'} 状态码 ${response.status}`);
    if (response.status === 401) {
      const body = await response.json();
      console.log(`    错误: ${body.error} - ${body.message}`);
    }
  }
  
  // 测试3: 使用protectedPaths配置
  console.log("\n测试3: 使用protectedPaths配置");
  const middleware2 = auth9Middleware({
    domain: "http://localhost:8080",
    protectedPaths: ["/api/users", "/api/admin"],
  });
  
  const testCases3 = [
    { path: "/public-page", token: null, expectedStatus: 200, description: "非保护路径" },
    { path: "/api/users", token: null, expectedStatus: 401, description: "保护路径无Token" },
  ];
  
  for (const test of testCases3) {
    const headers = {};
    if (test.token) {
      headers.authorization = `Bearer ${test.token}`;
    }
    
    const request = new Request(`http://localhost:3000${test.path}`, { headers });
    const response = await middleware2(request);
    
    console.log(`  ${test.description} (${test.path}): ${response.status === test.expectedStatus ? '✅' : '❌'} 状态码 ${response.status}`);
  }
  
  console.log("\n=== 测试总结 ===");
  console.log("Next.js Middleware基本功能测试完成。");
  console.log("注意：由于TokenVerifier需要真实验证，完整集成测试需要运行真实服务。");
}

testScenario().catch(console.error);