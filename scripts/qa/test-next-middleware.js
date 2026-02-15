import { auth9Middleware } from "./sdk/packages/node/dist/middleware/next.js";

// 模拟TokenVerifier
class MockTokenVerifier {
  constructor() {
    this.verify = async (token) => {
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
  }
}

// 替换TokenVerifier
import { TokenVerifier } from "./sdk/packages/node/dist/index.js";
const originalTokenVerifier = TokenVerifier;
global.TokenVerifier = MockTokenVerifier;

// 创建中间件
const middleware = auth9Middleware({
  domain: "http://localhost:8080",
  audience: "my-service",
  publicPaths: ["/", "/login", "/api/health"],
});

// 测试函数
async function testMiddleware(path, token) {
  const headers = {};
  if (token) {
    headers.authorization = `Bearer ${token}`;
  }
  
  const request = new Request(`http://localhost:3000${path}`, {
    headers
  });
  
  const response = await middleware(request);
  
  console.log(`\n测试路径: ${path}`);
  console.log(`Token: ${token ? "有" : "无"}`);
  console.log(`状态码: ${response.status}`);
  
  if (response.status === 401) {
    const body = await response.json();
    console.log(`错误信息: ${body.error} - ${body.message}`);
  } else if (response.status === 200) {
    console.log("Headers:");
    for (const [key, value] of response.headers.entries()) {
      if (key.startsWith('x-auth9-')) {
        console.log(`  ${key}: ${value}`);
      }
    }
  }
  
  return response;
}

// 执行测试
async function runTests() {
  console.log("=== Next.js Middleware 测试 ===");
  
  // 1. 测试公开路径（无Token）
  console.log("\n1. 测试公开路径（无Token）:");
  await testMiddleware("/", null);
  await testMiddleware("/login", null);
  await testMiddleware("/api/health", null);
  
  // 2. 测试保护路径（无Token）
  console.log("\n2. 测试保护路径（无Token）:");
  await testMiddleware("/api/users", null);
  
  // 3. 测试保护路径（有效Token）
  console.log("\n3. 测试保护路径（有效Token）:");
  await testMiddleware("/api/users", "valid_token");
  
  // 4. 测试保护路径（无效Token）
  console.log("\n4. 测试保护路径（无效Token）:");
  await testMiddleware("/api/users", "invalid_token");
  
  // 5. 测试子路径匹配
  console.log("\n5. 测试子路径匹配:");
  await testMiddleware("/api/health/deep", null);
  await testMiddleware("/api/users/123", null);
  
  console.log("\n=== 测试完成 ===");
}

// 恢复原始TokenVerifier
process.on('exit', () => {
  global.TokenVerifier = originalTokenVerifier;
});

runTests().catch(console.error);