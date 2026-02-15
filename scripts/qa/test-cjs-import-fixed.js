// CJS导入测试
const { Auth9HttpClient, toSnakeCase } = require("./sdk/packages/core/dist/index.cjs");

console.log("=== CJS导入测试 (@auth9/core) ===");
console.log(`typeof Auth9HttpClient: ${typeof Auth9HttpClient} ${typeof Auth9HttpClient === "function" ? '✅' : '❌'}`);
console.log(`typeof toSnakeCase: ${typeof toSnakeCase} ${typeof toSnakeCase === "function" ? '✅' : '❌'}`);

// 测试函数调用 - toSnakeCase是用于转换对象键的
const testObj = { helloWorld: "value", anotherKey: 123 };
const snakeObj = toSnakeCase(testObj);
console.log(`toSnakeCase({helloWorld: "value", anotherKey: 123}):`);
console.log(`  结果: ${JSON.stringify(snakeObj)}`);
console.log(`  包含hello_world键: ${'hello_world' in snakeObj ? '✅' : '❌'}`);
console.log(`  包含another_key键: ${'another_key' in snakeObj ? '✅' : '❌'}`);

// 测试@auth9/node的CJS导入
try {
  const nodeModule = require("./sdk/packages/node/dist/index.cjs");
  console.log("\n=== CJS导入测试 (@auth9/node) ===");
  console.log(`typeof nodeModule.Auth9: ${typeof nodeModule.Auth9} ${typeof nodeModule.Auth9 === "function" ? '✅' : '❌'}`);
  console.log(`typeof nodeModule.TokenVerifier: ${typeof nodeModule.TokenVerifier} ${typeof nodeModule.TokenVerifier === "function" ? '✅' : '❌'}`);
  
  // 测试中间件导入
  const { auth9Middleware } = require("./sdk/packages/node/dist/middleware/express.cjs");
  console.log(`typeof auth9Middleware: ${typeof auth9Middleware} ${typeof auth9Middleware === "function" ? '✅' : '❌'}`);
  
  // 测试testing模块
  const { createMockToken } = require("./sdk/packages/node/dist/testing.cjs");
  console.log(`typeof createMockToken: ${typeof createMockToken} ${typeof createMockToken === "function" ? '✅' : '❌'}`);
  
  // 实际创建mock token
  const token = createMockToken();
  console.log(`createMockToken() 成功: ${token && token.length > 0 ? '✅' : '❌'}`);
  
} catch (error) {
  console.log(`\n❌ @auth9/node CJS导入失败: ${error.message}`);
}

console.log("\n=== CJS导入测试完成 ===");