// CJS导入测试
const { Auth9HttpClient, toSnakeCase } = require("./sdk/packages/core/dist/index.cjs");

console.log("=== CJS导入测试 (@auth9/core) ===");
console.log(`typeof Auth9HttpClient: ${typeof Auth9HttpClient} ${typeof Auth9HttpClient === "function" ? '✅' : '❌'}`);
console.log(`typeof toSnakeCase: ${typeof toSnakeCase} ${typeof toSnakeCase === "function" ? '✅' : '❌'}`);

// 测试函数调用
const snake = toSnakeCase("helloWorld");
console.log(`toSnakeCase("helloWorld"): ${snake} ${snake === "hello_world" ? '✅' : '❌'}`);

// 测试@auth9/node的CJS导入
try {
  const nodeModule = require("./sdk/packages/node/dist/index.cjs");
  console.log("\n=== CJS导入测试 (@auth9/node) ===");
  console.log(`typeof nodeModule.Auth9: ${typeof nodeModule.Auth9} ${typeof nodeModule.Auth9 === "function" ? '✅' : '❌'}`);
  console.log(`typeof nodeModule.TokenVerifier: ${typeof nodeModule.TokenVerifier} ${typeof nodeModule.TokenVerifier === "function" ? '✅' : '❌'}`);
} catch (error) {
  console.log(`\n❌ @auth9/node CJS导入失败: ${error.message}`);
}

// 测试中间件的CJS导入
try {
  const expressMiddleware = require("./sdk/packages/node/dist/middleware/express.cjs");
  console.log("\n=== CJS导入测试 (express middleware) ===");
  console.log(`typeof expressMiddleware.auth9Middleware: ${typeof expressMiddleware.auth9Middleware} ${typeof expressMiddleware.auth9Middleware === "function" ? '✅' : '❌'}`);
} catch (error) {
  console.log(`\n❌ express middleware CJS导入失败: ${error.message}`);
}

// 测试testing模块的CJS导入
try {
  const testing = require("./sdk/packages/node/dist/testing.cjs");
  console.log("\n=== CJS导入测试 (testing module) ===");
  console.log(`typeof testing.createMockToken: ${typeof testing.createMockToken} ${typeof testing.createMockToken === "function" ? '✅' : '❌'}`);
  console.log(`typeof testing.createMockAuth9: ${typeof testing.createMockAuth9} ${typeof testing.createMockAuth9 === "function" ? '✅' : '❌'}`);
} catch (error) {
  console.log(`\n❌ testing module CJS导入失败: ${error.message}`);
}

console.log("\n=== CJS导入测试完成 ===");