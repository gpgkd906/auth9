// ESM导入测试
import { Auth9, TokenVerifier } from "./sdk/packages/node/dist/index.js";
import { auth9Middleware } from "./sdk/packages/node/dist/middleware/express.js";
import { createMockToken } from "./sdk/packages/node/dist/testing.js";
import { Auth9HttpClient, toSnakeCase } from "./sdk/packages/core/dist/index.js";

console.log("=== ESM导入测试 ===");

// 测试@auth9/core
console.log("\n1. @auth9/core导入测试:");
console.log(`   typeof Auth9HttpClient: ${typeof Auth9HttpClient} ${typeof Auth9HttpClient === "function" ? '✅' : '❌'}`);
console.log(`   typeof toSnakeCase: ${typeof toSnakeCase} ${typeof toSnakeCase === "function" ? '✅' : '❌'}`);

// 测试@auth9/node
console.log("\n2. @auth9/node导入测试:");
console.log(`   typeof Auth9: ${typeof Auth9} ${typeof Auth9 === "function" ? '✅' : '❌'}`);
console.log(`   typeof TokenVerifier: ${typeof TokenVerifier} ${typeof TokenVerifier === "function" ? '✅' : '❌'}`);

// 测试中间件
console.log("\n3. 中间件导入测试:");
console.log(`   typeof auth9Middleware: ${typeof auth9Middleware} ${typeof auth9Middleware === "function" ? '✅' : '❌'}`);

// 测试testing模块
console.log("\n4. testing模块导入测试:");
console.log(`   typeof createMockToken: ${typeof createMockToken} ${typeof createMockToken === "function" ? '✅' : '❌'}`);

// 实际创建mock token
const token = createMockToken();
console.log(`   createMockToken() 成功: ${token && token.length > 0 ? '✅' : '❌'}`);

// 测试toSnakeCase功能
const testObj = { helloWorld: "value", anotherKey: 123 };
const snakeObj = toSnakeCase(testObj);
console.log(`   toSnakeCase转换: ${'hello_world' in snakeObj && 'another_key' in snakeObj ? '✅' : '❌'}`);

console.log("\n=== ESM导入测试完成 ===");